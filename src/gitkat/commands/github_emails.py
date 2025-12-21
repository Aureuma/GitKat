"""Find GitHub contribution emails across repositories."""

from __future__ import annotations

from collections import defaultdict
from typing import Dict, List, Set

import requests

GITHUB_API_URL = "https://api.github.com"


def create_github_session(token: str) -> requests.Session:
    session = requests.Session()
    session.headers.update(
        {
            "Authorization": f"Bearer {token}",
            "Accept": "application/vnd.github.v3+json",
            "User-Agent": "GitKat-Email-Finder",
        }
    )
    return session


def get_authenticated_user(session: requests.Session) -> Dict:
    response = session.get(f"{GITHUB_API_URL}/user")
    response.raise_for_status()
    return response.json()


def get_user_repos(session: requests.Session) -> List[Dict]:
    repos: list[Dict] = []
    page = 1
    while True:
        response = session.get(
            f"{GITHUB_API_URL}/user/repos",
            params={"per_page": 100, "page": page, "affiliation": "owner"},
        )
        response.raise_for_status()
        batch = response.json()
        if not batch:
            break
        repos.extend(batch)
        page += 1
    return repos


def get_org_repos(session: requests.Session) -> List[Dict]:
    orgs_response = session.get(f"{GITHUB_API_URL}/user/orgs")
    orgs_response.raise_for_status()
    orgs = orgs_response.json()

    all_org_repos: list[Dict] = []
    for org in orgs:
        org_name = org["login"]
        page = 1
        while True:
            response = session.get(
                f"{GITHUB_API_URL}/orgs/{org_name}/repos",
                params={"per_page": 100, "page": page},
            )
            response.raise_for_status()
            batch = response.json()
            if not batch:
                break
            push_repos = [repo for repo in batch if repo.get("permissions", {}).get("push", False)]
            all_org_repos.extend(push_repos)
            page += 1
    return all_org_repos


def get_contribution_emails(
    session: requests.Session,
    repo_owner: str,
    repo_name: str,
    username: str,
) -> Set[str]:
    emails: set[str] = set()
    page = 1
    while True:
        commits_response = session.get(
            f"{GITHUB_API_URL}/repos/{repo_owner}/{repo_name}/commits",
            params={"author": username, "per_page": 100, "page": page},
        )
        if commits_response.status_code != 200:
            break
        commits = commits_response.json()
        if not commits:
            break
        for commit in commits:
            author = commit.get("commit", {}).get("author", {})
            if author and "email" in author:
                emails.add(author["email"])
            committer = commit.get("commit", {}).get("committer", {})
            if committer and "email" in committer:
                emails.add(committer["email"])
        page += 1

    page = 1
    while True:
        prs_response = session.get(
            f"{GITHUB_API_URL}/repos/{repo_owner}/{repo_name}/pulls",
            params={"state": "all", "per_page": 100, "page": page},
        )
        if prs_response.status_code != 200:
            break
        prs = prs_response.json()
        if not prs:
            break
        user_prs = [pr for pr in prs if pr.get("user", {}).get("login") == username]
        for pr in user_prs:
            pr_number = pr["number"]
            pr_commits_response = session.get(
                f"{GITHUB_API_URL}/repos/{repo_owner}/{repo_name}/pulls/{pr_number}/commits"
            )
            if pr_commits_response.status_code == 200:
                pr_commits = pr_commits_response.json()
                for commit in pr_commits:
                    author = commit.get("commit", {}).get("author", {})
                    if author and "email" in author:
                        emails.add(author["email"])
                    committer = commit.get("commit", {}).get("committer", {})
                    if committer and "email" in committer:
                        emails.add(committer["email"])
        page += 1

    return emails


def run(token: str | None) -> int:
    if not token:
        print("Please provide a token using the --token argument.")
        return 1

    try:
        session = create_github_session(token)
        user = get_authenticated_user(session)
        username = user["login"]
        print(f"Authenticated as: {username}")

        print("\nFetching your repositories...")
        user_repos = get_user_repos(session)
        print(f"Found {len(user_repos)} repositories owned by you")

        print("\nFetching organization repositories...")
        org_repos = get_org_repos(session)
        print(f"Found {len(org_repos)} organization repositories where you have push access")

        all_repos = user_repos + org_repos
        print(f"\nAnalyzing contributions across {len(all_repos)} repositories...")
        all_emails: set[str] = set()
        repo_emails: dict[str, set[str]] = defaultdict(set)

        for i, repo in enumerate(all_repos, 1):
            repo_owner = repo["owner"]["login"]
            repo_name = repo["name"]
            print(f"[{i}/{len(all_repos)}] Checking {repo_owner}/{repo_name}...")
            emails = get_contribution_emails(session, repo_owner, repo_name, username)
            if emails:
                repo_emails[f"{repo_owner}/{repo_name}"] = emails
                all_emails.update(emails)

        print("\n" + "=" * 60)
        print(
            f"Found {len(all_emails)} unique email addresses across {len(repo_emails)} repositories:"
        )
        print("=" * 60)

        for email in sorted(all_emails):
            print(email)

        print("\nRepository breakdown:")
        for repo_name, emails in repo_emails.items():
            print(f"\n{repo_name}:")
            for email in sorted(emails):
                print(f"  - {email}")

    except requests.exceptions.RequestException as exc:
        print(f"Error: {exc}")
        return 1
    except KeyboardInterrupt:
        print("\nOperation cancelled by user")
        return 0

    return 0
