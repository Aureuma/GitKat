# Installation

## Requirements

- Git
- Rust toolchain (cargo)
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
