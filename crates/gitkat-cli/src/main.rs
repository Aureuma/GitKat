use std::collections::{BTreeSet, HashMap};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Args, Parser, Subcommand};
use gitkat_rewrite::{rewrite_repo, RewriteConfig};
use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use walkdir::WalkDir;

const REPOS: &[&str] = &[
    "https://github.com/octocat/Hello-World.git",
    "https://github.com/githubtraining/hellogitworld.git",
    "https://github.com/github/gitignore.git",
    "https://github.com/psf/requests.git",
    "https://github.com/pallets/flask.git",
    "https://github.com/BurntSushi/ripgrep.git",
    "https://github.com/tpope/vim-fugitive.git",
    "https://github.com/serde-rs/serde.git",
];

const CI_REPOS: &[&str] = &[
    "https://github.com/octocat/Hello-World.git",
    "https://github.com/githubtraining/hellogitworld.git",
    "https://github.com/github/gitignore.git",
];

const COMMIT_CALLBACK: &str = r#"
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
        if new_name:
            commit.author_name = new_name
        commit.author_email = new_email
        changed = True

    c_email = lower_bytes(commit.committer_email)
    c_name = lower_bytes(commit.committer_name)
    if c_email in old_emails and (not old_name or c_name == old_name):
        if new_name:
            commit.committer_name = new_name
        commit.committer_email = new_email
        changed = True

    if changed:
        msg = commit.message
        msg = re.sub(rb"(?im)^\\s*(signed-off-by|co-authored-by|reviewed-by|acked-by|tested-by|reported-by|suggested-by):.*\\n?", b"", msg)
        commit.message = msg
    return changed

rewrite_identity(commit)
"#;

const FILE_INFO_CALLBACK: &str = r#"
import fnmatch
import os
import re

raw_pairs = [line for line in os.environ.get("GITKIT_BLOB_MAP", "").splitlines() if line]
exclude_raw = [line for line in os.environ.get("GITKIT_EXCLUDE_PATTERNS", "").splitlines() if line]
rename_files = os.environ.get("GITKIT_RENAME_FILES", "0") == "1"
ignore_case = os.environ.get("GITKIT_IGNORE_CASE", "0") == "1"
preserve_case_enabled = os.environ.get("GITKIT_PRESERVE_CASE", "0") == "1"
if not raw_pairs:
    return (filename, mode, blob_id)

path_bytes = filename or b""
path_str = path_bytes.decode("utf-8", "ignore") or "<unknown path>"

state = value.data.setdefault("gitkat_blob_state", {})
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

if rename_files and filename:
    new_filename = filename
    for pattern, replacement in patterns:
        if preserve_case_enabled:
            def repl(m, replacement=replacement):
                return preserve_case(m, replacement)
            new_filename, _n = pattern.subn(repl, new_filename)
        else:
            new_filename, _n = pattern.subn(replacement, new_filename)
    filename = new_filename

contents = value.get_contents_by_identifier(blob_id)
if contents is None:
    return (filename, mode, blob_id)
if value.is_binary(contents):
    return (filename, mode, blob_id)

data = contents
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
        last = m.end()
    new_data.extend(snapshot[last:])
    data = bytes(new_data)

if not changed:
    return (filename, mode, blob_id)

new_blob_id = value.insert_file_with_contents(data)
return (filename, mode, new_blob_id)
"#;

#[derive(Parser)]
#[command(name = "gk", about = "GitKat: bulk Git repository utilities.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Check { name: String },
    Report { path: Option<PathBuf> },
    Push,
    Rewrite(RewriteArgs),
    GithubEmails { token: Option<String> },
    VerifyRewrite(VerifyArgs),
}

#[derive(Args)]
struct RewriteArgs {
    #[arg(short = 'n')]
    new_name: Option<String>,
    #[arg(short = 'e')]
    new_email: Option<String>,
    #[arg(short = 'o', action = ArgAction::Append)]
    old_emails: Vec<String>,
    #[arg(short = 'O')]
    old_name: Option<String>,
    #[arg(short = 'm', action = ArgAction::Append)]
    blob_map: Vec<String>,
    #[arg(short = 'x', action = ArgAction::Append)]
    exclude_patterns: Vec<String>,
    #[arg(long)]
    rename_files: bool,
    #[arg(long)]
    preserve_case: bool,
    #[arg(long, short = 'i')]
    ignore_case: bool,
}

