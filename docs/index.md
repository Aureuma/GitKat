# GitKit

GitKit is a Python CLI for bulk Git repository maintenance. It keeps the behavior of the original shell tooling while adding packaging, tests, and documentation.

## Highlights

- Search commit metadata across many repos.
- List author emails for auditing.
- Force-push current branches in bulk.
- Rewrite history with git-filter-repo, including case-preserving blob replacements.
- Query GitHub contribution emails via API.

## Quick start

```sh
gk check "Example Name"
gk report .
gk push
gk rewrite -m olddomain.com:newdomain.com --ignore-case --preserve-case
gk github-emails --token YOUR_GITHUB_TOKEN
```
