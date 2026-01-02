# Purge build artifacts from a monorepo

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: any</code>
  <code>Size: large</code>
</div>

Large build artifacts (dist, target, vendor) inflate history and slow rewrites.

## Goal

Delete build artifact directories across all history.

## Steps

```sh
gk rewrite \
  --delete-path "**/dist/**" \
  --delete-path "**/build/**" \
  --delete-path "**/target/**" \
  --delete-path "**/node_modules/**"
```

## Notes

- Use `--delete-path` for deletions and `-x` to skip scanning large directories during other rewrites.
- Consider adding these paths to `.gitignore` after the rewrite.