#[derive(Args)]
struct VerifyArgs {
    #[arg(long)]
    workdir: Option<PathBuf>,
    #[arg(long)]
    keep_workdir: bool,
    #[arg(long)]
    with_blob: bool,
    #[arg(long)]
    ci: bool,
    repos: Vec<String>,
}

#[derive(Clone, Debug)]
struct RewriteOptions {
    new_name: String,
    new_email: String,
    old_name: String,
    old_emails: Vec<String>,
    blob_map: Vec<String>,
    exclude_patterns: Vec<String>,
    preserve_case: bool,
    ignore_case: bool,
    rename_files: bool,
}

#[derive(Clone, Debug)]
struct RemoteConfig {
    name: String,
    fetch_urls: Vec<String>,
    push_urls: Vec<String>,
}

#[derive(Clone, Debug)]
struct VerifyOptions {
    new_name: String,
    new_email: String,
    old_email: String,
    old_name: String,
    blob_map: Vec<String>,
    exclude: Vec<String>,
    preserve_case: bool,
    ignore_case: bool,
    rename_files: bool,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Check { name } => run_check(&name, None),
        Commands::Report { path } => run_report(path.as_deref()),
        Commands::Push => run_push(None),
        Commands::Rewrite(args) => run_rewrite(args, None),
        Commands::GithubEmails { token } => run_github_emails(token),
        Commands::VerifyRewrite(args) => run_verify(args),
    };

    match result {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
    }
}

fn run_check(name: &str, base_dir: Option<&Path>) -> Result<i32> {
    let base = base_dir.map(PathBuf::from).unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let repos = list_child_repos(&base)?;
    if repos.is_empty() {
        println!("No git repositories found in {}.", base.display());
        return Ok(1);
    }

    let needle = name.to_lowercase();
    for repo in repos {
        let repo_name = repo
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<unknown>");
        println!("== Checking repo: {} ==", repo_name);
        let output = git_output(["log", "--all", "--pretty=%an <%ae>%n%cn <%ce>"], &repo)?;
        if output.to_lowercase().contains(&needle) {
            println!("Found commits with '{}' in author or committer fields.", name);
        } else {
            println!("Nothing.");
        }
        println!();
    }
    Ok(0)
}

fn run_report(base_dir: Option<&Path>) -> Result<i32> {
    let base = base_dir.map(PathBuf::from).unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let mut repos = list_repos_recursive(&base)?;
    if repos.is_empty() {
        println!("No git repositories found under {}.", base.display());
        return Ok(1);
    }

    repos.sort();
    for repo in repos {
        println!();
        println!("Repo: {}", repo.display());
        let output = git_output_or_empty(["log", "--format=%ae"], &repo);
        let mut emails = BTreeSet::new();
        for line in output.lines() {
            let email = line.trim();
            if !email.is_empty() {
                emails.insert(email.to_string());
            }
        }
        if emails.is_empty() {
            println!("  (no commits)");
        } else {
            for email in emails {
                println!("{}", email);
            }
        }
    }
    Ok(0)
}

fn run_push(base_dir: Option<&Path>) -> Result<i32> {
    let base = base_dir.map(PathBuf::from).unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let repos = list_child_repos(&base)?;
    if repos.is_empty() {
        println!("No git repositories found in {}.", base.display());
        return Ok(1);
    }

    println!("== Force-pushing current branches of all repos in {} ==", base.display());
    for repo in repos {
        let repo_name = repo
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<unknown>");
        println!("--------------------------------------------");
        println!("-> Repo: {}", repo_name);
        let branch = git_output_or_empty(["rev-parse", "--abbrev-ref", "HEAD"], &repo);
        let branch = branch.trim();
        if matches!(branch, "HEAD" | "detached" | "") {
            println!("  Skipping (detached HEAD)");
            continue;
        }
        println!("  Detected branch: {}", branch);
        println!("  Force pushing to origin...");
        run_git_status(["push", "-f", "origin", branch], &repo)?;
    }

    println!("--------------------------------------------");
    println!("All repos processed.");
    Ok(0)
}

