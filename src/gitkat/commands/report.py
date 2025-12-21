"""Report unique author emails for repositories."""

from __future__ import annotations

from pathlib import Path

from ..git import git_output_or_empty, list_repos_recursive


def run(base_dir: Path | None = None) -> int:
    base = base_dir or Path.cwd()
    repos = list_repos_recursive(base)
    if not repos:
        print(f"No git repositories found under {base}.")
        return 1

    for repo in repos:
        print()
        print(f"Repo: {repo}")
        output = git_output_or_empty(["log", "--format=%ae"], cwd=repo)
        emails = sorted({line.strip() for line in output.splitlines() if line.strip()})
        if emails:
            for email in emails:
                print(email)
        else:
            print("  (no commits)")
    return 0
