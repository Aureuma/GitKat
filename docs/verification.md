# Verification

GitKat can validate rewrite parity against other tools using `gk verify-rewrite`.

## What gets compared

- `git-filter-repo`: identity and blob rewrites (fast-export hash match).
- BFG Repo-Cleaner: blob-only rewrites (blob set match after cleanup).

## Requirements

- `git-filter-repo` must be on your `PATH`.
- BFG requires Java 17+ and the BFG jar. Set `BFG_JAR` or pass `--bfg-jar`.

## Examples

```sh
# Identity + blob parity with git-filter-repo
gk verify-rewrite --ci --with-blob

# Include regex mapping verification
gk verify-rewrite --ci --with-blob --with-regex

# Include BFG blob-only verification
gk verify-rewrite --ci --with-blob --with-bfg --bfg-jar /path/to/bfg.jar
```

## Notes

- `--with-regex` and `--with-bfg` imply a blob sample rewrite.
- BFG verification skips identity rewrites and focuses on blob contents only.
