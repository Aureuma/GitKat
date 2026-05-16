# Release 0.5.1

GitKat is Rust-only. The CLI and rewrite engine run entirely in Rust (gitoxide/gix). This release adds installer wrappers for common package managers.

Highlights:
- Rust-only `gk` CLI with the same commands as before.
- Rewrite engine implemented in-process via gitoxide.
- Verification harness that compares rewrites against `git-filter-repo`.
- Installers for crates.io, Homebrew, pnpm, and PyPI (wrappers download the Rust binary).

Upgrade notes:
- Install/build with `cargo install gitkat` or `cargo build --release`.
- `gk rewrite` usage and flags are unchanged.
