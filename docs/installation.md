# Installation

## Requirements

- Git
- Rust toolchain (cargo) for building from source
- `git-filter-repo` (optional, for `gk verify-rewrite`)
- BFG Repo-Cleaner + Java (optional, for `gk verify-rewrite --with-bfg`)

## Package managers

```sh
# crates.io (Rust)
cargo install gitkat --locked

# Homebrew
brew tap Aureuma/gitkat
brew install gitkat

# npm
npm install -g @aureuma/gitkat

# pipx (recommended for CLI tools)
pipx install gitkat

# pip
python -m pip install gitkat
```

## GitHub Releases

Download the binary that matches your OS/arch from:

https://github.com/Aureuma/GitKat/releases

Extract it and place `gk` on your `PATH`.

## Build from source

```sh
cargo build --release -p gitkat
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
- `BFG_JAR`: Path to the BFG jar when running `gk verify-rewrite --with-bfg`.
