# Contributing

Thanks for helping improve GitKat.

## Local setup

```sh
cargo build --workspace
```

## Running tests

```sh
cargo test --workspace
```

The rewrite equivalence test compares against `git-filter-repo`. Install it if you want that verification step to run.

## Verification tools

`gk verify-rewrite` can compare against external tooling to validate parity:

- `git-filter-repo` for identity + blob equivalence checks.
- BFG Repo-Cleaner (plus Java) for blob-only rewrite comparisons. Provide `--with-bfg` and set `BFG_JAR` or pass `--bfg-jar`.

Example:

```sh
gk verify-rewrite --ci --with-blob --with-regex
gk verify-rewrite --ci --with-blob --with-bfg --bfg-jar /path/to/bfg.jar
```

## Linting

```sh
cargo fmt --check
```

## Docs

```sh
mdbook build
```

## Pull requests

- Keep changes focused and describe why they are needed.
- Update documentation and tests when behavior changes.
- Add changelog entries for user-facing changes.
