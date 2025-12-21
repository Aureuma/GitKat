"""Command-line entrypoint for GitKat."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Iterable, Optional

from .commands import check, github_emails, push, report, rewrite
from .commands.rewrite import RewriteOptions


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="gk",
        description="GitKat: bulk Git repository utilities.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    check_parser = subparsers.add_parser("check", help="Search for author/committer names.")
    check_parser.add_argument("name", help="Name to search for in author/committer fields.")
    check_parser.set_defaults(func=lambda args: check.run(args.name, Path.cwd()))

    report_parser = subparsers.add_parser("report", help="List unique author emails per repo.")
    report_parser.add_argument("path", nargs="?", default=".", help="Base directory to scan.")
    report_parser.set_defaults(func=lambda args: report.run(Path(args.path)))

    push_parser = subparsers.add_parser("push", help="Force-push current branches.")
    push_parser.set_defaults(func=lambda args: push.run(Path.cwd()))

    rewrite_parser = subparsers.add_parser("rewrite", help="Rewrite history across repos.")
    rewrite_parser.add_argument("-n", dest="new_name", default="", help="New author/committer name.")
    rewrite_parser.add_argument("-e", dest="new_email", default="", help="New author/committer email.")
    rewrite_parser.add_argument("-o", dest="old_emails", action="append", help="Old emails to match (comma-separated).")
    rewrite_parser.add_argument("-O", dest="old_name", default="", help="Old author/committer name to require.")
    rewrite_parser.add_argument("-m", dest="blob_map", action="append", help="Blob mapping old:new (repeatable).")
    rewrite_parser.add_argument("-x", dest="exclude_patterns", action="append", help="Exclude file globs from blob rewrites.")
    rewrite_parser.add_argument("--rename-files", action="store_true", help="Apply mappings to file paths.")
    rewrite_parser.add_argument("--preserve-case", action="store_true", help="Preserve casing for replacements.")
    rewrite_parser.add_argument("--ignore-case", "-i", action="store_true", help="Match blob replacements case-insensitively.")
    rewrite_parser.set_defaults(
        func=lambda args: rewrite.run(
            RewriteOptions(
                new_name=args.new_name,
                new_email=args.new_email,
                old_name=args.old_name,
                old_emails=args.old_emails,
                blob_map=args.blob_map,
                exclude_patterns=args.exclude_patterns,
                preserve_case=args.preserve_case,
                ignore_case=args.ignore_case,
                rename_files=args.rename_files,
            ),
            Path.cwd(),
        )
    )

    emails_parser = subparsers.add_parser("github-emails", help="Find GitHub contribution emails.")
    emails_parser.add_argument("--token", help="GitHub personal access token.")
    emails_parser.set_defaults(func=lambda args: github_emails.run(args.token))
    return parser


def main(argv: Optional[Iterable[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(list(argv) if argv is not None else None)
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
