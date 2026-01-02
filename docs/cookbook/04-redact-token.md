# Redact a leaked token in documentation

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
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

Verify the token is gone:

```sh
rg -n "${TOKEN}" -S
```

## Notes

- Blob rewrites only touch text files; binary blobs are skipped.
- Use `--ignore-case` and `--preserve-case` if casing is inconsistent.
