# Purge build artifacts from a monorepo

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: tailwindlabs/tailwindcss</code>
  <code>Size: large</code>
</div>

A monorepo accidentally committed build output. You want to drop all of it from history.

## Goal

Delete build output directories like dist/ and build/ across every commit.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone --depth 200 https://github.com/tailwindlabs/tailwindcss.git
cd tailwindcss

# Optional: see how many build artifacts are tracked

git ls-files | grep -E '/(dist|build)/' | head -n 20 || true

# Remove build artifacts from history

gk rewrite \
  --delete-path "**/dist/**" \
  --delete-path "**/build/**" \
  --delete-path "**/*.map"
```

## Notes

- For a full rewrite, clone without --depth so all commits are included.
- Use multiple delete paths to keep the intent explicit.
