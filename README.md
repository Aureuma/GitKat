# 𝔾𝚒𝚝𝕂𝚊𝚝 ⫷⫸

[![Tests](https://github.com/Aureuma/GitKat/actions/workflows/tests.yml/badge.svg)](https://github.com/Aureuma/GitKat/actions/workflows/tests.yml)
[![GitHub](https://img.shields.io/badge/GitHub-Aureuma/GitKat-181717?logo=github&logoColor=white)](https://github.com/Aureuma/GitKat)
[![crates.io](https://img.shields.io/crates/v/gitkat.svg?logo=rust)](https://crates.io/crates/gitkat)
[![Homebrew](https://img.shields.io/badge/Homebrew-Aureuma%2Fgitkat-FBB040?logo=homebrew&logoColor=black)](https://github.com/Aureuma/GitKat/blob/main/Formula/gitkat.rb)
[![npm](https://img.shields.io/npm/v/@aureuma/gitkat?logo=npm)](https://www.npmjs.com/package/@aureuma/gitkat)
[![PyPI](https://img.shields.io/pypi/v/gitkat?logo=pypi&logoColor=white)](https://pypi.org/project/gitkat/)

𝔾𝚒𝚝𝕂𝚊𝚝 ⫷⫸ GitKat is a Rust toolkit for managing Git repositories in bulk. It ships a single CLI, `gk`, for a packaged, testable workflow.

## Install

```sh
# crates.io
cargo install gitkat --locked

# Homebrew
brew tap Aureuma/gitkat
brew install gitkat

# npm
npm install -g @aureuma/gitkat

# pipx (recommended)
pipx install gitkat

# pip
python -m pip install gitkat

# local builds
cargo build --release
./target/release/gk --help
```

The pip/npm wrappers download the Rust binary from GitHub Releases on first run. Set `GITKAT_RELEASE_BASE` to override the download base URL. The pip wrapper respects `GITKAT_CACHE_DIR`.

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
- `gk rewrite`: rewrite identity metadata and/or blob contents using a Rust gitoxide (gix) rewriter.
- `gk github-emails --token <token>`: find contribution emails across GitHub repos you can access.
- `gk verify-rewrite`: compare rewrite output against `git-filter-repo`, with optional blob-only checks against BFG.

## Command responsibilities

GitKat keeps history rewriting and metadata discovery separate:

- `gk rewrite` is the only command that rewrites history. It updates author/committer metadata (identity) and rewrites blob contents or paths in the commit tree. Commit messages are preserved.
- `gk check`, `gk report`, and `gk github-emails` only read history to help you find the right identities or repositories to target. They never rewrite objects.
- `gk verify-rewrite` validates parity against other tools; it does not change your working repo.

## Rewrite notes

`gk rewrite` preserves the existing behavior of `rewrite.sh`, including case-aware blob mapping and commit metadata rewrites. The rewrite engine is implemented directly in Rust using gitoxide (gix).

Examples:

```sh
# Identity rewrite
gk rewrite -n "New Name" -e "new@example.test" -o "old@example.test"

# Blob rewrite with preserved casing and case-insensitive matching
gk rewrite -m foo:bar --ignore-case --preserve-case

# Regex blob rewrite
gk rewrite -m "token_[0-9]+:REDACTED" --regex

# Escape literal colons in mappings
gk rewrite -m "foo\\:bar:baz"

# Exclude files from blob rewrites
gk rewrite -m token:REDACTED -x "data/*.csv" -x "vendor/*"

# Rename file paths using the same mappings
gk rewrite -m oldname:newname --rename-files

# Delete a file or glob across history
gk rewrite --delete-path "path/to/file.txt"
gk rewrite --delete-path "assets/**/*.png"
```

Delete paths accept glob patterns and log each removed file in the colored rewrite output.

## Development

```sh
gh workflow run tests.yml
gh run watch --workflow tests.yml --exit-status
cargo run -p gitkat -- --help
```

All tests and verification run in GitHub Actions. Local test runs are intentionally avoided to keep CI parity.

Testing strategy:

- CI is the source of truth for tests and verification.
- `Tests` runs linting, unit tests, doc builds, and verification.
- Verification covers identity, blob, regex, and BFG (blob-only).
- Concurrency is capped to respect public repo minutes.

## License

MIT License. See `LICENSE`.
