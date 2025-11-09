#!/usr/bin/env bash
set -euo pipefail

echo "== Force-pushing current branches of all repos in $(pwd) =="

for repo in */.git; do
  repo_dir="$(dirname "$repo")"
  echo "────────────────────────────────────────────"
  echo "→ Repo: $repo_dir"

  cd "$repo_dir"

  # detect current branch name
  branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "detached")

  if [ "$branch" = "HEAD" ] || [ "$branch" = "detached" ]; then
    echo "  ⚠️  Skipping (detached HEAD)"
  else
    echo "  Detected branch: $branch"
    echo "  Force pushing to origin..."
    git push -f origin "$branch"
  fi

  cd - >/dev/null
done

echo "────────────────────────────────────────────"
echo "All repos processed."
