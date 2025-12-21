"""Lightweight helpers for invoking git."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import Sequence


def run_git(
    args: Sequence[str],
    *,
    cwd: Path,
    check: bool = True,
    capture_output: bool = False,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=cwd,
        check=check,
        capture_output=capture_output,
        text=True,
    )


def list_child_repos(base_dir: Path) -> list[Path]:
    repos: list[Path] = []
    for entry in sorted(base_dir.iterdir()):
        if entry.is_dir() and (entry / ".git").is_dir():
            repos.append(entry)
    return repos


def list_repos_recursive(base_dir: Path) -> list[Path]:
    repos: list[Path] = []
    for root, dirs, _files in os.walk(base_dir):
        if ".git" in dirs:
            repos.append(Path(root))
            # Avoid recursing into .git directories.
            dirs[:] = [d for d in dirs if d != ".git"]
    return repos


def find_git_root(start: Path) -> Path | None:
    try:
        result = subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=start,
            text=True,
        )
    except subprocess.CalledProcessError:
        return None
    return Path(result.strip())


def git_output(args: Sequence[str], *, cwd: Path) -> str:
    return run_git(args, cwd=cwd, capture_output=True).stdout


def git_output_or_empty(args: Sequence[str], *, cwd: Path) -> str:
    try:
        return git_output(args, cwd=cwd)
    except subprocess.CalledProcessError:
        return ""
