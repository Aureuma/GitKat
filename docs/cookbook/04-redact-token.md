# Redact a leaked token in documentation

<div class="cookbook-meta">
  <span class="level-badge level-easy">Easy</span>
  <code>Repo: github/gitignore</code>
  <code>Size: medium</code>
</div>

A token shows up in a README. You want to replace it everywhere in history.

## Goal

Replace a string inside text blobs with a redacted version.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/github/gitignore.git
cd gitignore

# Choose a token from the first line of README.md

TOKEN=$(git show HEAD:README.md | head -n 1 | rg -o "[A-Za-z][A-Za-z0-9_-]+" | head -n 1)

gk rewrite -m "${TOKEN}:${TOKEN}_REDACTED"
```

## Notes

- This rewrites blob contents in text files only.
- The example uses a token found in README.md so the replacement is deterministic.
