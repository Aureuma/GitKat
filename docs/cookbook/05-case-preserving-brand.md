# Preserve case while rebranding documentation

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: rust-lang/rustlings</code>
  <code>Size: medium</code>
</div>

You are running a workshop fork and need to rename the product while keeping the original casing (RUSTLINGS, Rustlings, rustlings).

## Goal

Replace every case variant of a name while preserving the original casing style.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/rust-lang/rustlings.git
cd rustlings

# See where the word appears before changing history

git grep -n "rustlings" | head -n 20

# Rename with case preservation

gk rewrite -m "rustlings:rustcamp" --ignore-case --preserve-case
```

## Notes

- Keep the replacement in lowercase; `--preserve-case` will adapt it to the matched case.
- Use `-x` to skip large directories if the repo is heavy.
