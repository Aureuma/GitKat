#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <group1|group2|group3|group4>" >&2
  exit 2
fi

group="$1"

GK_BIN="${GK_BIN:-}"
if [[ -z "$GK_BIN" ]]; then
  if command -v gk >/dev/null 2>&1; then
    GK_BIN="$(command -v gk)"
  else
    GK_BIN="${PWD}/target/release/gk"
  fi
fi

if [[ ! -x "$GK_BIN" ]]; then
  echo "gk binary not found. Set GK_BIN to the compiled binary." >&2
  exit 1
fi

WORK_DIR="${WORK_DIR:-${PWD}/target/cookbook}"
mkdir -p "$WORK_DIR"

quiet_in_ci() {
  if [[ "${CI:-}" == "true" ]]; then
    printf '%s\n' "--quiet"
  fi
}

clone_repo() {
  local url="$1"
  local dest="$2"

  if [[ -d "$dest/.git" ]]; then
    return 0
  fi

  git clone $(quiet_in_ci) "$url" "$dest"
}

run_rewrite() {
  "$GK_BIN" rewrite $(quiet_in_ci) "$@"
}

recipe_01() {
  local repo="$WORK_DIR/01-hello-world"
  clone_repo "https://github.com/octocat/Hello-World.git" "$repo"
  (
    cd "$repo"
    git log -n 20 --format="%an <%ae>" || true
    "$GK_BIN" check "octocat"
    "$GK_BIN" report .
  )
}

recipe_02() {
  local repo="$WORK_DIR/02-hellogitworld"
  clone_repo "https://github.com/githubtraining/hellogitworld.git" "$repo"
  (
    cd "$repo"
    "$GK_BIN" report .
  )
}

recipe_03() {
  local repo="$WORK_DIR/03-requests"
  clone_repo "https://github.com/psf/requests.git" "$repo"
  (
    cd "$repo"
    "$GK_BIN" report .
    run_rewrite -o "old@example.com" -e "new@example.com" -n "New Name"
  )
}

recipe_04() {
  local repo="$WORK_DIR/04-gitignore"
  clone_repo "https://github.com/github/gitignore.git" "$repo"
  (
    cd "$repo"
    git log -p -S "TOKEN" | head -n 40 || true
    run_rewrite -m "TOKEN:[redacted]" -m "SECRET:[redacted]"
  )
}

recipe_05() {
  local repo="$WORK_DIR/05-rustlings"
  clone_repo "https://github.com/rust-lang/rustlings.git" "$repo"
  (
    cd "$repo"
    git grep -n "rustlings" | head -n 20 || true
    run_rewrite -m "rustlings:rustcamp" --ignore-case --preserve-case
  )
}

recipe_06() {
  local repo="$WORK_DIR/06-fastify"
  clone_repo "https://github.com/fastify/fastify.git" "$repo"
  (
    cd "$repo"
    git ls-files | grep -E '\\.env($|\\.)' || true
    run_rewrite \
      --delete-path ".env" \
      --delete-path ".env.local" \
      --delete-path "**/.env" \
      --delete-path "**/.env.*"
  )
}

recipe_07() {
  local repo="$WORK_DIR/07-tailwindcss"
  clone_repo "https://github.com/tailwindlabs/tailwindcss.git" "$repo"
  (
    cd "$repo"
    git ls-files | grep -E '/(dist|build)/' | head -n 20 || true
    run_rewrite \
      --delete-path "**/dist/**" \
      --delete-path "**/build/**" \
      --delete-path "**/*.map"
  )
}

recipe_08() {
  local repo="$WORK_DIR/08-cli"
  clone_repo "https://github.com/cli/cli.git" "$repo"
  (
    cd "$repo"
    git ls-files docs | head -n 20 || true
    run_rewrite \
      --rename-files \
      -m "docs/changelog:docs/releases" \
      -m "docs/changelog.md:docs/releases.md"
  )
}

recipe_09() {
  local repo="$WORK_DIR/09-axios"
  clone_repo "https://github.com/axios/axios.git" "$repo"
  (
    cd "$repo"
    git log -p -S "AKIA" | head -n 40 || true
    run_rewrite \
      --regex \
      -m "AKIA[0-9A-Z]{16}:[redacted]" \
      -m "sk_live_[0-9a-zA-Z]{24}:[redacted]"
  )
}

