"""Module entrypoint for python -m gitkit."""

from .cli import main


if __name__ == "__main__":
    raise SystemExit(main())
