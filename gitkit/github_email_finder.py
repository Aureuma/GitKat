#!/usr/bin/env python3
"""
GitHub Email Finder

This script identifies all unique email addresses associated with your GitHub contributions
across all repositories you own and all repositories under organizations where you have
push or contribution access.

Usage:
    python github_email_finder.py [--token TOKEN]
"""

import argparse
import sys
from collections import defaultdict
from typing import Dict, List, Set

import requests

# GitHub API base URL
GITHUB_API_URL = "https://api.github.com"


def create_github_session(token: str) -> requests.Session:
    """
    Create a GitHub API session with authentication.
    """
    session = requests.Session()
    session.headers.update({
        "Authorization": f"Bearer {token}",
        "Accept": "application/vnd.github.v3+json",
        "User-Agent": "GitHub-Email-Finder"
    })
    return session


def get_authenticated_user(session: requests.Session) -> Dict:
    """
    Get information about the authenticated user.
    """
    response = session.get(f"{GITHUB_API_URL}/user")
    response.raise_for_status()
    return response.json()


def get_user_repos(session: requests.Session) -> List[Dict]:
    """
    Get all repositories owned by the authenticated user.
    """
    repos = []
    page = 1

    while True:
        response = session.get(
            f"{GITHUB_API_URL}/user/repos",
            params={"per_page": 100, "page": page, "affiliation": "owner"}
        )
        response.raise_for_status()

        batch = response.json()
        if not batch:
            break

        repos.extend(batch)
        page += 1

    return repos


def get_org_repos(session: requests.Session) -> List[Dict]:
    """
    Get all repositories from organizations where the user has push access.
    """
    # First get all organizations the user is a member of
    orgs_response = session.get(f"{GITHUB_API_URL}/user/orgs")
    orgs_response.raise_for_status()
    orgs = orgs_response.json()

    all_org_repos = []

    for org in orgs:
        org_name = org["login"]
        page = 1

        while True:
            response = session.get(
                f"{GITHUB_API_URL}/orgs/{org_name}/repos",
                params={"per_page": 100, "page": page}
            )
            response.raise_for_status()

            batch = response.json()
            if not batch:
                break

            # Filter to only include repos where the user has push access
            push_repos = [repo for repo in batch if
                          repo.get("permissions", {}).get("push", False)]
            all_org_repos.extend(push_repos)

            page += 1

    return all_org_repos


def get_contribution_emails(session: requests.Session, repo_owner: str,
                            repo_name: str, username: str) -> Set[str]:
    """
    Get all email addresses associated with a user's contributions to a repository.
    """
    emails = set()

    # Get commits by the user
    page = 1
    while True:
        commits_response = session.get(
            f"{GITHUB_API_URL}/repos/{repo_owner}/{repo_name}/commits",
            params={"author": username, "per_page": 100, "page": page}
        )

        # Skip if we don't have access or other issues
        if commits_response.status_code != 200:
            break

        commits = commits_response.json()
        if not commits:
            break

        for commit in commits:
            # Get author email
            author = commit.get("commit", {}).get("author", {})
            if author and "email" in author:
                emails.add(author["email"])

            # Get committer email
            committer = commit.get("commit", {}).get("committer", {})
            if committer and "email" in committer:
                emails.add(committer["email"])

        page += 1

    # Get pull requests by the user
    page = 1
    while True:
        prs_response = session.get(
            f"{GITHUB_API_URL}/repos/{repo_owner}/{repo_name}/pulls",
            params={"state": "all", "per_page": 100, "page": page}
        )

        # Skip if we don't have access or other issues
        if prs_response.status_code != 200:
            break

        prs = prs_response.json()
        if not prs:
            break

        # Filter PRs by the user
        user_prs = [pr for pr in prs if
                    pr.get("user", {}).get("login") == username]

        for pr in user_prs:
            # For each PR, get the commits
            pr_number = pr["number"]
            pr_commits_response = session.get(
                f"{GITHUB_API_URL}/repos/{repo_owner}/{repo_name}/pulls/{pr_number}/commits"
            )

            if pr_commits_response.status_code == 200:
                pr_commits = pr_commits_response.json()

                for commit in pr_commits:
                    # Get author email
                    author = commit.get("commit", {}).get("author", {})
                    if author and "email" in author:
                        emails.add(author["email"])

                    # Get committer email
                    committer = commit.get("commit", {}).get("committer", {})
                    if committer and "email" in committer:
                        emails.add(committer["email"])

        page += 1

    return emails


def main():
    parser = argparse.ArgumentParser(
        description="Find all email addresses associated with your GitHub contributions")
    parser.add_argument("--token", help="GitHub personal access token")
    args = parser.parse_args()

    # Get token from args
    token = args.token

    if not token:
        print("Please provide a token using the --token argument.")
        sys.exit(1)

    try:
        session = create_github_session(token)

        # Get authenticated user info
        user = get_authenticated_user(session)
        username = user["login"]
        print(f"Authenticated as: {username}")

        # Get all repositories owned by the user
        print("\nFetching your repositories...")
        user_repos = get_user_repos(session)
        print(f"Found {len(user_repos)} repositories owned by you")

        # Get all repositories from organizations where the user has push access
        print("\nFetching organization repositories...")
        org_repos = get_org_repos(session)
        print(
            f"Found {len(org_repos)} organization repositories where you have push access")

        # Combine all repositories
        all_repos = user_repos + org_repos

        # Get all email addresses from contributions
        print(
            f"\nAnalyzing contributions across {len(all_repos)} repositories...")
        all_emails = set()
        repo_emails = defaultdict(set)

        for i, repo in enumerate(all_repos, 1):
            repo_owner = repo["owner"]["login"]
            repo_name = repo["name"]

            print(
                f"[{i}/{len(all_repos)}] Checking {repo_owner}/{repo_name}...")

            emails = get_contribution_emails(session, repo_owner, repo_name,
                                             username)
            if emails:
                repo_emails[f"{repo_owner}/{repo_name}"] = emails
                all_emails.update(emails)

        # Print results
        print("\n" + "=" * 60)
        print(
            f"Found {len(all_emails)} unique email addresses across {len(repo_emails)} repositories:")
        print("=" * 60)

        for email in sorted(all_emails):
            print(email)

        print("\nRepository breakdown:")
        for repo_name, emails in repo_emails.items():
            print(f"\n{repo_name}:")
            for email in sorted(emails):
                print(f"  - {email}")

    except requests.exceptions.RequestException as e:
        print(f"Error: {e}")
        sys.exit(1)
    except KeyboardInterrupt:
        print("\nOperation cancelled by user")
        sys.exit(0)


if __name__ == "__main__":
    main()
