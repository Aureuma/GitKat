# Audit authors before a rewrite

<div class="cookbook-meta">
  <span class="level-badge level-easy">Easy</span>
  <code>Repo: octocat/Hello-World</code>
  <code>Size: small</code>
</div>

You inherited a tiny demo repo and want to confirm who authored the commits before rewriting anything.

## Goal

List authors and search for a specific name without touching history.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/octocat/Hello-World.git
cd Hello-World

# Review recent commits for context

git log -n 20 --format="%an <%ae>"

# Search for a name across author/committer fields

gk check "octocat"

# Inventory emails in this repo

gk report .
```

## Notes

- `gk check` and `gk report` only read history.
- Use these before choosing targets for `gk rewrite`.
