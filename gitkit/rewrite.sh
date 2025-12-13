#!/usr/bin/env bash
set -euo pipefail

# Git history rewriter for identity metadata and blob data (case-preserving).
# - Identity: replace author/committer name/email when matched (case-insensitive match; uses provided casing).
# - Blob data: replace within tree objects without word boundaries; matches are case-insensitive but replacements mirror matched casing.

usage() {
  cat <<'EOF'
Usage: rewrite.sh [-n <new_name>] [-e <new_email>] [-o <old_emails_comma_separated>] [-O <old_name>] [-m <old:new>]...
  -n  New author/committer name (applied when identity matches)
  -e  New author/committer email (required with -o)
  -o  Comma-separated list of old emails to match for identity rewrites
  -O  Optional old author/committer name to require for identity rewrites
  -m  Blob data replacement mapping in the form old:new (repeatable, case-preserving, no word boundaries)
Examples:
  rewrite.sh -n "Jane Example" -e jane@new.com -o old@ex.com -m foo:bar -m olddomain.com:newdomain.com
  rewrite.sh -m secret:REDACTED
EOF
  exit 1
}

NEW_NAME=""
NEW_EMAIL=""
OLD_NAME=""
OLD_EMAILS=()
BLOB_MAP=()

while getopts ":n:e:o:O:m:h" opt; do
  case $opt in
    n) NEW_NAME="$OPTARG" ;;
    e) NEW_EMAIL="$OPTARG" ;;
    o) IFS=',' read -r -a OLD_EMAILS <<< "$OPTARG" ;;
    O) OLD_NAME="$OPTARG" ;;
    m) BLOB_MAP+=("$OPTARG") ;;
    h|\?) usage ;;
    :) echo "Option -$OPTARG requires an argument." >&2; usage ;;
  esac
done

