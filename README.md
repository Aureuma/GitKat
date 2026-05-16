# 𝔾𝚒𝚝𝕂𝚊𝚝 ⫷⫸

[![Tests](https://github.com/Aureuma/GitKat/actions/workflows/tests.yml/badge.svg)](https://github.com/Aureuma/GitKat/actions/workflows/tests.yml)
[![GitHub](https://img.shields.io/badge/GitHub-Aureuma/GitKat-181717?logo=github&logoColor=white)](https://github.com/Aureuma/GitKat)
[![crates.io](https://img.shields.io/crates/v/gitkat.svg?logo=rust)](https://crates.io/crates/gitkat)
[![Homebrew](https://img.shields.io/badge/Homebrew-Aureuma%2Fgitkat-FBB040?logo=homebrew&logoColor=black)](https://github.com/Aureuma/GitKat/blob/main/Formula/gitkat.rb)
[![npm](https://img.shields.io/npm/v/@aureuma/gitkat?logo=npm)](https://www.npmjs.com/package/@aureuma/gitkat)
[![PyPI](https://img.shields.io/pypi/v/gitkat?logo=pypi&logoColor=white)](https://pypi.org/project/gitkat/)

GitKat is a Rust CLI for bulk Git repository maintenance. It focuses on fast, repeatable workflows for identity cleanup, content redaction, and repository audits across many repos.

## What GitKat does

- Audit commit metadata across many repositories.
- Inventory author emails to plan identity rewrites.
- Force-push current branches in bulk.
- Rewrite history in-process with a Rust gitoxide (gix) engine.
- Rename paths and delete files across all history.
- Verify rewrite parity against `git-filter-repo` and optionally BFG.

## Install

```sh
# crates.io
cargo install gitkat --locked

# Homebrew
brew tap Aureuma/gitkat
brew install gitkat

# npm
corepack pnpm install -g @aureuma/gitkat

# pipx (recommended)
pipx install gitkat

# pip
python -m pip install gitkat
```

The pip/npm wrappers download the Rust binary from GitHub Releases on first run.

## Quick start

```sh
# Inspect identities
gk check "Example Name"
gk report .

# Rewrite data in-place
gk rewrite -m olddomain.com:newdomain.com --ignore-case --preserve-case

# Discover GitHub contribution emails
gk github-emails --token YOUR_GITHUB_TOKEN
```

## Command overview

### `gk check <name>`

Search for a name in author/committer fields across child repos.

```sh
gk check "Example Name"
```

### `gk report [path]`

List unique author emails per repo, searching recursively from `path`.

```sh
gk report .
```

### `gk push`

Force-push the current branch and tags of each child repo to `origin`.

```sh
gk push
```

### `gk rewrite`

Rewrite identity metadata and/or blob contents using the gitoxide engine.

Identity rewrite:

```sh
gk rewrite -n "New Name" -e "new@example.test" -o "old@example.test"
```

Content rewrite (case-insensitive, case-preserving):

```sh
gk rewrite -m olddomain.com:newdomain.com --ignore-case --preserve-case
```

Regex mapping:

```sh
gk rewrite -m "token_[0-9]+:REDACTED" --regex
```

Rename file paths using the same mappings:

```sh
gk rewrite -m OldBrand:NewBrand --rename-files
```

Delete a path or glob across history:

```sh
gk rewrite --delete-path "path/to/file.txt"
gk rewrite --delete-path "assets/**/*.png"
```

### `gk github-emails --token <token>`

Use the GitHub API to find emails you have used in commits and PRs.

```sh
gk github-emails --token YOUR_GITHUB_TOKEN
```

### `gk verify-rewrite`

Compare rewrite output against `git-filter-repo`, with optional blob-only checks against BFG.

```sh
gk verify-rewrite --ci --with-blob
```

### `gk fast-export` and `gk fast-import`

Stream fast-export data out of a repo, or import a fast-import stream into a repo.

```sh
# Export to a file
gk fast-export --repo /path/to/repo --output export.fw

# Import from stdin
cat export.fw | gk fast-import --repo /path/to/new/repo
```

## Rewrite workflow (recommended)

1. **Audit identities:** use `gk check` or `gk report` to inventory names/emails.
2. **Plan mappings:** define `-m old:new` mappings and any `--delete-path` rules.
3. **Rewrite:** run `gk rewrite` from inside a repo or a parent directory of repos.
4. **Verify:** use `gk verify-rewrite` for parity checks if needed.
5. **Push:** force-push rewritten history after inspection.

## Mapping rules

- `-m old:new` replaces literal `old` with `new` in text blobs.
- Escape literal colons with `\:` (example: `foo\:bar:baz`).
- `--regex` treats the left side as a Rust regex.
- `--ignore-case` and `--preserve-case` control matching and replacement casing.
- `--rename-files` applies the same mappings to file paths.
- `-x` excludes file globs from content and path rewriting.
- `--delete-path` deletes matching paths across all history (glob supported).

## Safety notes

- Rewrites modify history. Use a clean working tree and coordinate with collaborators.
- After a rewrite, force-push branches and tags:

```sh
git push --force --tags origin main
```

## Environment variables

- `GITKAT_RELEASE_BASE`: override the GitHub Releases download base URL.
- `GITKAT_CACHE_DIR`: override the cache directory used by the pip wrapper.
- `BFG_JAR`: path to the BFG jar when running `gk verify-rewrite --with-bfg`.

## Development

```sh
gh workflow run tests.yml
gh run watch --workflow tests.yml --exit-status
cargo run -p gitkat -- --help
```

## License

MIT License. See `LICENSE`.
