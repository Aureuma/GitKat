#!/usr/bin/env bash
set -euo pipefail

# ──────────────────────────────────────────────────────────────
#  Git Commit Identity Rewriter — Multi-Repo Batch Version
# ──────────────────────────────────────────────────────────────
# ... (rest of the description remains the same)
# ──────────────────────────────────────────────────────────────

usage() {
  echo "Usage: $0 -n <new_name> -e <new_email> -o <old_emails_comma_separated>"
  echo "  -n: New author and committer name"
  echo "  -e: New author and committer email"
  echo "  -o: Comma-separated list of old emails to replace"
  exit 1
}

while getopts ":n:e:o:" opt; do
  case $opt in
    n) NEW_NAME="$OPTARG"
    ;;
    e) NEW_EMAIL="$OPTARG"
    ;;
    o) IFS=',' read -r -a OLD_EMAILS <<< "$OPTARG"
    ;;
    \?) echo "Invalid option -$OPTARG" >&2; usage
    ;;
    :) echo "Option -$OPTARG requires an argument." >&2; usage
    ;;
  esac
done

if [ -z "${NEW_NAME:-}" ] || [ -z "${NEW_EMAIL:-}" ] || [ ${#OLD_EMAILS[@]} -eq 0 ]; then
    usage
fi

for repo in */.git; do
  repo_dir="${repo%/.git}"
  echo
  echo "========================================"
  echo " Processing: $repo_dir"
  echo "========================================"
  cd "$repo_dir"

  old_remote_url=$(git config remote.origin.url 2>/dev/null || true)

  git filter-repo --force \
    --commit-callback "
import re
old_emails = {$(printf "'%s':1," "${OLD_EMAILS[@]}")}
old_emails = {k.lower(): v for k, v in old_emails.items()}

def lower(s):
    try:
        return s.decode().lower()
    except Exception:
        return s.lower()

changed = False

# Author check
a_email = lower(commit.author_email)
if a_email in old_emails:
    old = commit.author_email.decode()
    commit.author_name = b'$NEW_NAME'
    commit.author_email = b'$NEW_EMAIL'
    print(f'[Author] {old}  →  $NEW_EMAIL')
    changed = True

# Committer check
c_email = lower(commit.committer_email)
if c_email in old_emails:
    old = commit.committer_email.decode()
    commit.committer_name = b'$NEW_NAME'
    commit.committer_email = b'$NEW_EMAIL'
    print(f'[Committer] {old}  →  $NEW_EMAIL')
    changed = True

# Remove trace lines if commit changed
if changed:
    msg = commit.message
    msg = re.sub(rb'(?im)^\\s*(signed-off-by|co-authored-by|reviewed-by|acked-by|tested-by|reported-by|suggested-by):.*\\n?', b'', msg)
    if msg != commit.message:
        print(f'[Message Cleanup] Removed DCO trace lines')
    commit.message = msg
"

  if [ -n "$old_remote_url" ]; then
    git remote add origin "$old_remote_url"
  fi

  echo
  echo "---- Summary for $repo_dir ----"
  total=$(git rev-list --all --count)
  replaced=$(git log --all --format='%ae' | grep -i -c "$NEW_EMAIL" || echo 0)
  echo "Total commits:        $total"
  echo "Now using new email:  $replaced"
  echo "Remote(s):"
  git remote -v || echo "  (none)"
  echo "----------------------------------------"

  cd ..
done

echo
echo "✅ Rewrite complete (case-insensitive identity + trace cleanup + remotes restored)."
echo "Verify logs, then push rewritten histories with:"
echo "  git push --force --tags origin main"

