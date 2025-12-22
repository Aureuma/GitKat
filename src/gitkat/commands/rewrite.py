"""Rewrite git history across repositories."""

from __future__ import annotations

import os
import subprocess
import tempfile
import textwrap
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

from ..git import find_git_root, git_output, git_output_or_empty, list_child_repos, run_git

COMMIT_CALLBACK = textwrap.dedent(
    """
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
            old_a_name = commit.author_name.decode(errors="ignore")
            old_a_email = commit.author_email.decode(errors="ignore")
            if new_name:
                commit.author_name = new_name
            commit.author_email = new_email
            print(f"[Author] {old_a_name} <{old_a_email}>  →  {(new_name or commit.author_name).decode(errors='ignore')} <{new_email.decode(errors='ignore')}>")
            changed = True

        c_email = lower_bytes(commit.committer_email)
        c_name = lower_bytes(commit.committer_name)
        if c_email in old_emails and (not old_name or c_name == old_name):
            old_c_name = commit.committer_name.decode(errors="ignore")
            old_c_email = commit.committer_email.decode(errors="ignore")
            if new_name:
                commit.committer_name = new_name
            commit.committer_email = new_email
            print(f"[Committer] {old_c_name} <{old_c_email}>  →  {(new_name or commit.committer_name).decode(errors='ignore')} <{new_email.decode(errors='ignore')}>")
            changed = True

        if changed:
            msg = commit.message
            msg = re.sub(rb"(?im)^\\s*(signed-off-by|co-authored-by|reviewed-by|acked-by|tested-by|reported-by|suggested-by):.*\\n?", b"", msg)
            if msg != commit.message:
                print("[Message Cleanup] Removed DCO trace lines")
            commit.message = msg
        return changed

    rewrite_identity(commit)
    """
).lstrip("\n")

FILE_INFO_CALLBACK = textwrap.dedent(
    """
    import fnmatch
    import os
    import re

    raw_pairs = [line for line in os.environ.get("GITKIT_BLOB_MAP", "").splitlines() if line]
    exclude_raw = [line for line in os.environ.get("GITKIT_EXCLUDE_PATTERNS", "").splitlines() if line]
    rename_files = os.environ.get("GITKIT_RENAME_FILES", "0") == "1"
    ignore_case = os.environ.get("GITKIT_IGNORE_CASE", "0") == "1"
    preserve_case_enabled = os.environ.get("GITKIT_PRESERVE_CASE", "0") == "1"
    CTX_WORDS = 2
    COLOR_PATH = "\033[95m"   # magenta for file paths
    COLOR_MATCH = "\033[31m"  # red for matches
    COLOR_REPL = "\033[34m"  # blue for replacements
    COLOR_RESET = "\033[0m"
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
        # Mirror the matched casing pattern onto the replacement.
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

    rename_changed = False
    if rename_files and filename:
        new_filename = filename
        for pattern, replacement in patterns:
            if preserve_case_enabled:
                def repl(m, replacement=replacement):
                    return preserve_case(m, replacement)
                new_filename, n = pattern.subn(repl, new_filename)
            else:
                new_filename, n = pattern.subn(replacement, new_filename)
            if n:
                rename_changed = True
        if rename_changed:
            new_path_str = new_filename.decode("utf-8", "ignore") or "<unknown path>"
            print(f"{COLOR_PATH}{path_str}{COLOR_RESET} -> {COLOR_PATH}{new_path_str}{COLOR_RESET}")
            filename = new_filename
            path_str = new_path_str

    contents = value.get_contents_by_identifier(blob_id)
    if value.is_binary(contents):
        return (filename, mode, blob_id)

    data = contents

    def decode_snippet(b):
        return b.decode("utf-8", "replace")

    def extract_context(line_text, match_text, repl_text, match_pos):
        prefix = line_text[:match_pos]
        suffix = line_text[match_pos + len(match_text):]
        pre_words = prefix.strip().split()
        post_words = suffix.strip().split()
        left = " ".join(pre_words[-CTX_WORDS:])
        right = " ".join(post_words[:CTX_WORDS])
        left = (left + " ").strip() if left else ""
        right = (" " + right).strip() if right else ""
        left_line = f"{left}{COLOR_MATCH}{match_text}{COLOR_RESET}{right}".strip()
        right_line = f"{left}{COLOR_REPL}{repl_text}{COLOR_RESET}{right}".strip()
        return left_line, right_line

    printed_path = False
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
            if not printed_path:
                print(f"{COLOR_PATH}{path_str}{COLOR_RESET}")
                printed_path = True
            line_start = snapshot.rfind(b"\\n", 0, m.start()) + 1
            line_end = snapshot.find(b"\\n", m.end())
            if line_end == -1:
                line_end = len(snapshot)
            line_bytes = snapshot[line_start:line_end]
            line_text = decode_snippet(line_bytes)
            match_text = decode_snippet(m.group(0))
            repl_text = decode_snippet(repl_bytes)
            rel_match_pos = m.start() - line_start
            left_line, right_line = extract_context(line_text, match_text, repl_text, rel_match_pos)
            print(f"{COLOR_PATH}{path_str}{COLOR_RESET} {left_line} -> {right_line}")
            last = m.end()
        new_data.extend(snapshot[last:])
        data = bytes(new_data)

    if not changed:
        return (filename, mode, blob_id)

    new_blob_id = value.insert_file_with_contents(data)
    return (filename, mode, new_blob_id)
    """
).lstrip("\n")


