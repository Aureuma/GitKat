#!/usr/bin/env bash
set -euo pipefail

# Git history rewriter for identity metadata and blob data.
# - Identity: replace author/committer name/email when matched (case-insensitive match; uses provided casing).
# - Blob data: replace within tree objects without word boundaries.
#   * Matching: case-sensitive by default; add --ignore-case to match all casings.
#   * Replacements: add --preserve-case to mirror each match's casing onto the replacement (works with or without --ignore-case).
#   * Reporting: per-repo dividers, per-file colored lines (path magenta; match red; replacement blue) with single-line context.

usage() {
  cat <<'EOF'
Usage: rewrite.sh [-n <new_name>] [-e <new_email>] [-o <old_emails_comma_separated>] [-O <old_name>] [-m <old:new>] [-x <glob>] [--preserve-case] [--ignore-case|-i]
  -n  New author/committer name (applied when identity matches)
  -e  New author/committer email (required with -o)
  -o  Comma-separated list of old emails to match for identity rewrites
  -O  Optional old author/committer name to require for identity rewrites
  -m  Blob data replacement mapping in the form old:new (repeatable, no word boundaries)
  -x  Exclude files from blob rewrites (glob, repeatable; comma-separated allowed)
  --preserve-case    Mirror matched casing onto replacements (blob data only; works with or without --ignore-case)
  --ignore-case, -i  Apply blob replacements case-insensitively (default is case-sensitive for blob data)
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
EXCLUDE_PATTERNS=()
PRESERVE_CASE=0
IGNORE_CASE=0

require_arg() {
  if [ $# -lt 2 ] || [[ "$2" == -* ]]; then
    echo "Option $1 requires an argument." >&2
    usage
  fi
}

while [ $# -gt 0 ]; do
  case "$1" in
    -n)
      require_arg "$1" "${2:-}"
      NEW_NAME="$2"; shift 2 ;;
    -e)
      require_arg "$1" "${2:-}"
      NEW_EMAIL="$2"; shift 2 ;;
    -o)
      require_arg "$1" "${2:-}"
      IFS=',' read -r -a OLD_EMAILS <<< "$2"; shift 2 ;;
    -O)
      require_arg "$1" "${2:-}"
      OLD_NAME="$2"; shift 2 ;;
    -m)
      require_arg "$1" "${2:-}"
      BLOB_MAP+=("$2"); shift 2 ;;
    -x)
      require_arg "$1" "${2:-}"
      IFS=',' read -r -a exclude_items <<< "$2"
      EXCLUDE_PATTERNS+=("${exclude_items[@]}"); shift 2 ;;
    --preserve-case)
      PRESERVE_CASE=1; shift ;;
    --ignore-case|-i)
      IGNORE_CASE=1; shift ;;
    -h|--help)
      usage ;;
    --)
      shift; break ;;
    -*)
      echo "Unknown option $1" >&2
      usage ;;
    *)
      break ;;
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
if [ ${#BLOB_MAP[@]} -gt 0 ]; then
  for pair in "${BLOB_MAP[@]}"; do
    if [[ "$pair" != *:* ]]; then
      echo "Invalid -m entry '$pair'. Expected old:new." >&2
      exit 1
    fi
    old=${pair%%:*}
    new=${pair#*:}
    BLOB_SERIALIZED+="${old}"$'\t'"${new}"$'\n'
  done
fi

if [ ${#OLD_EMAILS[@]} -gt 0 ]; then
  OLD_EMAILS_SERIALIZED="$(printf '%s\n' "${OLD_EMAILS[@]}")"
else
  OLD_EMAILS_SERIALIZED=""
fi

if [ ${#EXCLUDE_PATTERNS[@]} -gt 0 ]; then
  EXCLUDE_SERIALIZED="$(printf '%s\n' "${EXCLUDE_PATTERNS[@]}")"
else
  EXCLUDE_SERIALIZED=""
fi

shopt -s nullglob
start_dir="$(pwd)"
repos=(*/.git)
if [ ${#repos[@]} -eq 0 ]; then
  if git_root="$(git rev-parse --show-toplevel 2>/dev/null)"; then
    repos=("$git_root/.git")
  else
    echo "Error: no git repositories found under $(pwd). Run from a parent directory containing repos or from inside a repo." >&2
    exit 1
  fi
fi

for repo in "${repos[@]}"; do
  repo_dir="${repo%/.git}"
  echo
  echo "========================================"
  echo " Repo: $repo_dir"
  echo "========================================"
  cd "$repo_dir"

  REMOTE_DUMP=""
  while IFS= read -r r; do
    fetch_urls=()
    while IFS= read -r u; do fetch_urls+=("$u"); done < <(git config --get-all remote."$r".url || true)
    push_urls=()
    while IFS= read -r u; do push_urls+=("$u"); done < <(git config --get-all remote."$r".pushurl || true)
    line="$r"
    for u in "${fetch_urls[@]:-}"; do line+=$'\tf:'"$u"; done
    for u in "${push_urls[@]:-}"; do line+=$'\tp:'"$u"; done
    REMOTE_DUMP+="$line"$'\n'
  done < <(git remote)

  GITKIT_NEW_NAME="$NEW_NAME" \
  GITKIT_NEW_EMAIL="$NEW_EMAIL" \
  GITKIT_OLD_NAME="$OLD_NAME" \
  GITKIT_OLD_EMAILS="$OLD_EMAILS_SERIALIZED" \
  GITKIT_BLOB_MAP="$BLOB_SERIALIZED" \
  GITKIT_EXCLUDE_PATTERNS="$EXCLUDE_SERIALIZED" \
  GITKIT_PRESERVE_CASE="$PRESERVE_CASE" \
  GITKIT_IGNORE_CASE="$IGNORE_CASE" \
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
        print(f"[Author] {old_a_name} <{old_a_email}>  →  {(new_name or commit.author_name).decode(errors='ignore')} <{new_email.decode(errors='ignore')}>")
        changed = True

    c_email = lower_bytes(commit.committer_email)
    c_name = lower_bytes(commit.committer_name)
    if c_email in old_emails and (not old_name or c_name == old_name):
        old_c_name = commit.committer_name.decode(errors="ignore")
        old_c_email = commit.committer_email.decode(errors="ignore")
        if new_name:
            commit.committer_name = new_name
        commit.committer_email = new_email
        print(f"[Committer] {old_c_name} <{old_c_email}>  →  {(new_name or commit.committer_name).decode(errors='ignore')} <{new_email.decode(errors='ignore')}>")
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
    --file-info-callback '
import fnmatch
import os
import re

raw_pairs = [line for line in os.environ.get("GITKIT_BLOB_MAP", "").splitlines() if line]
exclude_raw = [line for line in os.environ.get("GITKIT_EXCLUDE_PATTERNS", "").splitlines() if line]
ignore_case = os.environ.get("GITKIT_IGNORE_CASE", "0") == "1"
preserve_case_enabled = os.environ.get("GITKIT_PRESERVE_CASE", "0") == "1"
CTX_WORDS = 2
COLOR_PATH = "\033[95m"   # magenta for file paths
COLOR_MATCH = "\033[31m"  # red for matches
COLOR_REPL = "\033[34m"   # blue for replacements
COLOR_RESET = "\033[0m"
if not raw_pairs:
    return (filename, mode, blob_id)

path_bytes = filename or b""
path_str = path_bytes.decode("utf-8", "ignore") or "<unknown path>"

state = value.data.setdefault("gitkit_blob_state", {})
exclude_patterns = state.get("exclude_patterns")
if exclude_patterns is None:
    state["exclude_patterns"] = exclude_raw
    exclude_patterns = state["exclude_patterns"]

if exclude_patterns:
    for pat in exclude_patterns:
        if fnmatch.fnmatchcase(path_str, pat):
            return (filename, mode, blob_id)

patterns = state.get("patterns")
if patterns is None:
    pairs = []
    for line in raw_pairs:
        if "\t" not in line:
            continue
        old, new = line.split("\t", 1)
        pairs.append((old.encode(), new.encode()))

    if not pairs:
        state["patterns"] = []
    else:
        re_flags = re.IGNORECASE if ignore_case else 0
        state["patterns"] = [(re.compile(re.escape(old), re_flags), new) for old, new in pairs]
    patterns = state["patterns"]

if not patterns:
    return (filename, mode, blob_id)

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

contents = value.get_contents_by_identifier(blob_id)
if value.is_binary(contents):
    return (filename, mode, blob_id)

data = contents

def decode_snippet(b):
    return b.decode("utf-8", "replace")

def extract_context(line_text, match_text, repl_text, match_pos):
    prefix = line_text[:match_pos]
    suffix = line_text[match_pos + len(match_text):]
    pre_words = prefix.strip().split()
    post_words = suffix.strip().split()
    left = " ".join(pre_words[-CTX_WORDS:])
    right = " ".join(post_words[:CTX_WORDS])
    left = (left + " ").strip() if left else ""
    right = (" " + right).strip() if right else ""
    left_line = f"{left}{COLOR_MATCH}{match_text}{COLOR_RESET}{right}".strip()
    right_line = f"{left}{COLOR_REPL}{repl_text}{COLOR_RESET}{right}".strip()
    return left_line, right_line

printed_path = False
changed = False
for pattern, replacement in patterns:
    matches = list(pattern.finditer(data))
    if not matches:
        continue

    changed = True
    snapshot = data
    new_data = bytearray()
    last = 0
    for m in matches:
        repl_bytes = preserve_case(m, replacement) if preserve_case_enabled else replacement
        new_data.extend(snapshot[last:m.start()])
        new_data.extend(repl_bytes)
        if not printed_path:
            print(f"{COLOR_PATH}{path_str}{COLOR_RESET}")
            printed_path = True
        line_start = snapshot.rfind(b"\n", 0, m.start()) + 1
        line_end = snapshot.find(b"\n", m.end())
        if line_end == -1:
            line_end = len(snapshot)
        line_bytes = snapshot[line_start:line_end]
        line_text = decode_snippet(line_bytes)
        match_text = decode_snippet(m.group(0))
        repl_text = decode_snippet(repl_bytes)
        rel_match_pos = m.start() - line_start
        left_line, right_line = extract_context(line_text, match_text, repl_text, rel_match_pos)
        print(f"{COLOR_PATH}{path_str}{COLOR_RESET} {left_line} -> {right_line}")
        last = m.end()
    new_data.extend(snapshot[last:])
    data = bytes(new_data)

if not changed:
    return (filename, mode, blob_id)

new_blob_id = value.insert_file_with_contents(data)
return (filename, mode, new_blob_id)
'

  if [ -n "$REMOTE_DUMP" ]; then
    while IFS= read -r line; do
      [ -z "$line" ] && continue
      IFS=$'\t' read -r -a parts <<< "$line"
      name="${parts[0]}"
      [ -z "$name" ] && continue
      fetch_urls=()
      push_urls=()
      for ((i=1; i<${#parts[@]}; i++)); do
        entry="${parts[$i]}"
        case "$entry" in
          f:*) fetch_urls+=("${entry:2}") ;;
          p:*) push_urls+=("${entry:2}") ;;
        esac
      done
      if [ ${#fetch_urls[@]:-0} -gt 0 ]; then
        git remote add "$name" "${fetch_urls[0]}" 2>/dev/null || git remote set-url "$name" "${fetch_urls[0]}"
        for ((i=1; i<${#fetch_urls[@]}; i++)); do
          git remote set-url --add "$name" "${fetch_urls[$i]}"
        done
      fi
      if [ ${#push_urls[@]:-0} -gt 0 ]; then
        for url in "${push_urls[@]:-}"; do
          git remote set-url --add --push "$name" "$url"
        done
      fi
    done <<< "$REMOTE_DUMP"
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

  cd "$start_dir"
done

echo
echo "✅ Rewrite complete (identity metadata + blob data)."
echo "Verify logs, then push rewritten histories with:"
echo "  git push --force --tags origin main"
