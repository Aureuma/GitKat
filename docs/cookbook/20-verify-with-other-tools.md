# Verify rewrites against other tools

<div class="cookbook-meta">
  <span class="level-badge level-advanced">Advanced</span>
  <code>Repo: any</code>
  <code>Size: any</code>
</div>

You want to confirm GitKat produces the same output as other rewrite tools.

## Goal

Run `gk verify-rewrite` with optional BFG checks.

## Steps

```sh
# Compare identity + blob rewrites to git-filter-repo
gk verify-rewrite --ci --with-blob

# Include regex mapping checks
gk verify-rewrite --ci --with-blob --with-regex

# Compare blob-only rewrite to BFG
gk verify-rewrite --ci --with-blob --with-bfg --bfg-jar /path/to/bfg.jar
```

## Notes

- `git-filter-repo` must be on your `PATH`.
- BFG requires Java 17+ and the jar file.
- Verification runs in temporary clones and does not modify your original repos.