if [ ${#OLD_EMAILS[@]} -eq 0 ] && [ ${#BLOB_MAP[@]} -eq 0 ]; then
  echo "Error: specify at least one identity rewrite (-o/-e) or blob data mapping (-m)." >&2
  usage
fi

if [ ${#OLD_EMAILS[@]} -gt 0 ] && [ -z "${NEW_EMAIL:-}" ]; then
  echo "Error: identity rewrites require -e <new_email> along with -o <old_emails>." >&2
  usage
fi

if [ -n "${NEW_EMAIL:-}" ] && [ ${#OLD_EMAILS[@]} -eq 0 ]; then
  echo "Error: -e was provided without any -o entries to match." >&2
  usage
fi

BLOB_SERIALIZED=""
for pair in "${BLOB_MAP[@]}"; do
  if [[ "$pair" != *:* ]]; then
    echo "Invalid -m entry '$pair'. Expected old:new." >&2
    exit 1
  fi
  old=${pair%%:*}
  new=${pair#*:}
  BLOB_SERIALIZED+="${old}"$'\t'"${new}"$'\n'
done

OLD_EMAILS_SERIALIZED="$(printf '%s\n' "${OLD_EMAILS[@]}")"

for repo in */.git; do
  repo_dir="${repo%/.git}"
  echo
  echo "========================================"
  echo " Processing: $repo_dir"
  echo "========================================"
  cd "$repo_dir"

  old_remote_url=$(git config remote.origin.url 2>/dev/null || true)

  GITKIT_NEW_NAME="$NEW_NAME" \
  GITKIT_NEW_EMAIL="$NEW_EMAIL" \
  GITKIT_OLD_NAME="$OLD_NAME" \
  GITKIT_OLD_EMAILS="$OLD_EMAILS_SERIALIZED" \
  GITKIT_BLOB_MAP="$BLOB_SERIALIZED" \
  git filter-repo --force \
    --commit-callback '
import os
import re

new_name = os.environ.get("GITKIT_NEW_NAME", "").encode()
new_email = os.environ.get("GITKIT_NEW_EMAIL", "").encode()
old_name_raw = os.environ.get("GITKIT_OLD_NAME", "")
old_name = old_name_raw.lower() if old_name_raw else None
old_emails = {e.lower() for e in os.environ.get("GITKIT_OLD_EMAILS", "").splitlines() if e}
identity_enabled = bool(new_email and old_emails)

def lower_bytes(val):
    try:
        return val.decode().lower()
    except Exception:
        return val.lower()

def rewrite_identity(commit):
    changed = False
    if not identity_enabled:
        return changed

    a_email = lower_bytes(commit.author_email)
    a_name = lower_bytes(commit.author_name)
    if a_email in old_emails and (not old_name or a_name == old_name):
        old_a_name = commit.author_name.decode(errors="ignore")
        old_a_email = commit.author_email.decode(errors="ignore")
        if new_name:
            commit.author_name = new_name
        commit.author_email = new_email
        print(f"[Author] {old_a_name} <{old_a_email}>  →  {(new_name or commit.author_name).decode(errors=\"ignore\")} <{new_email.decode(errors=\"ignore\")}>")
        changed = True

    c_email = lower_bytes(commit.committer_email)
    c_name = lower_bytes(commit.committer_name)
    if c_email in old_emails and (not old_name or c_name == old_name):
        old_c_name = commit.committer_name.decode(errors="ignore")
        old_c_email = commit.committer_email.decode(errors="ignore")
        if new_name:
            commit.committer_name = new_name
        commit.committer_email = new_email
        print(f"[Committer] {old_c_name} <{old_c_email}>  →  {(new_name or commit.committer_name).decode(errors=\"ignore\")} <{new_email.decode(errors=\"ignore\")}>")
        changed = True

    if changed:
        msg = commit.message
        msg = re.sub(rb"(?im)^\s*(signed-off-by|co-authored-by|reviewed-by|acked-by|tested-by|reported-by|suggested-by):.*\n?", b"", msg)
        if msg != commit.message:
            print("[Message Cleanup] Removed DCO trace lines")
        commit.message = msg
    return changed

rewrite_identity(commit)
' \
    --blob-callback '
import os
import re

raw_pairs = [line for line in os.environ.get("GITKIT_BLOB_MAP", "").splitlines() if line]
if not raw_pairs:
    return

pairs = []
for line in raw_pairs:
    if "\t" not in line:
        continue
    old, new = line.split("\t", 1)
    pairs.append((old.encode(), new.encode()))

if not pairs:
    return

patterns = [(re.compile(re.escape(old), re.IGNORECASE), new) for old, new in pairs]

def preserve_case(match, replacement):
    # Mirror the matched casing pattern onto the replacement.
    src = match.group(0)
    if not replacement:
        return replacement
    if src.isupper():
        return replacement.upper()
    if src.islower():
        return replacement.lower()
    if src[:1].isupper() and src[1:].islower():
        return replacement[:1].upper() + replacement[1:].lower()
    out = bytearray()
    for i, b in enumerate(replacement):
        if i < len(src):
            sb = chr(src[i])
            rb = chr(b)
            if sb.isupper():
                out.append(ord(rb.upper()))
            elif sb.islower():
                out.append(ord(rb.lower()))
            else:
                out.append(b)
        else:
            out.append(b)
    return bytes(out)

if b"\0" in blob.data:
    return

data = blob.data
for pattern, replacement in patterns:
    data, _ = pattern.subn(lambda m, r=replacement: preserve_case(m, r), data)

blob.data = data
'

  if [ -n "$old_remote_url" ]; then
    git remote add origin "$old_remote_url"
  fi

  echo
  echo "---- Summary for $repo_dir ----"
  total=$(git rev-list --all --count)
  echo "Total commits:               $total"
  if [ -n "${NEW_EMAIL:-}" ]; then
    replaced=$(git log --all --format='%ae' | grep -i -c "${NEW_EMAIL}" || echo 0)
    echo "Commits now using new email: $replaced"
  else
    echo "Commits now using new email: (identity rewrite skipped)"
  fi
  echo "Blob mappings applied:       ${#BLOB_MAP[@]}"
  echo "Remote(s):"
  git remote -v || echo "  (none)"
  echo "----------------------------------------"

  cd ..
done

echo
echo "✅ Rewrite complete (identity metadata + blob data)."
echo "Verify logs, then push rewritten histories with:"
echo "  git push --force --tags origin main"