fn run_rewrite(args: RewriteArgs, base_dir: Option<&Path>) -> Result<i32> {
    let blob_map = parse_blob_map(&args.blob_map)?;
    let opts = RewriteOptions {
        new_name: args.new_name.unwrap_or_default(),
        new_email: args.new_email.unwrap_or_default(),
        old_name: args.old_name.unwrap_or_default(),
        old_emails: split_comma_args(&args.old_emails),
        blob_map,
        exclude_patterns: split_comma_args(&args.exclude_patterns),
        preserve_case: args.preserve_case,
        ignore_case: args.ignore_case,
        rename_files: args.rename_files,
    };

    if opts.old_emails.is_empty() && opts.blob_map.is_empty() {
        println!("Error: specify at least one identity rewrite (-o/-e) or blob data mapping (-m).");
        return Ok(1);
    }

    if !opts.old_emails.is_empty() && opts.new_email.is_empty() {
        println!("Error: identity rewrites require -e <new_email> along with -o <old_emails>.");
        return Ok(1);
    }

    if !opts.new_email.is_empty() && opts.old_emails.is_empty() {
        println!("Error: -e was provided without any -o entries to match.");
        return Ok(1);
    }

    let base = base_dir.map(PathBuf::from).unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let repos = resolve_repos(&base)?;
    if repos.is_empty() {
        println!("Error: no git repositories found under {}. Run from a parent directory containing repos or from inside a repo.", base.display());
        return Ok(1);
    }

    for (repo, display) in repos {
        println!();
        println!("========================================");
        println!(" Repo: {}", display);
        println!("========================================");
        let remotes = capture_remotes(&repo)?;
        run_gix_rewrite(&repo, &opts)?;
        restore_remotes(&repo, &remotes)?;
        println!();
        println!("---- Summary for {} ----", display);
        print_summary(&repo, &opts)?;
    }

    println!();
    println!("✅ Rewrite complete (identity metadata + blob data).");
    println!("Verify logs, then push rewritten histories with:");
    println!("  git push --force --tags origin main");
    Ok(0)
}

