# Fix a maintainer's author email

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: BurntSushi/ripgrep</code>
  <code>Size: medium</code>
</div>

A maintainer changed emails and wants their old commits updated to the new address.

## Goal

Rewrite author and committer metadata without touching file contents.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/BurntSushi/ripgrep.git
cd ripgrep

# Inspect current author emails

gk report .

# Replace the old email with the new one (adjust values to match your repo)

gk rewrite -o "old@example.com" -e "new@example.com" -n "New Name"
```

## Notes

- Use the output of `gk report` to fill in the real emails.
- Identity rewrites do not modify file contents.
