# Release 0.5.0

GitKat is now Rust-only. The CLI and rewrite engine run entirely in Rust (gitoxide/gix), and the Python packaging/runtime has been removed.

Highlights:
- Rust-only `gk` CLI with the same commands as before.
- Rewrite engine implemented in-process via gitoxide.
- Verification harness that compares rewrites against `git-filter-repo`.
- Updated CI for Rust builds and rewrite verification.

Upgrade notes:
- Install/build with `cargo install --path .` or `cargo build --release`.
- `gk rewrite` usage and flags are unchanged.
