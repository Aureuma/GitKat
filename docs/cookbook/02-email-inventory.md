# Build an email inventory for a training repo

<div class="cookbook-meta">
  <span class="level-badge level-easy">Easy</span>
  <code>Repo: githubtraining/hellogitworld</code>
  <code>Size: small</code>
</div>

A training repo has inconsistent contributor emails. You need a quick inventory before contacting people.

## Goal

List every unique author email in the repo.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/githubtraining/hellogitworld.git
cd hellogitworld

# Inventory emails in this repo

gk report .
```

## Notes

- `gk report` reads history only and works on a single repo or a directory of repos.
- Save the output and decide which addresses to consolidate later.
