from pathlib import Path

from gitkit.commands import push
from tests.conftest import commit_file


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
