# Installation

## Requirements

- Git
- A supported OS/arch (macOS, Linux, Windows)
- Rust toolchain (only if building from source)
- Optional for verification:
  - `git-filter-repo` on your `PATH`
  - BFG Repo-Cleaner + Java 17+

## Install via package managers

```sh
# crates.io
cargo install gitkat --locked

# Homebrew
brew tap Aureuma/gitkat
brew install gitkat

# pnpm
corepack pnpm install -g @aureuma/gitkat

# pipx (recommended)
pipx install gitkat

# pip
python -m pip install gitkat
```

## Install from GitHub Releases

Download the binary for your OS/arch from:

https://github.com/Aureuma/GitKat/releases

Unpack it and place `gk` on your `PATH`.

## Build from source

```sh
cargo build --release -p gitkat
./target/release/gk --help
```

## Verify the install

```sh
gk --help
gk --version
```

## Optional verification tools

`gk verify-rewrite` compares GitKat output to other tools.

```sh
# Install git-filter-repo (Python)
python -m pip install --upgrade git-filter-repo

# Download BFG (jar) and set BFG_JAR
export BFG_JAR=/path/to/bfg.jar
```

## Environment variables

- `GITKAT_RELEASE_BASE`: override the GitHub Releases download base URL.
- `GITKAT_CACHE_DIR`: override the cache directory used by the pip wrapper.
- `BFG_JAR`: path to the BFG jar for `gk verify-rewrite --with-bfg`.
