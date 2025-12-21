from pathlib import Path

from gitkit.commands import rewrite
from tests.conftest import commit_file


def test_rewrite_builds_env(monkeypatch, git_repo):
    commit_file(
        git_repo,
        "file.txt",
        "content",
        author_name="Sample User",
        author_email="sample@example.test",
    )
    captured = {}

    def fake_run_filter_repo(repo, env, runner=None):
        captured["env"] = env

    monkeypatch.setattr(rewrite, "_run_filter_repo", fake_run_filter_repo)
    monkeypatch.setattr(rewrite, "_capture_remotes", lambda repo: [])
    monkeypatch.setattr(rewrite, "_restore_remotes", lambda repo, remotes: None)

    opts = rewrite.RewriteOptions(
        new_name="New Name",
        new_email="new@example.test",
        old_emails=["old@example.test"],
        blob_map=["old:new"],
        exclude_patterns=["data/*.csv,logs/*"],
        preserve_case=True,
        ignore_case=True,
        rename_files=True,
    )

    exit_code = rewrite.run(opts, Path(git_repo))
    assert exit_code == 0
    env = captured["env"]
    assert env["GITKIT_BLOB_MAP"] == "old\tnew"
    assert "data/*.csv" in env["GITKIT_EXCLUDE_PATTERNS"]
    assert env["GITKIT_PRESERVE_CASE"] == "1"
    assert env["GITKIT_IGNORE_CASE"] == "1"
    assert env["GITKIT_RENAME_FILES"] == "1"


def test_rewrite_requires_old_emails():
    opts = rewrite.RewriteOptions(new_email="new@example.test")
    exit_code = rewrite.run(opts, Path.cwd())
    assert exit_code == 1
