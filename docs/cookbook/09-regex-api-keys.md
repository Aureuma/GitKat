# Scrub API keys with regex

<div class="cookbook-meta">
  <span class="level-badge level-advanced">Advanced</span>
  <code>Repo: any</code>
  <code>Size: large</code>
</div>

You need to redact API keys that follow a predictable pattern.

## Goal

Use regex mappings to replace keys across all history.

## Steps

```sh
# Example pattern: api_key=XXXXXXXX
gk rewrite -m "api_key=[A-Za-z0-9_-]+:api_key=REDACTED" --regex
```

## Notes

- Regex syntax follows Rust regex (similar to RE2).
- Replacement is literal text; capture groups are not expanded.
- Add `--ignore-case` if keys use inconsistent casing.
