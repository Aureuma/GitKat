# Rewrite behavior

The rewrite command uses a Rust helper built on gitoxide (gix) and preserves the behavior of the legacy `rewrite.sh` script.

## Scope

`gk rewrite` can:

- Rewrite author/committer identity metadata.
- Replace content inside text blobs (case-aware mappings).
- Rename file paths using the same mappings.
- Delete files across all history using exact paths or globs.

## Identity rewrite

```sh
gk rewrite -n "New Name" -e "new@example.test" -o "old@example.test"
```

Optionally require a specific old name:

```sh
gk rewrite -n "New Name" -e "new@example.test" -o "old@example.test" -O "Old Name"
```

## Blob rewrite

```sh
gk rewrite -m olddomain.com:newdomain.com -m token:REDACTED
```

## Regex mappings

```sh
gk rewrite -m "token_[0-9]+:REDACTED" --regex
```

When `--regex` is enabled, the left side of each `-m old:new` mapping is treated as a Rust regex.

## Case handling

```sh
gk rewrite -m foo:bar --ignore-case --preserve-case
```

- `--ignore-case` matches all casings.
- `--preserve-case` mirrors the matched casing onto the replacement.

## Excluding files

```sh
gk rewrite -m token:REDACTED -x "data/*.csv" -x "vendor/*"
```

## Renaming file paths

```sh
gk rewrite -m oldname:newname --rename-files
```

## Deleting paths

```sh
gk rewrite --delete-path "path/to/file.txt"
gk rewrite --delete-path "assets/**/*.png"
```

Delete paths accept glob patterns and log each removed file in the same colored output style as other rewrites.

## Logging

Rewrite output is verbose by default:

- File paths are printed in magenta.
- Match text is printed in red and replacements in blue.
- Deleted files log `path -> [deleted]`.

## Verification

To compare rewrite output against `git-filter-repo`:

```sh
gk verify-rewrite --ci --with-blob
```

## Notes

- Rewrites skip binary blobs.
- Running from a directory containing multiple repos rewrites each child repo.
- Running inside a repo rewrites only that repo.
- After verifying, force-push rewritten history.
- The rewrite engine runs in-process via gitoxide (gix).
