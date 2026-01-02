# Build an email inventory for a training repo

<div class="cookbook-meta">
  <span class="level-badge level-easy">Easy</span>
  <code>Repo: githubtraining/hellogitworld</code>
  <code>Size: small</code>
</div>

You want a quick list of emails used in a repository before planning a rewrite.

## Goal

List unique author emails locally and compare them with your GitHub contribution emails.

## Steps

```sh
mkdir -p ~/scratch/gitkat-cookbook
cd ~/scratch/gitkat-cookbook

git clone https://github.com/githubtraining/hellogitworld.git
cd hellogitworld

gk report .
```

Optional: compare to GitHub emails (requires token):

```sh
gk github-emails --token YOUR_GITHUB_TOKEN
```

## Notes

- Use `gk report` to see which emails appear in repo history.
- Use `gk github-emails` to confirm which emails you have used across GitHub.
