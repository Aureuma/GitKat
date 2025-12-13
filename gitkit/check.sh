#!/usr/bin/env bash
set -euo pipefail

if [ -z "$1" ]; then
  echo "Usage: $0 <name_to_check>"
  exit 1
fi

NAME_TO_CHECK="$1"

# Loop through all directories containing a .git folder
for repo in */.git; do
  repo_dir="$(dirname "$repo")"
  echo "== Checking repo: $repo_dir =="

  cd "$repo_dir"

  # Use git log to search author or committer names containing '$NAME_TO_CHECK'
  if git log --all --pretty='%an <%ae>%n%cn <%ce>' | grep -i "$NAME_TO_CHECK" >/dev/null; then
    echo "Found commits with '$NAME_TO_CHECK' in author or committer fields."
  else
    echo "Nothing."
  fi

  cd - >/dev/null
  echo
done
