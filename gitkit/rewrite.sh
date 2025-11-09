#!/usr/bin/env bash
set -euo pipefail
#
# ──────────────────────────────────────────────────────────────
#  Git Commit Identity Rewriter — Multi-Repo Batch Version
# ──────────────────────────────────────────────────────────────
# Description:
#   Scans all Git repositories in the current directory and rewrites
#   their full commit history to unify author/committer identity and
#   clean commit message traces when necessary.
#
#   For each commit:
#     • If the author or committer email matches any in OLD_EMAILS
#       (case-insensitive), it is replaced with NEW_EMAIL and NEW_NAME.
#     • For those same commits only, the following commit-message
#       footers are removed using regex:
#           Signed-off-by:
#           Co-authored-by:
#           Reviewed-by:
#           Acked-by:
#           Tested-by:
#           Reported-by:
#           Suggested-by:
#     • Remotes are preserved automatically.
#
#   Notes:
#     • Case-insensitive matching for both email and footer cleanup.
#     • Commit hashes change (expected for rewritten history).
#     • After verification, push with `--force`.
#
# Requirements:
#     brew install git-filter-repo
#
# Usage:
#     chmod +x rewrite_commits.sh
#     ./rewrite_commits.sh
#
# ──────────────────────────────────────────────────────────────

NEW_NAME="My New Name"
NEW_EMAIL="mynew@email.com"

declare -a OLD_EMAILS=(
'myold1@email.com'
'myold2@email.com'
'myold3@email.com'
)

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
