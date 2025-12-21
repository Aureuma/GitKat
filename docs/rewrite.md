# Rewrite behavior

The rewrite command is a Python wrapper around git-filter-repo and preserves the behavior of the legacy `rewrite.sh` script.

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

## Notes

- Rewrites skip binary blobs.
- Running from a directory containing multiple repos rewrites each child repo.
- Running inside a repo rewrites only that repo.
- After verifying, force-push rewritten history.
