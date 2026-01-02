# Contributing

Thanks for helping improve GitKat. This doc summarizes contributor workflows; see `CONTRIBUTING.md` at the repository root for complete guidance.

## Quick setup

```sh
cargo build -p gitkat
cargo test --workspace --all-targets
```

## Development workflow

- Keep changes small and focused.
- Prefer adding tests when changing rewrite logic or verification behavior.
- Use `gk verify-rewrite` sparingly locally; CI is the source of truth.

## CI strategy

- `Tests` runs linting, unit tests, and documentation builds.
- `Verify` compares GitKat output against `git-filter-repo` and optionally BFG.
- CI is the authoritative validation for rewrite parity.

## Verification tools

- `git-filter-repo` validates rewrite equivalence.
- BFG Repo-Cleaner (with Java) validates blob-only rewrites.

## Code style

- Keep CLI output consistent and human-readable.
- Avoid breaking changes to rewrite semantics without updating docs and verification.
- Keep docs and README in sync with behavior changes.
