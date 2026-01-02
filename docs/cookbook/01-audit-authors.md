# Audit authors before a rewrite

<div class="cookbook-meta">
  <span class="level-badge level-easy">Easy</span>
  <code>Repo: any</code>
  <code>Size: any</code>
</div>

You are about to rewrite history and need to know which names and emails appear in commits.

## Goal

Inventory author identities so you can build accurate rewrite mappings.

## Steps

```sh
# From a directory containing multiple repos
gk report .

gk check "Jane Developer"
```

## Notes

- `gk report` lists unique author emails per repo.
- `gk check` searches both author and committer fields.
- Use the results to decide which emails to include with `-o` in `gk rewrite`.