recipe_10() {
  local repo="$WORK_DIR/10-node"
  clone_repo "https://github.com/nodejs/node.git" "$repo"
  (
    cd "$repo"
    run_rewrite \
      -x "deps/**" \
      -x "test/fixtures/**" \
      -m "http\\://nodejs.org:https\\://nodejs.org" \
      --ignore-case
  )
}

recipe_11() {
  local parent="$WORK_DIR/11-bulk"
  mkdir -p "$parent"
  clone_repo "https://github.com/octocat/Hello-World.git" "$parent/Hello-World"
  clone_repo "https://github.com/githubtraining/hellogitworld.git" "$parent/hellogitworld"
  clone_repo "https://github.com/github/gitignore.git" "$parent/gitignore"
  (
    cd "$parent"
    run_rewrite -m "http\\://:https\\://" --ignore-case
  )
}

recipe_12() {
  local repo="$WORK_DIR/12-seaborn"
  clone_repo "https://github.com/mwaskom/seaborn.git" "$repo"
  (
    cd "$repo"
    git ls-files | grep -E '\\.csv$' | head -n 20 || true
    run_rewrite --delete-path "**/*.csv"
  )
}

recipe_13() {
  local repo="$WORK_DIR/13-simple-icons"
  clone_repo "https://github.com/simple-icons/simple-icons.git" "$repo"
  (
    cd "$repo"
    git grep -n "\\.jpeg" | head -n 20 || true
    run_rewrite --rename-files -m ".jpeg:.jpg" --ignore-case
  )
}

recipe_14() {
  local repo="$WORK_DIR/14-terraform"
  clone_repo "https://github.com/hashicorp/terraform.git" "$repo"
  (
    cd "$repo"
    run_rewrite -m "corp.internal:example.com" --ignore-case
  )
}

recipe_15() {
  local repo="$WORK_DIR/15-ripgrep"
  clone_repo "https://github.com/BurntSushi/ripgrep.git" "$repo"
  (
    cd "$repo"
    "$GK_BIN" report .
    run_rewrite -o "old@example.com" -e "new@example.com" -n "New Name"
  )
}

recipe_16() {
  local repo="$WORK_DIR/16-react"
  clone_repo "https://github.com/facebook/react.git" "$repo"
  (
    cd "$repo"
    run_rewrite \
      --delete-path "**/.DS_Store" \
      --delete-path "**/.idea/**"
  )
}

recipe_17() {
  local repo="$WORK_DIR/17-googletest"
  clone_repo "https://github.com/google/googletest.git" "$repo"
  (
    cd "$repo"
    run_rewrite --regex \
      -m $'By\\: Shayan Amani\\n\\nFeel free to PR\\. \\:heart_eyes\\::[redacted]'
  )
}

recipe_18() {
  local repo="$WORK_DIR/18-deno"
  clone_repo "https://github.com/denoland/deno.git" "$repo"
  (
    cd "$repo"
    run_rewrite --rename-files -m "example.env:sample.env" --ignore-case
  )
}

recipe_19() {
  local repo="$WORK_DIR/19-brew"
  clone_repo "https://github.com/Homebrew/brew.git" "$repo"
  (
    cd "$repo"
      run_rewrite \
      -m "git@corp.example.com:git@github.com" \
      -m "https\\://corp.example.com/:https\\://github.com/" \
      --ignore-case
  )
}

recipe_20() {
  "$GK_BIN" verify-rewrite --ci --with-blob --with-regex
}

case "$group" in
  group1)
    recipe_01
    recipe_02
    recipe_03
    recipe_04
    recipe_05
    ;;
  group2)
    recipe_06
    recipe_07
    recipe_08
    recipe_09
    recipe_10
    ;;
  group3)
    recipe_11
    recipe_12
    recipe_13
    recipe_14
    recipe_15
    ;;
  group4)
    recipe_16
    recipe_17
    recipe_18
    recipe_19
    recipe_20
    ;;
  *)
    echo "Unknown group: $group" >&2
    exit 2
    ;;
esac
