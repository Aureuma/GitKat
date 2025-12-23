# Rename docs paths across history

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: cli/cli</code>
  <code>Size: medium</code>
</div>

You are reorganizing documentation and want all historic paths updated to match the new layout.

## Goal

Rename file paths and update references in text in one pass.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/cli/cli.git
cd cli

# Preview existing doc paths

git ls-files docs | head -n 20

# Rename a docs directory (adjust the mapping to your repo)

gk rewrite \
  --rename-files \
  -m "docs/changelog:docs/releases" \
  -m "docs/changelog.md:docs/releases.md"
```

## Notes

- `--rename-files` applies the same mappings to file paths and file contents.
- Keep mappings specific to avoid renaming unrelated paths.
