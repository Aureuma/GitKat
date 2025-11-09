#!/usr/bin/env bash
set -euo pipefail

# Loop through all directories containing a .git folder
for repo in */.git; do
  repo_dir="$(dirname "$repo")"
  echo "== Checking repo: $repo_dir =="

  cd "$repo_dir"

  # Use git log to search author or committer names containing 'My Old Name'
  if git log --all --pretty='%an <%ae>%n%cn <%ce>' | grep -i 'My Old Name' >/dev/null; then
    echo "Found commits with 'My Old Name' in author or committer fields."
  else
    echo "Nothing."
  fi

  cd - >/dev/null
  echo
done
