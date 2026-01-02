# Rename config templates across history

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: any</code>
  <code>Size: medium</code>
</div>

A repo uses inconsistent config template names (for example, `.env.example`).

## Goal

Rename template files across history and update references.

## Steps

```sh
# Rename .env.example to .env.template
gk rewrite -m ".env.example:.env.template" --rename-files
```

## Notes

- Use `-m` mappings to update references in documentation as well.
- If you only want to rename paths, do not include other mappings.
