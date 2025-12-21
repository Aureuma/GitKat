"""Command-line entrypoint for GitKit."""

from __future__ import annotations

import argparse
from typing import Iterable, Optional


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="gk",
        description="GitKit: bulk Git repository utilities.",
    )
    subparsers = parser.add_subparsers(dest="command")
    for name in ("check", "report", "push", "rewrite", "github-emails"):
        subparsers.add_parser(name)
    return parser


def main(argv: Optional[Iterable[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(list(argv) if argv is not None else None)
    if not args.command:
        parser.print_help()
        return 0

    parser.error("CLI wiring pending in this build.")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
