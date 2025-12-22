"""Rewrite git history across repositories."""

from __future__ import annotations

import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

from ..git import find_git_root, git_output, git_output_or_empty, list_child_repos, run_git


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


def _parse_blob_map(entries: Iterable[str]) -> list[str]:
    parsed: list[str] = []
    for entry in entries:
        if ":" not in entry:
            raise ValueError(f"Invalid -m entry '{entry}'. Expected old:new.")
        parsed.append(entry)
    return parsed


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


def _resolve_gix_rewrite_binary() -> Path:
    override = os.environ.get("GITKAT_REWRITE_BIN")
    if override:
        return Path(override)

    manifest_path = None
    for parent in Path(__file__).resolve().parents:
        candidate = parent / "crates" / "gitkat-rewrite" / "Cargo.toml"
        if candidate.exists():
            manifest_path = candidate
            break
    if not manifest_path:
        raise FileNotFoundError("Could not locate crates/gitkat-rewrite/Cargo.toml")

    profile = os.environ.get("GITKAT_REWRITE_PROFILE", "release")
    binary_name = "gitkat-rewrite.exe" if os.name == "nt" else "gitkat-rewrite"
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", manifest_path.parent / "target"))
    binary_path = target_dir / profile / binary_name
    if binary_path.exists():
        binary_mtime = binary_path.stat().st_mtime
        source_paths = [manifest_path, *manifest_path.parent.rglob("*.rs")]
        latest_source = max(path.stat().st_mtime for path in source_paths)
        if latest_source <= binary_mtime:
            return binary_path

    command = [
        "cargo",
        "build",
        "--manifest-path",
        str(manifest_path),
    ]
    if profile == "release":
        command.append("--release")
    subprocess.run(command, check=True, text=True)
    if not binary_path.exists():
        raise FileNotFoundError(f"Expected rewrite binary at {binary_path}")
    return binary_path


def _run_gix_rewrite(repo: Path, options: RewriteOptions, runner=subprocess.run) -> None:
    binary = _resolve_gix_rewrite_binary()
    command = [
        str(binary),
        "--repo",
        str(repo),
    ]
    if options.new_name:
        command.extend(["--new-name", options.new_name])
    if options.new_email:
        command.extend(["--new-email", options.new_email])
    if options.old_name:
        command.extend(["--old-name", options.old_name])
    for email in options.old_emails or []:
        command.extend(["--old-email", email])
    for mapping in options.blob_map or []:
        command.extend(["--map", mapping])
    for pattern in options.exclude_patterns or []:
        command.extend(["--exclude", pattern])
    if options.preserve_case:
        command.append("--preserve-case")
    if options.ignore_case:
        command.append("--ignore-case")
    if options.rename_files:
        command.append("--rename-files")
    runner(command, cwd=repo, check=True, text=True)


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

    for repo, display in repos:
        print()
        print("========================================")
        print(f" Repo: {display}")
        print("========================================")
        remotes = _capture_remotes(repo)
        _run_gix_rewrite(repo, opts)
        _restore_remotes(repo, remotes)
        print()
        print(f"---- Summary for {display} ----")
        _print_summary(repo, opts)

    print()
    print("✅ Rewrite complete (identity metadata + blob data).")
    print("Verify logs, then push rewritten histories with:")
    print("  git push --force --tags origin main")
    return 0
