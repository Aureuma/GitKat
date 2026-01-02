# Remove a multiline signature block

<div class="cookbook-meta">
  <span class="level-badge level-advanced">Advanced</span>
  <code>Repo: any</code>
  <code>Size: medium</code>
</div>

A multiline signature block appears in docs or config files and needs to be removed everywhere.

## Goal

Use a regex mapping that spans multiple lines.

## Steps

```sh
# Example: remove a multi-line signature block
gk rewrite -m "(?s)BEGIN SIGNATURE.*?END SIGNATURE:REDACTED" --regex
```

## Notes

- Use `(?s)` to make `.` match newlines.
- Replacement text is literal; capture groups are not expanded.
- Test on a small sample repo before rewriting production history.
