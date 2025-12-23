# Bulk rewrite multiple repos from a parent directory

<div class="cookbook-meta">
  <span class="level-badge level-intermediate">Intermediate</span>
  <code>Repo: multiple</code>
  <code>Size: mixed</code>
</div>

You manage several small repos that all need the same URL update. Run one command from the parent folder.

## Goal

Apply a single mapping across every Git repo under a directory.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook/bulk
cd ~/scratch/gitkat-cookbook/bulk

git clone https://github.com/octocat/Hello-World.git

git clone https://github.com/githubtraining/hellogitworld.git

git clone https://github.com/github/gitignore.git

# Run once from the parent; GitKat finds all child repos

gk rewrite -m "http://:https://" --ignore-case
```

## Notes

- GitKat scans child directories for .git folders when run from a parent.
- Use this for fleets of repos when the change is consistent.
