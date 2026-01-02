# Normalize image extensions

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: any</code>
  <code>Size: medium</code>
</div>

You want to normalize binary file extensions (for example, `.jpeg` to `.jpg`).

## Goal

Rename file paths across history without touching file contents.

## Steps

```sh
# Rename .jpeg to .jpg across history
gk rewrite -m ".jpeg:.jpg" --rename-files
```

## Notes

- This only changes paths, not blob contents.
- If both `.jpeg` and `.jpg` exist in the same directory, you may get conflicts.
