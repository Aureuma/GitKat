from __future__ import annotations

import os
import subprocess
from pathlib import Path


def commit_file(
    repo: Path,
    filename: str,
    content: str,
    *,
    author_name: str,
    author_email: str,
    message: str = "commit",
) -> None:
    file_path = repo / filename
    file_path.write_text(content, encoding="utf-8")
    subprocess.run(["git", "add", filename], cwd=repo, check=True)
    env = os.environ.copy()
    env.update(
        {
            "GIT_AUTHOR_NAME": author_name,
            "GIT_AUTHOR_EMAIL": author_email,
            "GIT_COMMITTER_NAME": author_name,
            "GIT_COMMITTER_EMAIL": author_email,
        }
    )
    subprocess.run(["git", "commit", "-m", message], cwd=repo, check=True, env=env)
