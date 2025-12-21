# Installation

## Requirements

- Python 3.9+
- Git
- git-filter-repo (installed via package dependencies)

## Using uv

```sh
uv venv
uv pip install -e .
uv pip install -e . --group dev
```

## Using pip

```sh
python -m venv .venv
. .venv/bin/activate
pip install -e .
```
