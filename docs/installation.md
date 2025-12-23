# Installation

## Requirements

- Git
- Rust toolchain (cargo) for building from source
- `git-filter-repo` (optional, for `gk verify-rewrite`)

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

## Binary releases

GitHub Releases publish prebuilt binaries for:

- macOS (x86_64, arm64)
- Linux (x86_64, arm64)
- Windows (x86_64)

If you install via pip or npm, the wrapper downloads the matching binary into `~/.cache/gitkat/<version>` and executes it.

## Environment variables

- `GITKAT_RELEASE_BASE`: Override the GitHub Releases download base URL.
- `GITKAT_CACHE_DIR`: Override the cache directory used by the pip wrapper.
