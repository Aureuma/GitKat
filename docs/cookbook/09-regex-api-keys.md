# Scrub API keys with regex

<div class="cookbook-meta">
  <span class="level-badge level-advanced">Advanced</span>
  <code>Repo: axios/axios</code>
  <code>Size: medium</code>
</div>

An example project leaked API keys in fixtures. You need to redact any key-like patterns across the full history.

## Goal

Use regex-based mappings to replace secret patterns with a placeholder.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/axios/axios.git
cd axios

# Search history for a suspect token pattern

git log -p -S "AKIA" | head -n 40

# Replace likely AWS and Stripe patterns across history

gk rewrite \
  --regex \
  -m "AKIA[0-9A-Z]{16}:[redacted]" \
  -m "sk_live_[0-9a-zA-Z]{24}:[redacted]"
```

## Notes

- `--regex` treats the left side of `-m` as a regex; the replacement is literal.
- Add more patterns as needed for the secrets you expect to find.
