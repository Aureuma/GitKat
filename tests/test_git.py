import subprocess
from pathlib import Path

from gitkat import git


def init_repo(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)
    subprocess.run(["git", "init"], cwd=path, check=True, capture_output=True)


def test_list_child_repos(tmp_path: Path):
    repo_a = tmp_path / "repo-a"
    repo_b = tmp_path / "repo-b"
    init_repo(repo_a)
    init_repo(repo_b)
    repos = git.list_child_repos(tmp_path)
    assert repos == [repo_a, repo_b]


def test_list_repos_recursive(tmp_path: Path):
    repo_root = tmp_path / "root"
    repo_nested = tmp_path / "nested" / "repo"
    init_repo(repo_root)
    init_repo(repo_nested)
    repos = git.list_repos_recursive(tmp_path)
    assert repo_root in repos
    assert repo_nested in repos


def test_find_git_root(tmp_path: Path):
    repo = tmp_path / "repo"
    init_repo(repo)
    child = repo / "child"
    child.mkdir()
    root = git.find_git_root(child)
    assert root == repo


def test_find_git_root_none(tmp_path: Path):
    assert git.find_git_root(tmp_path) is None


def test_git_output_or_empty_on_error(tmp_path: Path):
    repo = tmp_path / "repo"
    init_repo(repo)
    output = git.git_output_or_empty(["rev-parse", "--verify", "refs/heads/nope"], cwd=repo)
    assert output == ""


def test_git_output_or_empty_handles_called_process_error(monkeypatch, tmp_path: Path):
    repo = tmp_path / "repo"
    init_repo(repo)

    def raise_error(*args, **kwargs):
        raise subprocess.CalledProcessError(1, "git")

    monkeypatch.setattr(git, "run_git", raise_error)
    output = git.git_output_or_empty(["status"], cwd=repo)
    assert output == ""


def test_run_git_status(tmp_path: Path):
    repo = tmp_path / "repo"
    init_repo(repo)
    result = git.run_git(["status", "-s"], cwd=repo, capture_output=True)
    assert result.returncode == 0
