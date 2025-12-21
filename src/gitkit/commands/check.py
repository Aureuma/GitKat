"""Search commit history for matching author or committer names."""

from __future__ import annotations

from pathlib import Path

from ..git import git_output, list_child_repos


def run(name: str, base_dir: Path | None = None) -> int:
    base = base_dir or Path.cwd()
    repos = list_child_repos(base)
    if not repos:
        print(f"No git repositories found in {base}.")
        return 1

    needle = name.lower()
    for repo in repos:
        print(f"== Checking repo: {repo.name} ==")
        output = git_output(
            ["log", "--all", "--pretty=%an <%ae>%n%cn <%ce>"],
            cwd=repo,
        )
        if needle in output.lower():
            print(f"Found commits with '{name}' in author or committer fields.")
        else:
            print("Nothing.")
        print()
    return 0
