import os
import shutil
import subprocess
import tempfile
from pathlib import Path

import pytest

from gitkat.commands import rewrite
from tests.helpers import commit_file

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


def _serialize_lines(values):
    return "\n".join(values)


def _build_blob_map_env(entries):
    lines = []
    for entry in entries:
        old, new = entry.split(":", 1)
        lines.append(f"{old}\t{new}")
    return "\n".join(lines)


def _run_filter_repo(repo: Path, options: rewrite.RewriteOptions) -> None:
    env = os.environ.copy()
    env.update(
        {
            "GITKIT_NEW_NAME": options.new_name,
            "GITKIT_NEW_EMAIL": options.new_email,
            "GITKIT_OLD_NAME": options.old_name,
            "GITKIT_OLD_EMAILS": _serialize_lines(options.old_emails or []),
            "GITKIT_BLOB_MAP": _build_blob_map_env(options.blob_map or []),
            "GITKIT_EXCLUDE_PATTERNS": _serialize_lines(options.exclude_patterns or []),
            "GITKIT_PRESERVE_CASE": "1" if options.preserve_case else "0",
            "GITKIT_IGNORE_CASE": "1" if options.ignore_case else "0",
            "GITKIT_RENAME_FILES": "1" if options.rename_files else "0",
        }
    )

    with tempfile.TemporaryDirectory() as tmpdir:
        commit_path = Path(tmpdir) / "commit_callback.py"
        file_info_path = Path(tmpdir) / "file_info_callback.py"
        commit_path.write_text(COMMIT_CALLBACK)
        file_info_path.write_text(FILE_INFO_CALLBACK)
        subprocess.run(
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
            check=True,
            text=True,
        )


def _fast_export(repo: Path) -> str:
    return subprocess.check_output(["git", "fast-export", "--all"], cwd=repo, text=True)


def _init_repo(tmp_path: Path, name: str) -> Path:
    repo = tmp_path / name
    repo.mkdir(parents=True, exist_ok=True)
    subprocess.run(["git", "init"], cwd=repo, check=True, capture_output=True)
    return repo


def _add_binary(repo: Path, filename: str) -> None:
    path = repo / filename
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"\x00binary\x00")
    subprocess.run(["git", "add", filename], cwd=repo, check=True)


def _clone_repo(source: Path, dest: Path) -> None:
    subprocess.run(["git", "clone", str(source), str(dest)], check=True, capture_output=True)


@pytest.mark.skipif(shutil.which("git-filter-repo") is None, reason="git-filter-repo not installed")
@pytest.mark.skipif(shutil.which("cargo") is None, reason="cargo not installed")
def test_rewrite_matches_filter_repo(tmp_path: Path, monkeypatch):
    source = _init_repo(tmp_path, "source")
    commit_file(
        source,
        "docs/OldName.txt",
        "Token ABC and oldname in content",
        author_name="Old Name",
        author_email="old@example.test",
        message="initial\n\nSigned-off-by: Old Name <old@example.test>",
    )
    commit_file(
        source,
        "data/skip.csv",
        "token,oldname",
        author_name="Old Name",
        author_email="old@example.test",
        message="add csv",
    )
    _add_binary(source, "bin/blob.bin")
    env = os.environ.copy()
    env.update(
        {
            "GIT_AUTHOR_NAME": "Old Name",
            "GIT_AUTHOR_EMAIL": "old@example.test",
            "GIT_COMMITTER_NAME": "Old Name",
            "GIT_COMMITTER_EMAIL": "old@example.test",
        }
    )
    subprocess.run(["git", "commit", "-m", "add binary"], cwd=source, check=True, env=env)

    commit_file(
        source,
        "src/other.txt",
        "Another token here",
        author_name="Other Name",
        author_email="other@example.test",
        message="second",
    )
    subprocess.run(["git", "tag", "-a", "v1", "-m", "release"], cwd=source, check=True)

    gix_repo = tmp_path / "gix"
    filter_repo = tmp_path / "filter"
    _clone_repo(source, gix_repo)
    _clone_repo(source, filter_repo)

    options = rewrite.RewriteOptions(
        new_name="New Name",
        new_email="new@example.test",
        old_name="Old Name",
        old_emails=["old@example.test"],
        blob_map=["token:REDACTED", "OldName:NewName"],
        exclude_patterns=["data/*.csv"],
        preserve_case=True,
        ignore_case=True,
        rename_files=True,
    )

    monkeypatch.setenv("GITKAT_REWRITE_PROFILE", "debug")
    rewrite._run_gix_rewrite(gix_repo, options)
    _run_filter_repo(filter_repo, options)

    assert _fast_export(gix_repo) == _fast_export(filter_repo)
