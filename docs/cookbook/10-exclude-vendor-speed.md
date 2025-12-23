# Exclude vendor directories for a faster rewrite

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: nodejs/node</code>
  <code>Size: large</code>
</div>

Large repos can be heavy to rewrite. Exclude vendor-heavy folders to speed up targeted replacements.

## Goal

Rewrite a string while skipping large dependency trees.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone --depth 200 https://github.com/nodejs/node.git
cd node

# Replace a legacy URL while skipping heavy folders

gk rewrite \
  -x "deps/**" \
  -x "test/fixtures/**" \
  -m "http://nodejs.org:https://nodejs.org" \
  --ignore-case
```

## Notes

- Excludes are glob patterns that skip matching paths entirely.
- Remove --depth for a full rewrite of the entire commit graph.
