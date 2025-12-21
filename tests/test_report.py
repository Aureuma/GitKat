import subprocess
from pathlib import Path

from gitkat.commands import report
from tests.helpers import commit_file


def test_report_lists_unique_emails(git_repo, capsys):
    commit_file(
        git_repo,
        "alpha.txt",
        "one",
        author_name="Alpha User",
        author_email="alpha@example.test",
        message="first",
    )
    commit_file(
        git_repo,
        "beta.txt",
        "two",
        author_name="Beta User",
        author_email="beta@example.test",
        message="second",
    )
    exit_code = report.run(Path(git_repo.parent))
    captured = capsys.readouterr().out
    assert exit_code == 0
    assert "alpha@example.test" in captured
    assert "beta@example.test" in captured


def test_report_no_repos(tmp_path, capsys):
    exit_code = report.run(tmp_path)
    captured = capsys.readouterr().out
    assert exit_code == 1
    assert "No git repositories found" in captured


def test_report_no_commits(tmp_path, capsys):
    repo = tmp_path / "empty-repo"
    repo.mkdir()
    subprocess.run(["git", "init"], cwd=repo, check=True, capture_output=True)
    exit_code = report.run(tmp_path)
    captured = capsys.readouterr().out
    assert exit_code == 0
    assert "(no commits)" in captured
