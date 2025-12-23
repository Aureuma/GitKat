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

𝔾𝚒𝚝𝕂𝚊𝚝 ⫷⫸ (GitKat) is a Rust CLI for bulk Git repository maintenance. It keeps the behavior of the original shell tooling while adding packaged, testable workflows.

## Highlights

- Search commit metadata across many repos.
- List author emails for auditing.
- Force-push current branches in bulk.
- Rewrite history with a Rust gitoxide (gix) rewriter, including case-preserving blob replacements.
- Query GitHub contribution emails via API.
- Delete paths (including globs) across history with colored rewrite logs.

## Quick start

```sh
gk check "Example Name"
gk report .
gk push
gk rewrite -m olddomain.com:newdomain.com --ignore-case --preserve-case
gk github-emails --token YOUR_GITHUB_TOKEN
```

## Install

```sh
# crates.io
cargo install gitkat

# Homebrew (tap this repo)
brew install Aureuma/gitkat/gitkat

# npm
npm install -g @aureuma/gitkat

# pip (Python wrapper downloads the Rust binary)
python -m pip install gitkat
```

## Safety

- Rewrites change history. Run from a clean working tree and review results before pushing.
- After a rewrite, force-push branches and tags: `git push --force --tags origin main`.
