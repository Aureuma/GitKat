<a href="https://github.com/Aureuma/GitKat">
  <img src="docs/assets/logo.svg" alt="⫷⫸" width="88" height="88">
</a>

# 𝔾𝚒𝚝𝕂𝚊𝚝 ⫷⫸

[![CI](https://github.com/Aureuma/GitKat/actions/workflows/ci.yml/badge.svg)](https://github.com/Aureuma/GitKat/actions/workflows/ci.yml)
[![GitHub](https://img.shields.io/badge/GitHub-Aureuma/GitKat-181717?logo=github&logoColor=white)](https://github.com/Aureuma/GitKat)

𝔾𝚒𝚝𝕂𝚊𝚝 ⫷⫸ (GitKat) is a Rust toolkit for managing Git repositories in bulk. It ships a single CLI, `gk`, that mirrors the legacy shell scripts while adding a packaged, testable workflow.

## Install

```sh
# crates.io
cargo install gitkat

# Homebrew (tap this repo)
brew install Aureuma/gitkat/gitkat

# npm
npm install -g @aureuma/gitkat

# pip (Python wrapper downloads the Rust binary)
python -m pip install gitkat

# local builds
cargo build --release
./target/release/gk --help
```

The pip/npm wrappers download the Rust binary from GitHub Releases on first run. Set `GITKAT_RELEASE_BASE` to override the download base URL.

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
- `gk verify-rewrite`: compare rewrite output against `git-filter-repo` across real repositories.

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
cargo test --workspace
cargo run -p gitkat -- --help
gk verify-rewrite --ci --with-blob
mdbook build
```

`gk verify-rewrite` compares against `git-filter-repo`, so install it if you want equivalence checks.

## License

MIT License. See `LICENSE`.
