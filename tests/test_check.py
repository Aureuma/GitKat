from pathlib import Path

from gitkit.commands import check
from tests.conftest import commit_file


def test_check_finds_name(git_repo, capsys):
    commit_file(
        git_repo,
        "file.txt",
        "content",
        author_name="Sample Author",
        author_email="sample@example.test",
    )
    exit_code = check.run("author", Path(git_repo.parent))
    captured = capsys.readouterr().out
    assert exit_code == 0
    assert "Found commits" in captured


def test_check_no_match(git_repo, capsys):
    commit_file(
        git_repo,
        "file.txt",
        "content",
        author_name="Sample Author",
        author_email="sample@example.test",
    )
    exit_code = check.run("missing", Path(git_repo.parent))
    captured = capsys.readouterr().out
    assert exit_code == 0
    assert "Nothing." in captured
