#  GitKit 🛠️

A collection of scripts for managing and auditing Git repositories in bulk.

---

## Scripts

### `check.sh`

Audits the commit history of all local repositories.

- **What it does:** Scans all subdirectories for Git repositories and checks the commit logs for a specific author or committer name provided as an argument.
- **How to run:**
  ```sh
  ./gitkit/check.sh "<name_to_check>"
  ```

### `report.sh`

Generates a report of all unique author emails found in the commit history of local repositories.

- **What it does:** Finds all Git repositories in the specified directory (or current directory by default) and lists the unique author emails from their commit logs.
- **How to run:**
  ```sh
  ./gitkit/report.sh [path/to/repos]
  ```

### `rewrite.sh`

Rewrites commit history across multiple repositories to unify author identity.

- **What it does:** Uses `git-filter-repo` to replace old author/committer names and emails with new ones. It also cleans up commit message footers like `Signed-off-by:`.
- **Prerequisites:** Requires `git-filter-repo`. Install it with Homebrew:
  ```sh
  brew install git-filter-repo
  ```
- **How to run:**
  ```sh
  ./gitkit/rewrite.sh -n "New Name" -e "new@email.com" -o "old1@email.com,old2@email.com"
  ```
  > **Warning:** This script rewrites Git history. After verifying the changes, you will need to force-push.

### `push.sh`

Force-pushes the current branch of all local repositories to their `origin` remote.

- **What it does:** Iterates through all repositories in the subdirectories and force-pushes the currently checked-out branch.
- **How to run:**
  ```sh
  ./gitkit/push.sh
  ```
  > **Note:** Use with caution, as it force-pushes.

### `github_email_finder.py`

Finds all unique email addresses associated with your GitHub contributions.

- **What it does:** Scans all repositories you own and contribute to (in organizations) to find every email address you've used for commits and pull requests.
- **How to run:**
  ```sh
  python3 ./gitkit/github_email_finder.py --token YOUR_GITHUB_TOKEN
  ```
  You'll need to provide a GitHub Personal Access Token with repository access.