@dataclass
class RewriteOptions:
    new_name: str = ""
    new_email: str = ""
    old_name: str = ""
    old_emails: list[str] | None = None
    blob_map: list[str] | None = None
    exclude_patterns: list[str] | None = None
    preserve_case: bool = False
    ignore_case: bool = False
    rename_files: bool = False


@dataclass
class RemoteConfig:
    name: str
    fetch_urls: list[str]
    push_urls: list[str]


def _split_comma_args(values: Iterable[str] | None) -> list[str]:
    if not values:
        return []
    items: list[str] = []
    for value in values:
        for entry in value.split(","):
            entry = entry.strip()
            if entry:
                items.append(entry)
    return items


def _serialize_lines(values: Iterable[str]) -> str:
    return "\n".join(values)


def _parse_blob_map(entries: Iterable[str]) -> list[str]:
    parsed: list[str] = []
    for entry in entries:
        if ":" not in entry:
            raise ValueError(f"Invalid -m entry '{entry}'. Expected old:new.")
        parsed.append(entry)
    return parsed


def _build_blob_map_env(entries: Iterable[str]) -> str:
    lines = []
    for entry in entries:
        old, new = entry.split(":", 1)
        lines.append(f"{old}\t{new}")
    return "\n".join(lines)


def _resolve_repos(base_dir: Path) -> list[tuple[Path, str]]:
    repos = list_child_repos(base_dir)
    if repos:
        return [(repo, repo.name) for repo in repos]

    git_root = find_git_root(base_dir)
    if git_root:
        return [(git_root, str(git_root))]

    return []


def _capture_remotes(repo: Path) -> list[RemoteConfig]:
    names = [line.strip() for line in git_output_or_empty(["remote"], cwd=repo).splitlines() if line.strip()]
    remotes: list[RemoteConfig] = []
    for name in names:
        fetch_urls = [
            line.strip()
            for line in git_output_or_empty(["config", "--get-all", f"remote.{name}.url"], cwd=repo).splitlines()
            if line.strip()
        ]
        push_urls = [
            line.strip()
            for line in git_output_or_empty(["config", "--get-all", f"remote.{name}.pushurl"], cwd=repo).splitlines()
            if line.strip()
        ]
        remotes.append(RemoteConfig(name=name, fetch_urls=fetch_urls, push_urls=push_urls))
    return remotes


def _restore_remotes(repo: Path, remotes: list[RemoteConfig]) -> None:
    for remote in remotes:
        if remote.fetch_urls:
            first = remote.fetch_urls[0]
            result = run_git(["remote", "add", remote.name, first], cwd=repo, check=False)
            if result.returncode != 0:
                run_git(["remote", "set-url", remote.name, first], cwd=repo, check=False)
            for extra in remote.fetch_urls[1:]:
                run_git(["remote", "set-url", "--add", remote.name, extra], cwd=repo, check=False)
        if remote.push_urls:
            for url in remote.push_urls:
                run_git(["remote", "set-url", "--add", "--push", remote.name, url], cwd=repo, check=False)


