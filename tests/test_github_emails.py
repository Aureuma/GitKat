import requests

from gitkat.commands import github_emails


class DummyResponse:
    def __init__(self, status_code, payload):
        self.status_code = status_code
        self._payload = payload

    def json(self):
        return self._payload

    def raise_for_status(self):
        if self.status_code >= 400:
            raise requests.HTTPError("error")


class DummySession:
    def __init__(self, responses):
        self.responses = responses
        self.calls = []

    def get(self, url, params=None):
        self.calls.append((url, params))
        key = (url, tuple(sorted((params or {}).items())))
        return self.responses.get(key, DummyResponse(404, {}))


def test_get_contribution_emails_collects_from_commits_and_prs():
    base = github_emails.GITHUB_API_URL
    responses = {
        (f"{base}/repos/org/repo/commits", (("author", "user"), ("page", 1), ("per_page", 100))): DummyResponse(
            200,
            [
                {"commit": {"author": {"email": "alpha@example.test"}, "committer": {"email": "beta@example.test"}}}
            ],
        ),
        (f"{base}/repos/org/repo/commits", (("author", "user"), ("page", 2), ("per_page", 100))): DummyResponse(200, []),
        (f"{base}/repos/org/repo/pulls", (("page", 1), ("per_page", 100), ("state", "all"))): DummyResponse(
            200,
            [{"number": 5, "user": {"login": "user"}}],
        ),
        (f"{base}/repos/org/repo/pulls", (("page", 2), ("per_page", 100), ("state", "all"))): DummyResponse(200, []),
        (f"{base}/repos/org/repo/pulls/5/commits", ()): DummyResponse(
            200,
            [
                {"commit": {"author": {"email": "gamma@example.test"}, "committer": {"email": "beta@example.test"}}}
            ],
        ),
    }
    session = DummySession(responses)
    emails = github_emails.get_contribution_emails(session, "org", "repo", "user")
    assert emails == {"alpha@example.test", "beta@example.test", "gamma@example.test"}


def test_create_github_session_sets_headers():
    session = github_emails.create_github_session("token")
    assert session.headers["Authorization"] == "Bearer token"
    assert "GitKat-Email-Finder" in session.headers["User-Agent"]


def test_get_authenticated_user():
    base = github_emails.GITHUB_API_URL
    responses = {(f"{base}/user", ()): DummyResponse(200, {"login": "user"})}
    session = DummySession(responses)
    user = github_emails.get_authenticated_user(session)
    assert user["login"] == "user"


def test_get_user_repos_and_org_repos():
    base = github_emails.GITHUB_API_URL
    responses = {
        (f"{base}/user/repos", (("affiliation", "owner"), ("page", 1), ("per_page", 100))): DummyResponse(
            200,
            [{"name": "alpha", "owner": {"login": "acct"}}],
        ),
        (f"{base}/user/repos", (("affiliation", "owner"), ("page", 2), ("per_page", 100))): DummyResponse(200, []),
        (f"{base}/user/orgs", ()): DummyResponse(200, [{"login": "org"}]),
        (f"{base}/orgs/org/repos", (("page", 1), ("per_page", 100))): DummyResponse(
            200,
            [
                {"name": "bravo", "owner": {"login": "org"}, "permissions": {"push": True}},
                {"name": "skip", "owner": {"login": "org"}, "permissions": {"push": False}},
            ],
        ),
        (f"{base}/orgs/org/repos", (("page", 2), ("per_page", 100))): DummyResponse(200, []),
    }
    session = DummySession(responses)
    user_repos = github_emails.get_user_repos(session)
    org_repos = github_emails.get_org_repos(session)
    assert len(user_repos) == 1
    assert len(org_repos) == 1
    assert org_repos[0]["name"] == "bravo"


def test_run_requires_token():
    assert github_emails.run(None) == 1


def test_run_success(monkeypatch, capsys):
    monkeypatch.setattr(github_emails, "create_github_session", lambda token: object())
    monkeypatch.setattr(github_emails, "get_authenticated_user", lambda session: {"login": "user"})
    monkeypatch.setattr(
        github_emails,
        "get_user_repos",
        lambda session: [{"owner": {"login": "acct"}, "name": "alpha"}],
    )
    monkeypatch.setattr(
        github_emails,
        "get_org_repos",
        lambda session: [{"owner": {"login": "org"}, "name": "bravo"}],
    )
    monkeypatch.setattr(
        github_emails,
        "get_contribution_emails",
        lambda session, repo_owner, repo_name, username: {f"{repo_name}@example.test"},
    )
    exit_code = github_emails.run("token")
    captured = capsys.readouterr().out
    assert exit_code == 0
    assert "Authenticated as" in captured
    assert "alpha@example.test" in captured
    assert "bravo@example.test" in captured


def test_run_request_error(monkeypatch):
    def raise_error(session):
        raise requests.exceptions.RequestException("boom")

    monkeypatch.setattr(github_emails, "create_github_session", lambda token: object())
    monkeypatch.setattr(github_emails, "get_authenticated_user", raise_error)
    assert github_emails.run("token") == 1
