# Remove macOS metadata files

<div class="cookbook-meta">
  <span class="level-badge level-easy">Easy</span>
  <code>Repo: any</code>
  <code>Size: any</code>
</div>

`.DS_Store` files are noise and should not appear in history.

## Goal

Delete `.DS_Store` files across all commits.

## Steps

```sh
gk rewrite --delete-path "**/.DS_Store"
```

## Notes

- Add `.DS_Store` to `.gitignore` after rewriting.
