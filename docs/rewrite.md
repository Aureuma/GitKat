# Rewrite guide

`gk rewrite` is the only command that changes history. It can update author/committer metadata, rewrite text blobs, rename paths, and delete paths across all commits.

## What it does (and does not do)

**It does:**

- Rewrite author/committer identity metadata.
- Replace text content inside blobs.
- Rename file paths using the same mappings.
- Delete paths across all history using glob patterns.

**It does not:**

- Rewrite commit messages.
- Modify binary blobs (they are skipped).

## Where it runs

- Run inside a repo to rewrite that repo only.
- Run from a directory containing multiple repos to rewrite each direct child repo.

## Identity rewrites

Identity rewrites update author/committer name and email.

```sh
gk rewrite -n "New Name" -e "new@example.test" -o "old@example.test"
```

Require an old name as well (useful when multiple people share an email domain):

```sh
gk rewrite -n "New Name" -e "new@example.test" -o "old@example.test" -O "Old Name"
```

Notes:

- `-o` accepts multiple values and comma-separated lists.
- `-n` is optional; if omitted, only emails are updated.
- Matching is case-insensitive for emails and names.

## Blob rewrites

Blob mappings replace text inside blobs.

```sh
# Single mapping
gk rewrite -m olddomain.com:newdomain.com

# Multiple mappings
gk rewrite -m token:REDACTED -m api_key:REDACTED
```

Escape a literal colon inside mappings with `\:`:

```sh
gk rewrite -m "foo\:bar:baz"
```

## Regex mappings

Enable regex mode to treat the left side as a Rust regex:

```sh
gk rewrite -m "token_[0-9]+:REDACTED" --regex
```

Regex mode still treats the replacement as literal text.

## Case handling

```sh
gk rewrite -m foo:bar --ignore-case --preserve-case
```

- `--ignore-case` matches regardless of case.
- `--preserve-case` mirrors the matched casing onto the replacement.

## Excluding paths

Skip content/path rewrites for globs:

```sh
gk rewrite -m token:REDACTED -x "vendor/**" -x "data/*.csv"
```

Exclude patterns are useful for large vendor directories or generated assets.

## Renaming file paths

Use the same mappings to rename paths:

```sh
gk rewrite -m OldBrand:NewBrand --rename-files
```

This affects both file names and directory names in the tree.

## Deleting paths

Delete specific paths or globs across history:

```sh
gk rewrite --delete-path "path/to/file.txt"
gk rewrite --delete-path "assets/**/*.png"
```

Delete patterns are matched against full paths (relative to repo root).

## Logging and progress

By default, rewrite logs include:

- File paths in magenta.
- Replacements shown as `old -> new`.
- Deletes shown as `path -> [deleted]`.

Use `--quiet` to suppress detailed logs. Progress output appears on stderr when running in a terminal.

## Performance tips

- If you only need to delete paths, use `--delete-path` without `-m` for the fastest path.
- Exclude large directories with `-x` to avoid scanning oversized blobs.
- Use `--quiet` to reduce terminal I/O.

## Example: full rewrite

```sh
gk rewrite \
  -n "New Name" \
  -e "new@example.test" \
  -o "old@example.test" \
  -m olddomain.com:newdomain.com \
  -m token:REDACTED \
  --ignore-case \
  --preserve-case \
  --rename-files \
  -x "vendor/**" \
  --delete-path "secrets/.env"
```

## After the rewrite

Inspect results, then force-push branches and tags:

```sh
git push --force --tags origin main
```
