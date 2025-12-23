# Remove CSV datasets from history

<div class="cookbook-meta">
  <span class="level-badge level-easy">Easy</span>
  <code>Repo: seaborn/seaborn</code>
  <code>Size: medium</code>
</div>

Data files were accidentally checked in. You want to remove CSV datasets across all commits.

## Goal

Delete CSV blobs from every revision.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/mwaskom/seaborn.git
cd seaborn

# Find any tracked CSV files

git ls-files | grep -E '\\.csv$' | head -n 20 || true

# Remove all CSV files from history

gk rewrite --delete-path "**/*.csv"
```

## Notes

- Use a narrower glob if you need to keep some datasets.
- Combine with -x to exclude directories that should stay untouched.
