# Remove macOS metadata files

<div class="cookbook-meta">
  <span class="level-badge level-easy">Easy</span>
  <code>Repo: facebook/react</code>
  <code>Size: large</code>
</div>

macOS metadata files slipped into the repo over time. Remove them across all commits.

## Goal

Delete .DS_Store and .idea files from history.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/facebook/react.git
cd react

# Remove common editor metadata files

gk rewrite \
  --delete-path "**/.DS_Store" \
  --delete-path "**/.idea/**"
```

## Notes

- Add more glob entries for editor or OS artifacts that should never be versioned.