fn run_github_emails(token: Option<String>) -> Result<i32> {
    let token = match token {
        Some(token) => token,
        None => {
            println!("Please provide a token using the --token argument.");
            return Ok(1);
        }
    };

    let client = create_github_client(&token)?;
    let user = get_authenticated_user(&client)?;
    let username = user.login;
    println!("Authenticated as: {}", username);

    println!("\nFetching your repositories...");
    let user_repos = get_user_repos(&client)?;
    println!("Found {} repositories owned by you", user_repos.len());

    println!("\nFetching organization repositories...");
    let org_repos = get_org_repos(&client)?;
    println!("Found {} organization repositories where you have push access", org_repos.len());

    let all_repos = user_repos.into_iter().chain(org_repos.into_iter()).collect::<Vec<_>>();
    println!("\nAnalyzing contributions across {} repositories...", all_repos.len());

    let mut all_emails = BTreeSet::new();
    let mut repo_emails: HashMap<String, BTreeSet<String>> = HashMap::new();

    for (idx, repo) in all_repos.iter().enumerate() {
        let repo_owner = &repo.owner.login;
        let repo_name = &repo.name;
        println!("[{}/{}] Checking {}/{}...", idx + 1, all_repos.len(), repo_owner, repo_name);
        let emails = get_contribution_emails(&client, repo_owner, repo_name, &username)?;
        if !emails.is_empty() {
            repo_emails.insert(
                format!("{}/{}", repo_owner, repo_name),
                emails.iter().cloned().collect(),
            );
            for email in emails {
                all_emails.insert(email);
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!(
        "Found {} unique email addresses across {} repositories:",
        all_emails.len(),
        repo_emails.len()
    );
    println!("{}", "=".repeat(60));

    for email in &all_emails {
        println!("{}", email);
    }

    println!("\nRepository breakdown:");
    for (repo_name, emails) in repo_emails {
        println!("\n{}:", repo_name);
        for email in emails {
            println!("  - {}", email);
        }
    }

    Ok(0)
}

fn run_verify(args: VerifyArgs) -> Result<i32> {
    if !has_filter_repo() {
        println!("git-filter-repo is required for verification");
        return Ok(1);
    }

    if !args.repos.is_empty() && args.ci {
        println!("Provide explicit repos or use --ci, not both");
        return Ok(1);
    }

    let repos: Vec<String> = if !args.repos.is_empty() {
        args.repos
    } else if args.ci {
        CI_REPOS.iter().map(|repo| repo.to_string()).collect()
    } else {
        REPOS.iter().map(|repo| repo.to_string()).collect()
    };

    if repos.is_empty() {
        println!("No repositories provided");
        return Ok(1);
    }

    let mut temp_dir: Option<TempDir> = None;
    let workdir = if let Some(workdir) = args.workdir {
        fs::create_dir_all(&workdir)?;
        workdir
    } else {
        let dir = TempDir::new()?;
        let path = dir.path().to_path_buf();
        temp_dir = Some(dir);
        path
    };

    for url in repos {
        println!("Verifying {}...", url);
        verify_repo(&url, &workdir, args.with_blob)?;
        println!("OK: {}", url);
    }

    if args.keep_workdir {
        if let Some(temp_dir) = temp_dir.take() {
            let _ = temp_dir.keep();
        }
    }

    Ok(0)
}

fn run_gix_rewrite(repo: &Path, options: &RewriteOptions) -> Result<()> {
    let config = RewriteConfig {
        new_name: to_opt(&options.new_name),
        new_email: to_opt(&options.new_email),
        old_name: to_opt(&options.old_name),
        old_emails: options.old_emails.clone(),
        blob_map: options.blob_map.clone(),
        exclude_patterns: options.exclude_patterns.clone(),
        preserve_case: options.preserve_case,
        ignore_case: options.ignore_case,
        rename_files: options.rename_files,
    };
    rewrite_repo(repo, &config)?;
    Ok(())
}

fn print_summary(repo: &Path, opts: &RewriteOptions) -> Result<()> {
    let total = git_output(["rev-list", "--all", "--count"], repo)?;
    println!("Total commits:               {}", total.trim());
    if !opts.new_email.is_empty() {
        let replaced = count_matching_emails(repo, &opts.new_email)?;
        println!("Commits now using new email: {}", replaced);
    } else {
        println!("Commits now using new email: (identity rewrite skipped)");
    }
    println!("Blob mappings applied:       {}", opts.blob_map.len());
    println!("Remote(s):");
    let remotes = git_output_or_empty(["remote", "-v"], repo);
    if remotes.trim().is_empty() {
        println!("  (none)");
    } else {
        println!("{}", remotes.trim_end());
    }
    println!("----------------------------------------");
    Ok(())
}

fn count_matching_emails(repo: &Path, email: &str) -> Result<usize> {
    if email.is_empty() {
        return Ok(0);
    }
    let output = git_output(["log", "--all", "--format=%ae"], repo)?;
    let needle = email.to_lowercase();
    Ok(output
        .lines()
        .filter(|line| line.to_lowercase().contains(&needle))
        .count())
}

fn resolve_repos(base_dir: &Path) -> Result<Vec<(PathBuf, String)>> {
    let repos = list_child_repos(base_dir)?;
    if !repos.is_empty() {
        let output = repos
            .into_iter()
            .map(|repo| {
                let display = repo
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("<unknown>")
                    .to_string();
                (repo, display)
            })
            .collect::<Vec<_>>();
        return Ok(output);
    }

    if let Some(root) = find_git_root(base_dir) {
        return Ok(vec![(root.clone(), root.display().to_string())]);
    }

    Ok(Vec::new())
}

fn split_comma_args(values: &[String]) -> Vec<String> {
    let mut items = Vec::new();
    for value in values {
        for entry in value.split(',') {
            let trimmed = entry.trim();
            if !trimmed.is_empty() {
                items.push(trimmed.to_string());
            }
        }
    }
    items
}

fn parse_blob_map(entries: &[String]) -> Result<Vec<String>> {
    let mut parsed = Vec::new();
    for entry in entries {
        if !entry.contains(':') {
            return Err(anyhow!("Invalid -m entry '{}'. Expected old:new.", entry));
        }
        parsed.push(entry.clone());
    }
    Ok(parsed)
}

fn capture_remotes(repo: &Path) -> Result<Vec<RemoteConfig>> {
    let output = git_output_or_empty(["remote"], repo);
    let names = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());

    let mut remotes = Vec::new();
    for name in names {
        let fetch_urls = git_output_or_empty(["config", "--get-all", &format!("remote.{name}.url")], repo)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        let push_urls = git_output_or_empty(
            [
                "config",
                "--get-all",
                &format!("remote.{name}.pushurl"),
            ],
            repo,
        )
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
        remotes.push(RemoteConfig {
            name: name.to_string(),
            fetch_urls,
            push_urls,
        });
    }
    Ok(remotes)
}

fn restore_remotes(repo: &Path, remotes: &[RemoteConfig]) -> Result<()> {
    for remote in remotes {
        if !remote.fetch_urls.is_empty() {
            let first = &remote.fetch_urls[0];
            let added = run_git(["remote", "add", &remote.name, first], repo, false)?;
            if !added.status.success() {
                run_git(["remote", "set-url", &remote.name, first], repo, false)?;
            }
            for extra in remote.fetch_urls.iter().skip(1) {
                run_git(["remote", "set-url", "--add", &remote.name, extra], repo, false)?;
            }
        }
        if !remote.push_urls.is_empty() {
            for url in &remote.push_urls {
                run_git(
                    ["remote", "set-url", "--add", "--push", &remote.name, url],
                    repo,
                    false,
                )?;
            }
        }
    }
    Ok(())
}

fn list_child_repos(base_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut repos = Vec::new();
    for entry in fs::read_dir(base_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join(".git").is_dir() {
            repos.push(path);
        }
    }
    repos.sort();
    Ok(repos)
}

fn list_repos_recursive(base_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut repos = Vec::new();
    let mut walker = WalkDir::new(base_dir).into_iter();
    while let Some(entry) = walker.next() {
        let entry = entry?;
        if entry.file_type().is_dir() && entry.file_name() == ".git" {
            if let Some(parent) = entry.path().parent() {
                repos.push(parent.to_path_buf());
            }
            walker.skip_current_dir();
        }
    }
    Ok(repos)
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(start)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let path = stdout.trim();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

fn run_git<I, S>(args: I, cwd: &Path, check: bool) -> Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args_vec: Vec<OsString> = args.into_iter().map(|arg| arg.as_ref().to_os_string()).collect();
    let output = Command::new("git")
        .args(&args_vec)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("Failed to spawn git in {}", cwd.display()))?;
    if check && !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let args_display = args_vec
            .iter()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        return Err(anyhow!("git {} failed: {}", args_display, stderr.trim()));
    }
    Ok(output)
}

