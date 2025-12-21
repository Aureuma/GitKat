<a href="https://github.com/Aureuma/GitKat">
  <img src="docs/assets/logo.svg" alt="⫷⫸" width="88" height="88">
</a>

# 𝔾𝚒𝚝𝕂𝚊𝚝 ⫷⫸

[![CI](https://github.com/Aureuma/GitKat/actions/workflows/ci.yml/badge.svg)](https://github.com/Aureuma/GitKat/actions/workflows/ci.yml)
[![PyPI](https://img.shields.io/pypi/v/GitKat.svg?logo=pypi&logoColor=white)](https://pypi.org/project/GitKat/)
[![codecov](https://codecov.io/gh/Aureuma/GitKat/branch/main/graph/badge.svg)](https://codecov.io/gh/Aureuma/GitKat)
[![GitHub](https://img.shields.io/badge/GitHub-Aureuma/GitKat-181717?logo=github&logoColor=white)](https://github.com/Aureuma/GitKat)

𝔾𝚒𝚝𝕂𝚊𝚝 ⫷⫸ (GitKat) is a Python toolkit for managing Git repositories in bulk. It ships a single CLI, `gk`, that mirrors the legacy shell scripts while adding a packaged, testable workflow.

## Install

Using uv:

```sh
uv venv
uv pip install -e .
```

Using pip:

```sh
python -m venv .venv
. .venv/bin/activate
pip install -e .
```

## Quick start

```sh
gk check "Example Name"
gk report .
gk push
gk rewrite -m olddomain.com:newdomain.com --ignore-case --preserve-case
gk github-emails --token YOUR_GITHUB_TOKEN
```

## Commands

- `gk check <name>`: search author and committer names across repos in the current directory.
- `gk report [path]`: list unique author emails for each repo under a path.
- `gk push`: force-push the current branch of each repo in the current directory.
- `gk rewrite`: rewrite identity metadata and/or blob contents using git-filter-repo.
- `gk github-emails --token <token>`: find contribution emails across GitHub repos you can access.

## Rewrite notes

`gk rewrite` preserves the existing behavior of `rewrite.sh`, including case-aware blob mapping and commit metadata rewrites. It runs `git filter-repo` under the hood, so you need Git and git-filter-repo installed.

Examples:

```sh
# Identity rewrite
gk rewrite -n "New Name" -e "new@example.test" -o "old@example.test"

# Blob rewrite with preserved casing and case-insensitive matching
gk rewrite -m foo:bar --ignore-case --preserve-case

# Exclude files from blob rewrites
gk rewrite -m token:REDACTED -x "data/*.csv" -x "vendor/*"

# Rename file paths using the same mappings
gk rewrite -m oldname:newname --rename-files
```

## Development

```sh
uv pip install -e .
uv pip install -e . --group dev
uv run pytest
uv run mkdocs serve
```

Coverage (latest local run):

```sh
uv run pytest --cov=gitkat --cov-report=term-missing
```

## License

MIT License. See `LICENSE`.
