# Remove CSV datasets from history

<div class="cookbook-meta">
  <span class="level-badge level-easy">Easy</span>
  <code>Repo: any</code>
  <code>Size: large</code>
</div>

Large CSV datasets can bloat history and should be removed.

## Goal

Delete CSV files across history.

## Steps

```sh
gk rewrite --delete-path "**/*.csv"
```

To target a specific dataset directory:

```sh
gk rewrite --delete-path "data/archive/**"
```

## Notes

- Use more specific globs if you need to preserve certain CSVs.
- Add patterns to `.gitignore` to prevent reintroduction.
