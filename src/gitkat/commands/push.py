"""Force-push the current branch for each repository."""

from __future__ import annotations

from pathlib import Path
from typing import Callable

from ..git import git_output_or_empty, list_child_repos, run_git


def run(base_dir: Path | None = None, runner: Callable = run_git) -> int:
    base = base_dir or Path.cwd()
    repos = list_child_repos(base)
    if not repos:
        print(f"No git repositories found in {base}.")
        return 1

    print(f"== Force-pushing current branches of all repos in {base} ==")
    for repo in repos:
        print("--------------------------------------------")
        print(f"-> Repo: {repo.name}")
        branch = git_output_or_empty(
            ["rev-parse", "--abbrev-ref", "HEAD"],
            cwd=repo,
        ).strip()
        if branch in {"HEAD", "detached", ""}:
            print("  Skipping (detached HEAD)")
            continue

        print(f"  Detected branch: {branch}")
        print("  Force pushing to origin...")
        runner(["push", "-f", "origin", branch], cwd=repo)

    print("--------------------------------------------")
    print("All repos processed.")
    return 0
