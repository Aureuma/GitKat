# Remove .env secrets from history

<div class="cookbook-meta">
  <span class="level-badge level-easy">Easy</span>
  <code>Repo: fastify/fastify</code>
  <code>Size: medium</code>
</div>

A contributor committed local environment files. You want to remove them entirely from history.

## Goal

Delete .env files everywhere in the tree so they never existed in Git history.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/fastify/fastify.git
cd fastify

# Confirm whether any env files exist in tracked history

git ls-files | grep -E '\\.env($|\\.)' || true

# Remove env files from all commits

gk rewrite \
  --delete-path ".env" \
  --delete-path ".env.local" \
  --delete-path "**/.env" \
  --delete-path "**/.env.*"
```

## Notes

- The delete paths accept globs, so you can target nested files too.
- If nothing matches, GitKat will report zero deletions but still complete.
