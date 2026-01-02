# Consolidate identities in a popular library

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: psf/requests</code>
  <code>Size: large</code>
</div>

You have committed with multiple emails and want a single consistent identity.

## Goal

Rewrite author/committer metadata for a set of old emails.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/psf/requests.git
cd requests

# Rewrite multiple old emails to one new identity
gk rewrite \
  -n "New Name" \
  -e "new@example.test" \
  -o "old1@example.test" \
  -o "old2@example.test" \
  -o "old3@example.test"
```

## Notes

- `-o` is repeatable and can also accept comma-separated lists.
- This updates both author and committer fields.
- Commit messages remain unchanged.