fn run_git_status<I, S>(args: I, cwd: &Path) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args_vec: Vec<OsString> = args.into_iter().map(|arg| arg.as_ref().to_os_string()).collect();
    let status = Command::new("git")
        .args(&args_vec)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("Failed to spawn git in {}", cwd.display()))?;
    if !status.success() {
        let args_display = args_vec
            .iter()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        return Err(anyhow!("git {} failed", args_display));
    }
    Ok(())
}

fn git_output<I, S>(args: I, cwd: &Path) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = run_git(args, cwd, true)?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn git_output_or_empty<I, S>(args: I, cwd: &Path) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    match run_git(args, cwd, false) {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout).to_string(),
        _ => String::new(),
    }
}

fn to_opt(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn has_filter_repo() -> bool {
    let path_var = env::var_os("PATH").unwrap_or_default();
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(if cfg!(windows) {
            "git-filter-repo.exe"
        } else {
            "git-filter-repo"
        });
        if candidate.is_file() {
            return true;
        }
    }
    false
}

fn verify_repo(url: &str, workdir: &Path, with_blob: bool) -> Result<()> {
    let name = url
        .rsplit('/')
        .next()
        .unwrap_or(url)
        .trim_end_matches(".git");
    let source = workdir.join(name);
    clone_repo(url, &source, workdir)?;

    let (_old_author_name, old_email, old_name) = pick_identity(&source)?;
    let blob_map = if with_blob { pick_blob_map(&source)? } else { Vec::new() };
    let options = VerifyOptions {
        new_name: "GitKat Rewrite".to_string(),
        new_email: "rewrite@example.test".to_string(),
        old_email,
        old_name,
        blob_map,
        exclude: Vec::new(),
        ignore_case: false,
        preserve_case: false,
        rename_files: false,
    };

    let gix_repo = workdir.join(format!("{}-gix", name));
    let filter_repo = workdir.join(format!("{}-filter", name));
    clone_local(&source, &gix_repo, workdir)?;
    clone_local(&source, &filter_repo, workdir)?;

    run_gitkat(&gix_repo, &options)?;
    run_filter_repo(&filter_repo, &options)?;

    let gix_hash = fast_export_hash(&gix_repo)?;
    let filter_hash = fast_export_hash(&filter_repo)?;
    if gix_hash != filter_hash {
        return Err(anyhow!("Mismatch for {url}: {gix_hash} != {filter_hash}"));
    }

    Ok(())
}

