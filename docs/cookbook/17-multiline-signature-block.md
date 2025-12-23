# Remove a multiline signature block

<div class="cookbook-meta">
  <span class="level-badge level-advanced">Advanced</span>
  <code>Repo: google/googletest</code>
  <code>Size: medium</code>
</div>

A template signature block was copied into many files, including a literal colon. Remove it everywhere.

## Goal

Use regex mapping to delete a multiline block that includes colons.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/google/googletest.git
cd googletest

# Replace the multiline block with a redacted tag

gk rewrite --regex \
  -m $'By\\: Shayan Amani\\n\\nFeel free to PR\\. \\:heart_eyes\\::[redacted]'
```

## Notes

- Escape literal colons in mappings with `\:` so they are not treated as separators.
- Use $'..' to embed newlines and backslashes cleanly in shell.
