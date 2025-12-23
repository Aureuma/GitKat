# Replace private Git URLs

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: Homebrew/brew</code>
  <code>Size: medium</code>
</div>

Some docs still reference a private Git server. You need to replace the URLs across history.

## Goal

Rewrite internal Git URLs to their public equivalents.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/Homebrew/brew.git
cd brew

# Replace private Git URLs across text

gk rewrite \
  -m "git@corp.example.com:git@github.com" \
  -m "https://corp.example.com/:https://github.com/" \
  --ignore-case
```

## Notes

- Keep mappings narrowly scoped to avoid unintentional URL changes.
- Use `--regex` if your patterns need wildcards or anchors.