fn run_gitkat(repo: &Path, options: &VerifyOptions) -> Result<()> {
    let remotes = capture_remotes(repo)?;
    let config = RewriteConfig {
        new_name: to_opt(&options.new_name),
        new_email: to_opt(&options.new_email),
        old_name: to_opt(&options.old_name),
        old_emails: vec![options.old_email.clone()],
        blob_map: options.blob_map.clone(),
        exclude_patterns: options.exclude.clone(),
        preserve_case: options.preserve_case,
        ignore_case: options.ignore_case,
        rename_files: options.rename_files,
    };
    rewrite_repo(repo, &config)?;
    restore_remotes(repo, &remotes)?;
    Ok(())
}

fn run_filter_repo(repo: &Path, options: &VerifyOptions) -> Result<()> {
    let mut envs: HashMap<String, String> = env::vars().collect();
    envs.insert("GITKIT_NEW_NAME".to_string(), options.new_name.clone());
    envs.insert("GITKIT_NEW_EMAIL".to_string(), options.new_email.clone());
    envs.insert("GITKIT_OLD_NAME".to_string(), options.old_name.clone());
    envs.insert(
        "GITKIT_OLD_EMAILS".to_string(),
        serialize_lines(&[options.old_email.clone()]),
    );
    envs.insert(
        "GITKIT_BLOB_MAP".to_string(),
        build_blob_map_env(&options.blob_map)?,
    );
    envs.insert(
        "GITKIT_EXCLUDE_PATTERNS".to_string(),
        serialize_lines(&options.exclude),
    );
    envs.insert(
        "GITKIT_PRESERVE_CASE".to_string(),
        if options.preserve_case { "1" } else { "0" }.to_string(),
    );
    envs.insert(
        "GITKIT_IGNORE_CASE".to_string(),
        if options.ignore_case { "1" } else { "0" }.to_string(),
    );
    envs.insert(
        "GITKIT_RENAME_FILES".to_string(),
        if options.rename_files { "1" } else { "0" }.to_string(),
    );

    let temp_dir = TempDir::new()?;
    let commit_path = temp_dir.path().join("commit_callback.py");
    let file_info_path = temp_dir.path().join("file_info_callback.py");
    fs::write(&commit_path, COMMIT_CALLBACK.trim_start_matches('\n'))?;
    fs::write(&file_info_path, FILE_INFO_CALLBACK.trim_start_matches('\n'))?;

    let status = Command::new("git")
        .arg("filter-repo")
        .arg("--force")
        .arg("--commit-callback")
        .arg(&commit_path)
        .arg("--file-info-callback")
        .arg(&file_info_path)
        .current_dir(repo)
        .envs(envs)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to run git filter-repo")?;

    if !status.success() {
        return Err(anyhow!("git filter-repo failed"));
    }

    Ok(())
}

