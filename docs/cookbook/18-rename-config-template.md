# Rename config templates across history

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: denoland/deno</code>
  <code>Size: large</code>
</div>

You want every historical reference to point to a new config template name.

## Goal

Rename a config template file and update references everywhere.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/denoland/deno.git
cd deno

# Rename a template file name (adjust to match your repo)

gk rewrite --rename-files -m "example.env:sample.env" --ignore-case
```

## Notes

- The same mapping updates file content, so references stay in sync.
- For large repos, add -x excludes for vendor or build directories.
