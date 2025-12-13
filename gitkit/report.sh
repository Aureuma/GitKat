#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="${1:-.}"

find "$BASE_DIR" -type d -name ".git" | while read -r gitdir; do
    repo_dir="$(dirname "$gitdir")"
    echo
    echo "📁 $repo_dir"
    (
        cd "$repo_dir"
        git log --format='%ae' 2>/dev/null | sort -u || echo "  (no commits)"
    )
done