def _run_filter_repo(repo: Path, env: dict[str, str], runner=subprocess.run) -> None:
    # Write callbacks to temp files to avoid quoting issues in subprocess.
    with tempfile.TemporaryDirectory() as tmpdir:
        commit_path = Path(tmpdir) / "commit_callback.py"
        file_info_path = Path(tmpdir) / "file_info_callback.py"
        commit_path.write_text(COMMIT_CALLBACK)
        file_info_path.write_text(FILE_INFO_CALLBACK)
        runner(
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


def _count_matching_emails(repo: Path, email: str) -> int:
    if not email:
        return 0
    output = git_output(["log", "--all", "--format=%ae"], cwd=repo)
    needle = email.lower()
    return sum(1 for line in output.splitlines() if needle in line.lower())


def _print_summary(repo: Path, opts: RewriteOptions) -> None:
    total = git_output(["rev-list", "--all", "--count"], cwd=repo).strip()
    print(f"Total commits:               {total}")
    if opts.new_email:
        replaced = _count_matching_emails(repo, opts.new_email)
        print(f"Commits now using new email: {replaced}")
    else:
        print("Commits now using new email: (identity rewrite skipped)")
    blob_count = len(opts.blob_map or [])
    print(f"Blob mappings applied:       {blob_count}")
    print("Remote(s):")
    remotes = git_output_or_empty(["remote", "-v"], cwd=repo)
    if remotes.strip():
        print(remotes.rstrip())
    else:
        print("  (none)")
    print("----------------------------------------")


def run(options: RewriteOptions, base_dir: Path | None = None) -> int:
    try:
        blob_map = _parse_blob_map(options.blob_map or [])
    except ValueError as exc:
        print(str(exc))
        return 1

    opts = RewriteOptions(
        new_name=options.new_name or "",
        new_email=options.new_email or "",
        old_name=options.old_name or "",
        old_emails=_split_comma_args(options.old_emails),
        blob_map=blob_map,
        exclude_patterns=_split_comma_args(options.exclude_patterns),
        preserve_case=options.preserve_case,
        ignore_case=options.ignore_case,
        rename_files=options.rename_files,
    )

    if not opts.old_emails and not opts.blob_map:
        print("Error: specify at least one identity rewrite (-o/-e) or blob data mapping (-m).")
        return 1

    if opts.old_emails and not opts.new_email:
        print("Error: identity rewrites require -e <new_email> along with -o <old_emails>.")
        return 1

    if opts.new_email and not opts.old_emails:
        print("Error: -e was provided without any -o entries to match.")
        return 1

    base = base_dir or Path.cwd()
    repos = _resolve_repos(base)
    if not repos:
        print(f"Error: no git repositories found under {base}. Run from a parent directory containing repos or from inside a repo.")
        return 1

    env = os.environ.copy()
    env.update(
        {
            "GITKIT_NEW_NAME": opts.new_name,
            "GITKIT_NEW_EMAIL": opts.new_email,
            "GITKIT_OLD_NAME": opts.old_name,
            "GITKIT_OLD_EMAILS": _serialize_lines(opts.old_emails),
            "GITKIT_BLOB_MAP": _build_blob_map_env(opts.blob_map),
            "GITKIT_EXCLUDE_PATTERNS": _serialize_lines(opts.exclude_patterns),
            "GITKIT_PRESERVE_CASE": "1" if opts.preserve_case else "0",
            "GITKIT_IGNORE_CASE": "1" if opts.ignore_case else "0",
            "GITKIT_RENAME_FILES": "1" if opts.rename_files else "0",
        }
    )

    for repo, display in repos:
        print()
        print("========================================")
        print(f" Repo: {display}")
        print("========================================")
        remotes = _capture_remotes(repo)
        _run_filter_repo(repo, env)
        _restore_remotes(repo, remotes)
        print()
        print(f"---- Summary for {display} ----")
        _print_summary(repo, opts)

    print()
    print("✅ Rewrite complete (identity metadata + blob data).")
    print("Verify logs, then push rewritten histories with:")
    print("  git push --force --tags origin main")
    return 0
