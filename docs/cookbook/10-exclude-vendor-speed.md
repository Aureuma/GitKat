# Exclude vendor directories for a faster rewrite

<div class="cookbook-meta">
  <span class="level-badge level-easy">Easy</span>
  <code>Repo: any</code>
  <code>Size: large</code>
</div>

Large vendor directories slow blob rewrites because they contain many files.

## Goal

Exclude heavy directories while rewriting the rest of the repo.

## Steps

```sh
gk rewrite \
  -m olddomain.com:newdomain.com \
  -x "vendor/**" \
  -x "third_party/**"
```

## Notes

- Exclude globs skip both content rewrites and path renames.
- Deletes still apply if the path also matches `--delete-path`.
