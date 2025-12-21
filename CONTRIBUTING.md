# Contributing

Thanks for helping improve GitKat.

## Local setup

```sh
uv venv
uv pip install -e . --group dev
```

## Running tests

```sh
uv run pytest
```

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
