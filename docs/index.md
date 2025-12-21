<p align="center">
  <a href="https://github.com/Aureuma/GitKat">
    <img src="assets/logo.svg" alt="⫷⫸" width="96" height="96">
  </a>
</p>

<p align="center">
  <a href="https://github.com/Aureuma/GitKat">
    <img alt="GitHub" src="https://img.shields.io/badge/GitHub-Aureuma/GitKat-181717?logo=github&logoColor=white">
  </a>
  <a href="https://pypi.org/project/GitKat/">
    <img alt="PyPI" src="https://img.shields.io/pypi/v/GitKat.svg?logo=pypi&logoColor=white">
  </a>
</p>

# 𝔾𝚒𝚝𝕂𝚊𝚝 ⫷⫸

𝔾𝚒𝚝𝕂𝚊𝚝 ⫷⫸ (GitKat) is a Python CLI for bulk Git repository maintenance. It keeps the behavior of the original shell tooling while adding packaging, tests, and documentation.

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
