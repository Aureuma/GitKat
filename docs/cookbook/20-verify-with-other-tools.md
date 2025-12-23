# Verify rewrites against other tools

<div class="cookbook-meta">
  <span class="level-badge level-advanced">Advanced</span>
  <code>Repo: multiple (CI fixtures)</code>
  <code>Size: small</code>
</div>

Before shipping a rewrite workflow, compare GitKat output to other history rewrite tools.

## Goal

Run GitKat's built-in verification suite against git-filter-repo (and optionally BFG).

## Steps

```sh
# This command clones several known repos and compares outputs.
# Requires Python (git-filter-repo) and Java if you enable BFG.

gk verify-rewrite --ci --with-blob --with-regex

# Optional: add BFG comparison if you have Java and bfg.jar available.
# export BFG_JAR=/path/to/bfg.jar
# gk verify-rewrite --ci --with-blob --with-regex --with-bfg
```

## Notes

- The verify command uses its own fixture repositories and does not touch your current repo.
- This is the same verification strategy used in the GitHub Actions workflow.
