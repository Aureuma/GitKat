from __future__ import annotations

import argparse
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPOS = [
    "https://github.com/octocat/Hello-World.git",
    "https://github.com/githubtraining/hellogitworld.git",
    "https://github.com/github/gitignore.git",
]

COMMIT_CALLBACK = """
import os
import re

new_name = os.environ.get("GITKIT_NEW_NAME", "").encode()
new_email = os.environ.get("GITKIT_NEW_EMAIL", "").encode()
old_name_raw = os.environ.get("GITKIT_OLD_NAME", "")
old_name = old_name_raw.lower() if old_name_raw else None
old_emails = {e.lower() for e in os.environ.get("GITKIT_OLD_EMAILS", "").splitlines() if e}
identity_enabled = bool(new_email and old_emails)

def lower_bytes(val):
    try:
        return val.decode().lower()
    except Exception:
        return val.lower()

def rewrite_identity(commit):
    changed = False
    if not identity_enabled:
        return changed

    a_email = lower_bytes(commit.author_email)
    a_name = lower_bytes(commit.author_name)
    if a_email in old_emails and (not old_name or a_name == old_name):
        if new_name:
            commit.author_name = new_name
        commit.author_email = new_email
        changed = True

    c_email = lower_bytes(commit.committer_email)
    c_name = lower_bytes(commit.committer_name)
    if c_email in old_emails and (not old_name or c_name == old_name):
        if new_name:
            commit.committer_name = new_name
        commit.committer_email = new_email
        changed = True

    if changed:
        msg = commit.message
        msg = re.sub(rb"(?im)^\\s*(signed-off-by|co-authored-by|reviewed-by|acked-by|tested-by|reported-by|suggested-by):.*\\n?", b"", msg)
        commit.message = msg
    return changed

rewrite_identity(commit)
""".lstrip("\n")

FILE_INFO_CALLBACK = """
import fnmatch
import os
import re

raw_pairs = [line for line in os.environ.get("GITKIT_BLOB_MAP", "").splitlines() if line]
exclude_raw = [line for line in os.environ.get("GITKIT_EXCLUDE_PATTERNS", "").splitlines() if line]
rename_files = os.environ.get("GITKIT_RENAME_FILES", "0") == "1"
ignore_case = os.environ.get("GITKIT_IGNORE_CASE", "0") == "1"
preserve_case_enabled = os.environ.get("GITKIT_PRESERVE_CASE", "0") == "1"
if not raw_pairs:
    return (filename, mode, blob_id)

path_bytes = filename or b""
path_str = path_bytes.decode("utf-8", "ignore") or "<unknown path>"

state = value.data.setdefault("gitkat_blob_state", {})
exclude_patterns = state.get("exclude_patterns")
if exclude_patterns is None:
    state["exclude_patterns"] = exclude_raw
    exclude_patterns = state["exclude_patterns"]

if exclude_patterns:
    for pat in exclude_patterns:
        if fnmatch.fnmatchcase(path_str, pat):
            return (filename, mode, blob_id)

patterns = state.get("patterns")
if patterns is None:
    pairs = []
    for line in raw_pairs:
        if "\t" not in line:
            continue
        old, new = line.split("\t", 1)
        pairs.append((old.encode(), new.encode()))

    if not pairs:
        state["patterns"] = []
    else:
        re_flags = re.IGNORECASE if ignore_case else 0
        state["patterns"] = [(re.compile(re.escape(old), re_flags), new) for old, new in pairs]
    patterns = state["patterns"]

if not patterns:
    return (filename, mode, blob_id)

def preserve_case(match, replacement):
    src = match.group(0)
    if not replacement:
        return replacement
    if src.isupper():
        return replacement.upper()
    if src.islower():
        return replacement.lower()
    if src[:1].isupper() and src[1:].islower():
        return replacement[:1].upper() + replacement[1:].lower()
    out = bytearray()
    for i, b in enumerate(replacement):
        if i < len(src):
            sb = chr(src[i])
            rb = chr(b)
            if sb.isupper():
                out.append(ord(rb.upper()))
            elif sb.islower():
                out.append(ord(rb.lower()))
            else:
                out.append(b)
        else:
            out.append(b)
    return bytes(out)

if rename_files and filename:
    new_filename = filename
    for pattern, replacement in patterns:
        if preserve_case_enabled:
            def repl(m, replacement=replacement):
                return preserve_case(m, replacement)
            new_filename, _n = pattern.subn(repl, new_filename)
        else:
            new_filename, _n = pattern.subn(replacement, new_filename)
    filename = new_filename

contents = value.get_contents_by_identifier(blob_id)
if value.is_binary(contents):
    return (filename, mode, blob_id)

data = contents
changed = False
for pattern, replacement in patterns:
    matches = list(pattern.finditer(data))
    if not matches:
        continue

    changed = True
    snapshot = data
    new_data = bytearray()
    last = 0
    for m in matches:
        repl_bytes = preserve_case(m, replacement) if preserve_case_enabled else replacement
        new_data.extend(snapshot[last:m.start()])
        new_data.extend(repl_bytes)
        last = m.end()
    new_data.extend(snapshot[last:])
    data = bytes(new_data)

if not changed:
    return (filename, mode, blob_id)

new_blob_id = value.insert_file_with_contents(data)
return (filename, mode, new_blob_id)
""".lstrip("\n")


