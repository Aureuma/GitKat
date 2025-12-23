# Consolidate one author identity

<div class="cookbook-meta">
  <span class="level-badge level-easy">Easy</span>
  <code>Repo: psf/requests</code>
  <code>Size: medium</code>
</div>

A contributor used multiple emails. You want all commits to use one canonical identity.

## Goal

Rewrite author and committer metadata for a single email.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/psf/requests.git
cd requests

# Review recent commits

git log -n 20 --format="%an <%ae>"

# Pick an email from history and rewrite to a canonical identity

OLD_EMAIL=$(git log -n 1 --format="%ae")

gk rewrite -o "$OLD_EMAIL" -e "canonical@example.com" -n "Canonical Name"
```

## Notes

- `gk rewrite` updates author and committer signatures across history.
- Commit messages are preserved.
