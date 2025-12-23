# Replace internal hostnames

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: hashicorp/terraform</code>
  <code>Size: large</code>
</div>

Docs and configs contain an internal hostname that should never ship. Replace it everywhere.

## Goal

Rewrite an internal hostname into a public one across history.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone --depth 200 https://github.com/hashicorp/terraform.git
cd terraform

# Replace internal hostnames across text

gk rewrite -m "corp.internal:example.com" --ignore-case
```

## Notes

- Add more mappings if multiple internal domains exist.
- For full coverage, re-clone without --depth.
