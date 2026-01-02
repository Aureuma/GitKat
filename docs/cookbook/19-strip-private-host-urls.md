# Replace private Git URLs

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: any</code>
  <code>Size: medium</code>
</div>

Docs or config files reference private Git URLs that should be removed.

## Goal

Replace private URLs across text blobs.

## Steps

```sh
# Replace an internal Git hostname with a public placeholder
gk rewrite -m "git.internal.local:example.invalid" --ignore-case
```

## Notes

- Use `--regex` if URLs include variable path segments.
- Consider running a follow-up scan with `rg` to verify results.