fn serialize_lines(values: &[String]) -> String {
    values.join("\n")
}

fn build_blob_map_env(entries: &[String]) -> Result<String> {
    let mut lines = Vec::new();
    for entry in entries {
        let (old, new) = entry
            .split_once(':')
            .ok_or_else(|| anyhow!("Invalid blob mapping '{}': expected old:new", entry))?;
        lines.push(format!("{}\t{}", old, new));
    }
    Ok(lines.join("\n"))
}

fn clone_repo(url: &str, dest: &Path, workdir: &Path) -> Result<()> {
    run_git(["clone", "--quiet", url, dest.to_str().unwrap_or("")], workdir, true)?;
    Ok(())
}

fn clone_local(source: &Path, dest: &Path, workdir: &Path) -> Result<()> {
    run_git(
        [
            "clone",
            "--quiet",
            "--no-hardlinks",
            source.to_str().unwrap_or(""),
            dest.to_str().unwrap_or(""),
        ],
        workdir,
        true,
    )?;
    Ok(())
}

fn fast_export_hash(repo: &Path) -> Result<String> {
    let mut child = Command::new("git")
        .arg("fast-export")
        .arg("--all")
        .current_dir(repo)
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to spawn git fast-export")?;
    let mut stdout = child.stdout.take().ok_or_else(|| anyhow!("Missing git fast-export stdout"))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = stdout.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow!("git fast-export --all failed"));
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn pick_identity(repo: &Path) -> Result<(String, String, String)> {
    let line = git_output(["log", "-n", "1", "--format=%an%x00%ae"], repo)?;
    let line = line.trim();
    let (name, email) = line
        .split_once('\0')
        .ok_or_else(|| anyhow!("Unable to read author identity"))?;
    Ok((name.to_string(), email.to_string(), name.to_string()))
}

fn pick_blob_map(repo: &Path) -> Result<Vec<String>> {
    let candidates = ["README.md", "README", "readme.md"];
    let token_re = Regex::new(r"[A-Za-z][A-Za-z0-9_-]{5,}")?;
    for candidate in candidates {
        let spec = format!("HEAD:{}", candidate);
        let output = Command::new("git")
            .arg("show")
            .arg(spec)
            .current_dir(repo)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        let output = match output {
            Ok(output) if output.status.success() => output,
            _ => continue,
        };
        let content = String::from_utf8_lossy(&output.stdout);
        let first_line = content.lines().next().unwrap_or("");
        if let Some(mat) = token_re.find(first_line) {
            let token = mat.as_str();
            return Ok(vec![format!("{}:{}_REDACTED", token, token)]);
        }
    }
    Ok(Vec::new())
}

#[derive(Deserialize)]
struct GithubUser {
    login: String,
}

#[derive(Deserialize, Clone)]
struct GithubOwner {
    login: String,
}

#[derive(Deserialize, Clone)]
struct GithubPermissions {
    push: Option<bool>,
}

#[derive(Deserialize, Clone)]
struct GithubRepo {
    name: String,
    owner: GithubOwner,
    permissions: Option<GithubPermissions>,
}

#[derive(Deserialize)]
struct GithubOrg {
    login: String,
}

#[derive(Deserialize)]
struct GithubCommitWrapper {
    commit: GithubCommit,
}

#[derive(Deserialize)]
struct GithubCommit {
    author: Option<GithubCommitPerson>,
    committer: Option<GithubCommitPerson>,
}

#[derive(Deserialize)]
struct GithubCommitPerson {
    email: Option<String>,
}

#[derive(Deserialize)]
struct GithubPullRequest {
    number: u64,
    user: Option<GithubOwner>,
}

fn create_github_client(token: &str) -> Result<reqwest::blocking::Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {token}").parse()?,
    );
    headers.insert(
        reqwest::header::ACCEPT,
        "application/vnd.github.v3+json".parse()?,
    );
    let client = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .user_agent("GitKat-Email-Finder")
        .build()?;
    Ok(client)
}