def run(cmd: list[str], cwd: Path | None = None, env: dict[str, str] | None = None) -> None:
    subprocess.run(cmd, cwd=cwd, check=True, text=True, env=env)


def git_output(cmd: list[str], cwd: Path) -> str:
    return subprocess.check_output(cmd, cwd=cwd, text=True)


def fast_export_hash(repo: Path) -> str:
    proc = subprocess.Popen(["git", "fast-export", "--all"], cwd=repo, stdout=subprocess.PIPE)
    assert proc.stdout is not None
    digest = hashlib.sha256()
    while True:
        chunk = proc.stdout.read(1024 * 1024)
        if not chunk:
            break
        digest.update(chunk)
    proc.wait()
    if proc.returncode != 0:
        raise subprocess.CalledProcessError(proc.returncode, ["git", "fast-export", "--all"])
    return digest.hexdigest()


def pick_identity(repo: Path) -> tuple[str, str, str]:
    line = git_output(["git", "log", "-n", "1", "--format=%an%x00%ae"], repo).strip()
    if "\x00" not in line:
        raise RuntimeError("Unable to read author identity")
    name, email = line.split("\x00", 1)
    return name, email, name


def pick_blob_map(repo: Path) -> list[str]:
    for candidate in ("README.md", "README", "readme.md"):
        try:
            content = subprocess.check_output(
                ["git", "show", f"HEAD:{candidate}"],
                cwd=repo,
                stderr=subprocess.DEVNULL,
                text=True,
            )
        except subprocess.CalledProcessError:
            continue
        first_line = content.splitlines()[0] if content else ""
        tokens = re.findall(r"[A-Za-z][A-Za-z0-9_-]{5,}", first_line)
        if tokens:
            token = tokens[0]
            return [f"{token}:{token}_REDACTED"]
    return []


def run_gitkat(repo: Path, options: dict[str, list[str] | str | bool], repo_root: Path) -> None:
    env = os.environ.copy()
    env["PYTHONPATH"] = str(repo_root / "src") + os.pathsep + env.get("PYTHONPATH", "")
    env.setdefault("GITKAT_REWRITE_PROFILE", "release")
    cmd = [sys.executable, "-m", "gitkat.cli", "rewrite"]
    cmd.extend(["-n", options["new_name"], "-e", options["new_email"]])
    cmd.extend(["-o", options["old_email"]])
    if options.get("old_name"):
        cmd.extend(["-O", options["old_name"]])
    for mapping in options.get("blob_map", []):
        cmd.extend(["-m", mapping])
    for pattern in options.get("exclude", []):
        cmd.extend(["-x", pattern])
    if options.get("ignore_case"):
        cmd.append("--ignore-case")
    if options.get("preserve_case"):
        cmd.append("--preserve-case")
    if options.get("rename_files"):
        cmd.append("--rename-files")
    run(cmd, cwd=repo, env=env)


def _serialize_lines(values: list[str]) -> str:
    return "\n".join(values)


