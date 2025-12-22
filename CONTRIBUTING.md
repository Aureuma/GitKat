# Contributing

Thanks for helping improve GitKat.

## Local setup

```sh
uv venv
uv pip install -e . --group dev
```

`gk rewrite` builds a Rust helper, so install a Rust toolchain (`cargo`) if you want to run rewrite locally.

## Running tests

```sh
uv run pytest
```

The rewrite equivalence test compares against `git-filter-repo`. Install it if you want that verification step to run.

## Linting

```sh
uv run ruff check src tests
```

## Docs

```sh
uv run mkdocs serve
```

## Pull requests

- Keep changes focused and describe why they are needed.
- Update documentation and tests when behavior changes.
- Add changelog entries for user-facing changes.
