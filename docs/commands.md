# Commands

GitKat keeps history rewriting and metadata discovery separate. Only `gk rewrite` modifies Git objects; everything else reads or validates history.

## gk check

Search author and committer fields across child repositories of the current directory. If no child repos are found, it falls back to the current repo (if any).

```sh
gk check "Example Name"
```

Use this to confirm a name appears in history before running an identity rewrite.

## gk report

List unique author emails per repository under a path (recursive search).

```sh
gk report .
```

Output is per repo, with one email per line.

## gk push

Force-push the current branch and tags of each child repo to `origin`. Detached HEADs are skipped.

```sh
gk push
```

Use after a rewrite, and coordinate with collaborators before rewriting shared history.

## gk rewrite

Rewrite commit metadata and/or blob contents using the gitoxide (gix) rewrite engine.

Minimum inputs:

- Identity rewrites require `-o` (old emails) and `-e` (new email).
- Blob rewrites use `-m old:new` mappings.
- File deletes use `--delete-path` globs.

Examples:

```sh
# Identity rewrite
gk rewrite -n "New Name" -e "new@example.test" -o "old@example.test"

# Blob rewrite
gk rewrite -m token:REDACTED

# Regex mapping
gk rewrite -m "token_[0-9]+:REDACTED" --regex

# Case-insensitive + case-preserving
gk rewrite -m olddomain.com:newdomain.com --ignore-case --preserve-case

# Exclude large folders from blob rewrites
gk rewrite -m token:REDACTED -x "data/*.csv" -x "vendor/**"

# Rename file paths using the same mappings
gk rewrite -m OldBrand:NewBrand --rename-files

# Delete paths across history
gk rewrite --delete-path "path/to/file.txt"
gk rewrite --delete-path "assets/**/*.png"
```

Key flags:

- `-n`: new author/committer name
- `-e`: new author/committer email
- `-o`: old emails to match (repeatable, comma-separated)
- `-O`: old author/committer name to require
- `-m`: blob mapping `old:new` (repeatable, escape `:` as `\:`)
- `-x`: exclude globs from blob/path rewriting (repeatable)
- `-d`, `--delete-path`: delete path or glob (repeatable)
- `--regex`: treat mapping left-hand side as regex
- `--rename-files`: apply mappings to file paths
- `--preserve-case`: mirror casing of matches
- `-i`, `--ignore-case`: match mappings case-insensitively
- `-q`, `--quiet`: suppress detailed logs

## gk github-emails

Query GitHub to find emails you used in commits and PRs.

```sh
gk github-emails --token YOUR_GITHUB_TOKEN
```

Token guidance:

- `repo` scope for private repositories.
- `read:org` to list organization repos where you have access.

## gk verify-rewrite

Compare GitKat output against `git-filter-repo`, with optional blob-only checks against BFG.

```sh
# Identity + blob parity with git-filter-repo
gk verify-rewrite --ci --with-blob

# Include regex mapping verification
gk verify-rewrite --ci --with-blob --with-regex

# Compare blob-only output with BFG
gk verify-rewrite --ci --with-blob --with-bfg --bfg-jar /path/to/bfg.jar
```

Common options:

- `--ci`: use a smaller repo set for CI validation.
- `--with-blob`: include a small blob rewrite check.
- `--with-regex`: include regex mapping verification.
- `--with-bfg`: compare blob-only rewrites against BFG.
- `--bfg-jar`: explicit path to the BFG jar (or set `BFG_JAR`).
- `--workdir`: custom working directory for clones.
- `--keep-workdir`: do not remove the working directory.

## gk fast-export

Stream a repository as a fast-export stream to stdout or a file.

```sh
# Export to a file
gk fast-export --repo /path/to/repo --output export.fw

# Export to stdout
gk fast-export --repo /path/to/repo > export.fw
```

## gk fast-import

Import a fast-import stream into a repository.

```sh
# Import from a file
gk fast-import --repo /path/to/new-repo --input export.fw

# Import from stdin
cat export.fw | gk fast-import --repo /path/to/new-repo
```
