import requests

from gitkit.commands import github_emails


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


def test_run_requires_token():
    assert github_emails.run(None) == 1
