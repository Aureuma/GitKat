# Preserve case while rebranding documentation

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: serde-rs/serde</code>
  <code>Size: large</code>
</div>

A brand rename appears in multiple casings. You want to keep the casing of existing text.

## Goal

Replace text while preserving case of the original matches.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/serde-rs/serde.git
cd serde

gk rewrite -m "OldBrand:NewBrand" --ignore-case --preserve-case
```

## Notes

- `--ignore-case` matches every casing.
- `--preserve-case` mirrors the casing of each match.
- Combine with `--rename-files` if you also want path changes.
