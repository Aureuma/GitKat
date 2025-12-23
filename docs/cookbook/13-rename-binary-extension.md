# Normalize image extensions

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: simple-icons/simple-icons</code>
  <code>Size: medium</code>
</div>

Design assets came in with mixed extensions. You want to standardize .jpeg to .jpg and update references.

## Goal

Rename files and update text references in one rewrite.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/simple-icons/simple-icons.git
cd simple-icons

# See if any jpeg references exist

git grep -n "\\.jpeg" | head -n 20 || true

# Rename paths and update content

gk rewrite --rename-files -m ".jpeg:.jpg" --ignore-case
```

## Notes

- `--rename-files` changes file paths and blob text at the same time.
- Keep the mapping short so it only touches the extension.
