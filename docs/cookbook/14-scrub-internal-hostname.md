# Replace internal hostnames

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: any</code>
  <code>Size: medium</code>
</div>

Docs or configs reference internal hostnames that should be removed.

## Goal

Replace internal hostnames across text blobs.

## Steps

```sh
# Replace a private hostname with a public placeholder
gk rewrite -m "internal.company.local:example.invalid" --ignore-case
```

## Notes

- Use `--regex` if the hostname has variants you need to match.
- Consider using `--preserve-case` if casing varies.
