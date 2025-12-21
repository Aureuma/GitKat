import subprocess
from pathlib import Path

from gitkat.commands import rewrite
from tests.helpers import commit_file


def init_repo(tmp_path: Path, name: str = "repo") -> Path:
    repo = tmp_path / name
    repo.mkdir(parents=True, exist_ok=True)
    subprocess.run(["git", "init"], cwd=repo, check=True, capture_output=True)
    return repo


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
    opts = rewrite.RewriteOptions(new_email="new@example.test", blob_map=["old:new"])
    exit_code = rewrite.run(opts, Path.cwd())
    assert exit_code == 1


def test_rewrite_invalid_blob_map():
    opts = rewrite.RewriteOptions(blob_map=["invalid"])
    exit_code = rewrite.run(opts, Path.cwd())
    assert exit_code == 1


def test_rewrite_no_mappings():
    opts = rewrite.RewriteOptions()
    exit_code = rewrite.run(opts, Path.cwd())
    assert exit_code == 1


def test_rewrite_requires_new_email():
    opts = rewrite.RewriteOptions(old_emails=["old@example.test"], blob_map=["old:new"])
    exit_code = rewrite.run(opts, Path.cwd())
    assert exit_code == 1


def test_rewrite_no_repos(tmp_path: Path):
    opts = rewrite.RewriteOptions(blob_map=["old:new"])
    exit_code = rewrite.run(opts, tmp_path)
    assert exit_code == 1


def test_resolve_repos_child(tmp_path: Path):
    repo = init_repo(tmp_path, "child")
    repos = rewrite._resolve_repos(tmp_path)
    assert repos == [(repo, "child")]


def test_resolve_repos_inside(tmp_path: Path):
    repo = init_repo(tmp_path, "root")
    nested = repo / "nested"
    nested.mkdir()
    repos = rewrite._resolve_repos(nested)
    assert repos == [(repo, str(repo))]


def test_resolve_repos_none(tmp_path: Path):
    assert rewrite._resolve_repos(tmp_path) == []


def test_capture_and_restore_remotes(tmp_path: Path):
    repo = init_repo(tmp_path, "remote-repo")
    subprocess.run(["git", "remote", "add", "origin", "https://example.test/repo.git"], cwd=repo, check=True)
    subprocess.run(["git", "remote", "set-url", "--add", "--push", "origin", "https://push.example.test/repo.git"], cwd=repo, check=True)

    captured = rewrite._capture_remotes(repo)
    assert captured

    subprocess.run(["git", "remote", "remove", "origin"], cwd=repo, check=True)
    rewrite._restore_remotes(repo, captured)
    remotes = subprocess.check_output(["git", "remote", "-v"], cwd=repo, text=True)
    assert "origin" in remotes


def test_restore_remotes_multiple_fetch_urls(tmp_path: Path, monkeypatch):
    repo = init_repo(tmp_path, "multi-remote")
    remote = rewrite.RemoteConfig(
        name="origin",
        fetch_urls=["https://example.test/one.git", "https://example.test/two.git"],
        push_urls=["https://example.test/push.git"],
    )
    calls = []

    def fake_run_git(args, cwd, check=False, **kwargs):
        calls.append(args)
        class Result:
            returncode = 1 if args[:2] == ["remote", "add"] else 0
        return Result()

    monkeypatch.setattr(rewrite, "run_git", fake_run_git)
    rewrite._restore_remotes(repo, [remote])

    assert any(args[:2] == ["remote", "set-url"] for args in calls)


def test_run_filter_repo_invokes_runner(tmp_path: Path):
    repo = init_repo(tmp_path, "filter-repo")
    received = {}

    def fake_runner(cmd, cwd, env, check, text):
        received["cmd"] = cmd
        received["cwd"] = cwd
        received["env"] = env
        commit_path = Path(cmd[cmd.index("--commit-callback") + 1])
        file_info_path = Path(cmd[cmd.index("--file-info-callback") + 1])
        assert commit_path.read_text().strip()
        assert file_info_path.read_text().strip()

    rewrite._run_filter_repo(repo, {"TEST": "1"}, runner=fake_runner)
    assert received["cmd"][0:2] == ["git", "filter-repo"]
    assert received["cwd"] == repo


def test_count_matching_emails(tmp_path: Path):
    repo = init_repo(tmp_path, "email-repo")
    commit_file(
        repo,
        "file.txt",
        "content",
        author_name="Sample User",
        author_email="alpha@example.test",
    )
    count = rewrite._count_matching_emails(repo, "alpha@example.test")
    assert count == 1


def test_count_matching_emails_empty(tmp_path: Path):
    repo = init_repo(tmp_path, "email-empty")
    assert rewrite._count_matching_emails(repo, "") == 0


def test_print_summary(tmp_path: Path, capsys):
    repo = init_repo(tmp_path, "summary-repo")
    commit_file(
        repo,
        "file.txt",
        "content",
        author_name="Sample User",
        author_email="alpha@example.test",
    )
    subprocess.run(["git", "remote", "add", "origin", "https://example.test/repo.git"], cwd=repo, check=True)
    opts = rewrite.RewriteOptions(new_email="alpha@example.test", blob_map=["a:b"])
    rewrite._print_summary(repo, opts)
    captured = capsys.readouterr().out
    assert "Total commits" in captured
    assert "Remote(s):" in captured


def test_print_summary_without_email(tmp_path: Path, capsys):
    repo = init_repo(tmp_path, "summary-empty")
    commit_file(
        repo,
        "file.txt",
        "content",
        author_name="Sample User",
        author_email="alpha@example.test",
    )
    opts = rewrite.RewriteOptions(blob_map=["a:b"])
    rewrite._print_summary(repo, opts)
    captured = capsys.readouterr().out
    assert "identity rewrite skipped" in captured
