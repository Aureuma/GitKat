<p align="center">
  <a href="https://github.com/Aureuma/GitKat">
    <img src="assets/logo.svg" alt="⫷⫸" width="96" height="96">
  </a>
</p>

<p align="center">
  <a href="https://github.com/Aureuma/GitKat">
    <img alt="GitHub" src="https://img.shields.io/badge/GitHub-Aureuma/GitKat-181717?logo=github&logoColor=white">
  </a>
</p>

# 𝔾𝚒𝚝𝕂𝚊𝚝 ⫷⫸

GitKat is a Rust CLI for bulk Git repository maintenance. It focuses on repeatable history rewrites and identity audits across many repos while keeping the workflow simple: inspect, map, rewrite, verify, and push.

## Highlights

- Audit commit metadata across many repositories.
- Inventory author emails to prepare identity rewrites.
- Rewrite history with a Rust gitoxide (gix) engine.
- Rename paths and delete files across all history.
- Verify parity against `git-filter-repo` and optionally BFG.

<div class="cookbook-callout">
  <strong>Cookbook:</strong> <a href="cookbook/index.md">Hands-on recipes</a> with real repos, explicit goals, and copy/paste commands.
</div>

## Quick start

```sh
# Audit authors across repos
gk check "Example Name"
gk report .

# Rewrite content with case-preserving replacements
gk rewrite -m olddomain.com:newdomain.com --ignore-case --preserve-case

# Query GitHub contribution emails
gk github-emails --token YOUR_GITHUB_TOKEN
```

## Typical workflow

1. **Audit identities** with `gk check` or `gk report`.
2. **Plan mappings** (`-m old:new`) and any `--delete-path` rules.
3. **Rewrite** from inside a repo or from a parent directory.
4. **Verify** with `gk verify-rewrite` if you need tool parity checks.
5. **Push** rewritten history with `git push --force --tags`.

## Where to go next

- [Installation](installation.md)
- [Commands](commands.md)
- [Rewrite guide](rewrite.md)
- [Verification](verification.md)
- [Cookbook](cookbook/index.md)
