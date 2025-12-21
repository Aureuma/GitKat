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

Rewrite commit metadata and/or blob content using git-filter-repo. See the rewrite guide for details.

```sh
gk rewrite -m olddomain.com:newdomain.com --ignore-case --preserve-case
```

## gk github-emails

Find contribution emails for repositories you can access on GitHub.

```sh
gk github-emails --token YOUR_GITHUB_TOKEN
```
