# Verification

GitKat can validate rewrite parity against other tools using `gk verify-rewrite`. This is a regression safety net when changing rewrite behavior or comparing to established tools.

## What gets compared

- **`git-filter-repo`:** identity + blob rewrites (fast-export hash comparison).
- **BFG Repo-Cleaner:** blob-only rewrites (token presence/absence checks).

## Requirements

- `git-filter-repo` must be on your `PATH`.
- BFG requires Java 17+ and the BFG jar (set `BFG_JAR` or use `--bfg-jar`).
- Network access is required to clone verification repos unless you pass local repos explicitly.

## Examples

```sh
# Identity + blob parity with git-filter-repo
gk verify-rewrite --ci --with-blob

# Include regex mapping verification
gk verify-rewrite --ci --with-blob --with-regex

# Include BFG blob-only verification
gk verify-rewrite --ci --with-blob --with-bfg --bfg-jar /path/to/bfg.jar
```

## Options

- `--ci`: use a smaller repo set suitable for CI.
- `--with-blob`: include blob rewrite checks.
- `--with-regex`: include regex mapping verification.
- `--with-bfg`: compare blob-only rewrites against BFG.
- `--bfg-jar`: explicit path to the BFG jar.
- `--workdir`: custom working directory for clones.
- `--keep-workdir`: keep the working directory for inspection.

## Custom repos

Pass explicit repo URLs or local paths as positional arguments:

```sh
gk verify-rewrite --with-blob https://github.com/owner/repo.git
```

## How it works

1. Clone a repo into a temporary working directory.
2. Run GitKat rewrite on one clone.
3. Run `git-filter-repo` with equivalent callbacks on another clone.
4. Compare fast-export hashes for parity.
5. If `--with-bfg` is enabled, run BFG on a mirror clone and compare blob replacement results.

## Notes

- `--with-regex` and `--with-bfg` imply a blob rewrite sample.
- BFG verification is blob-only and does not compare identity rewrites.
- Verification does not modify your original repositories.
