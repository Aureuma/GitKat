from pathlib import Path

from gitkat.commands import push
from tests.helpers import commit_file


def test_push_invokes_force_push(git_repo):
    commit_file(
        git_repo,
        "file.txt",
        "content",
        author_name="Sample User",
        author_email="sample@example.test",
    )
    calls = []

    def stub_runner(args, *, cwd, check=True, capture_output=False, text=True):
        calls.append((args, cwd))
        class Result:
            returncode = 0
        return Result()

    exit_code = push.run(Path(git_repo.parent), runner=stub_runner)
    assert exit_code == 0
    assert any(call[0][:2] == ["push", "-f"] for call in calls)


def test_push_skips_detached_head(monkeypatch, git_repo, capsys):
    commit_file(
        git_repo,
        "file.txt",
        "content",
        author_name="Sample User",
        author_email="sample@example.test",
    )
    monkeypatch.setattr(push, "git_output_or_empty", lambda *args, **kwargs: "HEAD\n")
    calls = []

    def stub_runner(args, *, cwd, check=True, capture_output=False, text=True):
        calls.append((args, cwd))
        class Result:
            returncode = 0
        return Result()

    exit_code = push.run(Path(git_repo.parent), runner=stub_runner)
    captured = capsys.readouterr().out
    assert exit_code == 0
    assert "Skipping (detached HEAD)" in captured
    assert not calls


def test_push_no_repos(tmp_path, capsys):
    exit_code = push.run(tmp_path)
    captured = capsys.readouterr().out
    assert exit_code == 1
    assert "No git repositories found" in captured
