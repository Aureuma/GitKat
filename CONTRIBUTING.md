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

## Linting

```sh
cargo fmt --check
```

## Pull requests

- Keep changes focused and describe why they are needed.
- Update documentation and tests when behavior changes.
- Add changelog entries for user-facing changes.