def _build_blob_map_env(entries: list[str]) -> str:
    lines = []
    for entry in entries:
        old, new = entry.split(":", 1)
        lines.append(f"{old}\t{new}")
    return "\n".join(lines)


def run_filter_repo(repo: Path, options: dict[str, list[str] | str | bool]) -> None:
    env = os.environ.copy()
    env.update(
        {
            "GITKIT_NEW_NAME": options["new_name"],
            "GITKIT_NEW_EMAIL": options["new_email"],
            "GITKIT_OLD_NAME": options.get("old_name", ""),
            "GITKIT_OLD_EMAILS": _serialize_lines([options["old_email"]]),
            "GITKIT_BLOB_MAP": _build_blob_map_env(options.get("blob_map", [])),
            "GITKIT_EXCLUDE_PATTERNS": _serialize_lines(options.get("exclude", [])),
            "GITKIT_PRESERVE_CASE": "1" if options.get("preserve_case") else "0",
            "GITKIT_IGNORE_CASE": "1" if options.get("ignore_case") else "0",
            "GITKIT_RENAME_FILES": "1" if options.get("rename_files") else "0",
        }
    )

    with tempfile.TemporaryDirectory() as tmpdir:
        commit_path = Path(tmpdir) / "commit_callback.py"
        file_info_path = Path(tmpdir) / "file_info_callback.py"
        commit_path.write_text(COMMIT_CALLBACK)
        file_info_path.write_text(FILE_INFO_CALLBACK)
        run(
            [
                "git",
                "filter-repo",
                "--force",
                "--commit-callback",
                str(commit_path),
                "--file-info-callback",
                str(file_info_path),
            ],
            cwd=repo,
            env=env,
        )


def clone_repo(url: str, dest: Path) -> None:
    run(["git", "clone", "--quiet", url, str(dest)])


def clone_local(source: Path, dest: Path) -> None:
    run(["git", "clone", "--quiet", "--no-hardlinks", str(source), str(dest)])


def verify_repo(url: str, workdir: Path, repo_root: Path, with_blob: bool) -> None:
    name = url.rsplit("/", 1)[-1].replace(".git", "")
    source = workdir / name
    clone_repo(url, source)

    _old_author_name, old_email, old_name = pick_identity(source)
    blob_map = pick_blob_map(source) if with_blob else []
    options = {
        "new_name": "GitKat Rewrite",
        "new_email": "rewrite@example.test",
        "old_email": old_email,
        "old_name": old_name,
        "blob_map": blob_map,
        "exclude": [],
        "ignore_case": False,
        "preserve_case": False,
        "rename_files": False,
    }

    gix_repo = workdir / f"{name}-gix"
    filter_repo = workdir / f"{name}-filter"
    clone_local(source, gix_repo)
    clone_local(source, filter_repo)

    run_gitkat(gix_repo, options, repo_root)
    run_filter_repo(filter_repo, options)

    gix_hash = fast_export_hash(gix_repo)
    filter_hash = fast_export_hash(filter_repo)
    if gix_hash != filter_hash:
        raise RuntimeError(f"Mismatch for {url}: {gix_hash} != {filter_hash}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workdir", help="Work directory for clones")
    parser.add_argument("--keep-workdir", action="store_true", help="Keep the workdir after completion")
    parser.add_argument("--with-blob", action="store_true", help="Attempt a small blob rewrite based on README")
    parser.add_argument("repos", nargs="*", help="Repo URLs to verify")
    args = parser.parse_args()

    if shutil.which("git-filter-repo") is None:
        raise SystemExit("git-filter-repo is required for verification")

    repo_root = Path(__file__).resolve().parents[1]
    repos = args.repos or REPOS
    if not repos:
        raise SystemExit("No repositories provided")

    if args.workdir:
        workdir = Path(args.workdir)
        workdir.mkdir(parents=True, exist_ok=True)
        cleanup = False
    else:
        workdir = Path(tempfile.mkdtemp(prefix="gitkat-verify-"))
        cleanup = True

    try:
        for url in repos:
            print(f"Verifying {url}...")
            verify_repo(url, workdir, repo_root, args.with_blob)
            print(f"OK: {url}")
    finally:
        if cleanup and not args.keep_workdir:
            shutil.rmtree(workdir, ignore_errors=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
