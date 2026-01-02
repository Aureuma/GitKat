# Remove .env secrets from history

<div class="cookbook-meta">
  <span class="level-badge level-easy">Easy</span>
  <code>Repo: any</code>
  <code>Size: any</code>
</div>

Environment files often contain secrets and should be removed from history.

## Goal

Delete `.env` files across all history.

## Steps

```sh
# Delete specific .env files
gk rewrite --delete-path ".env"

gk rewrite --delete-path "config/.env"
```

For multiple .env variants:

```sh
gk rewrite --delete-path "**/.env" --delete-path "**/.env.*"
```

## Notes

- Delete patterns use glob matching.
- After removal, rotate any credentials that were exposed.
