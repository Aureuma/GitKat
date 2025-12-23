# Contributing

Please see `CONTRIBUTING.md` for local setup, coding standards, and the pull request process.

Tests and verification run in GitHub Actions to keep CI parity. Local test runs are intentionally avoided.

Verification tools used by contributors:

- `git-filter-repo` validates rewrite equivalence.
- BFG Repo-Cleaner (plus Java) validates blob-only rewrites when running `gk verify-rewrite --with-bfg`.
