# Rename docs paths across history

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: any</code>
  <code>Size: medium</code>
</div>

You want to rename a documentation directory and update references across history.

## Goal

Rename file paths and update textual references.

## Steps

```sh
# Rename docs/ to handbook/ and update text references
gk rewrite \
  -m "docs/:handbook/" \
  --rename-files
```

## Notes

- `--rename-files` applies mappings to paths.
- Without blob mappings, only paths change.
- Add additional `-m` entries for related references if needed.
