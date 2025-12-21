import runpy
import sys
from pathlib import Path

import pytest

from gitkat import cli


def test_cli_requires_command():
    with pytest.raises(SystemExit):
        cli.main([])


def test_cli_check_invokes_run(monkeypatch):
    captured = {}

    def fake_run(name, base_dir):
        captured["name"] = name
        captured["base_dir"] = base_dir
        return 0

    monkeypatch.setattr(cli.check, "run", fake_run)
    assert cli.main(["check", "Example"]) == 0
    assert captured["name"] == "Example"
    assert captured["base_dir"] == Path.cwd()


def test_cli_rewrite_invokes_run(monkeypatch):
    captured = {}

    def fake_run(opts, base_dir):
        captured["opts"] = opts
        captured["base_dir"] = base_dir
        return 0

    monkeypatch.setattr(cli.rewrite, "run", fake_run)
    args = [
        "rewrite",
        "-n",
        "New",
        "-e",
        "new@example.test",
        "-o",
        "old@example.test",
        "-m",
        "old:new",
        "-x",
        "data/*.csv",
        "--ignore-case",
        "--preserve-case",
        "--rename-files",
    ]
    assert cli.main(args) == 0
    opts = captured["opts"]
    assert opts.new_name == "New"
    assert opts.new_email == "new@example.test"
    assert opts.old_emails == ["old@example.test"]
    assert opts.blob_map == ["old:new"]
    assert opts.exclude_patterns == ["data/*.csv"]
    assert opts.ignore_case is True
    assert opts.preserve_case is True
    assert opts.rename_files is True


def test_cli_github_emails(monkeypatch):
    captured = {}

    def fake_run(token):
        captured["token"] = token
        return 0

    monkeypatch.setattr(cli.github_emails, "run", fake_run)
    assert cli.main(["github-emails", "--token", "token123"]) == 0
    assert captured["token"] == "token123"


def test_module_entrypoint(monkeypatch):
    def fake_run(name, base_dir):
        return 0

    monkeypatch.setattr(cli.check, "run", fake_run)
    monkeypatch.setattr(sys, "argv", ["gitkat", "check", "Example"])
    with pytest.raises(SystemExit) as exc:
        runpy.run_module("gitkat.__main__", run_name="__main__")
    assert exc.value.code == 0
