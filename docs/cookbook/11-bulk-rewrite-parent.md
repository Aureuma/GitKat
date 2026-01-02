# Bulk rewrite multiple repos from a parent directory

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: many</code>
  <code>Size: varies</code>
</div>

You have a directory with many repos and want to apply the same rewrite to each.

## Goal

Run a single command to rewrite each child repo in place.

## Steps

```sh
cd ~/workspaces/company-repos

gk rewrite -m olddomain.com:newdomain.com --ignore-case --preserve-case
```

After verifying, push all repos:

```sh
gk push
```

## Notes

- GitKat rewrites direct child repos of the current directory.
- Run inside a repo to target just that repo.
- Always verify before force-pushing.