fn get_authenticated_user(client: &reqwest::blocking::Client) -> Result<GithubUser> {
    let response = client
        .get("https://api.github.com/user")
        .send()?
        .error_for_status()?;
    Ok(response.json()?)
}

fn get_user_repos(client: &reqwest::blocking::Client) -> Result<Vec<GithubRepo>> {
    let mut repos = Vec::new();
    let mut page = 1;
    loop {
        let response = client
            .get("https://api.github.com/user/repos")
            .query(&[("per_page", "100"), ("page", &page.to_string()), ("affiliation", "owner")])
            .send()?
            .error_for_status()?;
        let batch: Vec<GithubRepo> = response.json()?;
        if batch.is_empty() {
            break;
        }
        repos.extend(batch);
        page += 1;
    }
    Ok(repos)
}

fn get_org_repos(client: &reqwest::blocking::Client) -> Result<Vec<GithubRepo>> {
    let orgs_response = client
        .get("https://api.github.com/user/orgs")
        .send()?
        .error_for_status()?;
    let orgs: Vec<GithubOrg> = orgs_response.json()?;

    let mut all_repos = Vec::new();
    for org in orgs {
        let mut page = 1;
        loop {
            let response = client
                .get(format!("https://api.github.com/orgs/{}/repos", org.login))
                .query(&[("per_page", "100"), ("page", &page.to_string())])
                .send()?
                .error_for_status()?;
            let batch: Vec<GithubRepo> = response.json()?;
            if batch.is_empty() {
                break;
            }
            for repo in batch {
                let has_push = repo
                    .permissions
                    .as_ref()
                    .and_then(|perm| perm.push)
                    .unwrap_or(false);
                if has_push {
                    all_repos.push(repo);
                }
            }
            page += 1;
        }
    }
    Ok(all_repos)
}

fn get_contribution_emails(
    client: &reqwest::blocking::Client,
    repo_owner: &str,
    repo_name: &str,
    username: &str,
) -> Result<BTreeSet<String>> {
    let mut emails = BTreeSet::new();
    let mut page = 1;
    loop {
        let response = client
            .get(format!(
                "https://api.github.com/repos/{}/{}/commits",
                repo_owner, repo_name
            ))
            .query(&[("author", username), ("per_page", "100"), ("page", &page.to_string())])
            .send()?;
        if !response.status().is_success() {
            break;
        }
        let commits: Vec<GithubCommitWrapper> = response.json()?;
        if commits.is_empty() {
            break;
        }
        for commit in commits {
            if let Some(author) = commit.commit.author {
                if let Some(email) = author.email {
                    emails.insert(email);
                }
            }
            if let Some(committer) = commit.commit.committer {
                if let Some(email) = committer.email {
                    emails.insert(email);
                }
            }
        }
        page += 1;
    }

    page = 1;
    loop {
        let response = client
            .get(format!(
                "https://api.github.com/repos/{}/{}/pulls",
                repo_owner, repo_name
            ))
            .query(&[("state", "all"), ("per_page", "100"), ("page", &page.to_string())])
            .send()?;
        if !response.status().is_success() {
            break;
        }
        let pulls: Vec<GithubPullRequest> = response.json()?;
        if pulls.is_empty() {
            break;
        }
        for pr in pulls.iter().filter(|pr| {
            pr.user
                .as_ref()
                .map(|user| user.login == username)
                .unwrap_or(false)
        }) {
            let response = client
                .get(format!(
                    "https://api.github.com/repos/{}/{}/pulls/{}/commits",
                    repo_owner, repo_name, pr.number
                ))
                .send()?;
            if !response.status().is_success() {
                continue;
            }
            let commits: Vec<GithubCommitWrapper> = response.json()?;
            for commit in commits {
                if let Some(author) = commit.commit.author {
                    if let Some(email) = author.email {
                        emails.insert(email);
                    }
                }
                if let Some(committer) = commit.commit.committer {
                    if let Some(email) = committer.email {
                        emails.insert(email);
                    }
                }
            }
        }
        page += 1;
    }

    Ok(emails)
}
