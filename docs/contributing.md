# Contributing

Please see `CONTRIBUTING.md` for local setup, coding standards, and the pull request process.

## Testing strategy (CI-only)

- CI is the source of truth for tests and verification.
- `Tests` workflow runs linting, unit tests, doc builds, and verification.
- Verification runs in a matrix: identity, blob, regex, and BFG (blob-only).
- Concurrency is capped in CI to respect public repo minutes.
- Trigger tests and proceed; do not wait for completion during development.

Verification tools used by contributors:

- `git-filter-repo` validates rewrite equivalence.
- BFG Repo-Cleaner (plus Java) validates blob-only rewrites when running `gk verify-rewrite --with-bfg`.
