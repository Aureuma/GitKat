# Commands

## gk check

Search commit history for a name across child repositories of the current directory.

```sh
gk check "Example Name"
```

## gk report

List unique author emails per repository under a path.

```sh
gk report .
```

## gk push

Force-push the current branch for each child repository of the current directory.

```sh
gk push
```

## gk rewrite

Rewrite commit metadata and/or blob content using the Rust gitoxide (gix) rewriter. See the rewrite guide for details.

```sh
gk rewrite -m olddomain.com:newdomain.com --ignore-case --preserve-case
```

Flags:

- `-n`: new author/committer name
- `-e`: new author/committer email
- `-o`: old emails to match (repeatable, comma-separated)
- `-O`: old author/committer name to require
- `-m`: blob mapping `old:new` (repeatable)
- `-x`: exclude globs from blob rewrites (repeatable)
- `-d`, `--delete-path`: delete path or glob (repeatable)
- `--regex`: treat mapping left-hand side as regex
- `--rename-files`: apply mappings to file paths
- `--preserve-case`: mirror casing of matches
- `-i`, `--ignore-case`: match mappings case-insensitively

Delete a file (or glob) across history:

```sh
gk rewrite --delete-path "path/to/file.txt"
gk rewrite --delete-path "assets/**/*.png"
```

## gk github-emails

Find contribution emails for repositories you can access on GitHub.

```sh
gk github-emails --token YOUR_GITHUB_TOKEN
```

## gk verify-rewrite

Compare GitKat rewrite output against `git-filter-repo` across real repositories.

```sh
gk verify-rewrite --ci --with-blob
```

Common options:

- `--ci`: use the smaller CI-safe repo set
- `--with-blob`: include a small blob rewrite check
- `--workdir`: custom working directory for clones
- `--keep-workdir`: do not remove the working directory
