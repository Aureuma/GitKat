use std::collections::{BTreeSet, HashMap};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Args, Parser, Subcommand};
use gitkat_rewrite::{gix_export, gix_import, rewrite_repo, RewriteConfig};
use serde::Deserialize;
use walkdir::WalkDir;

mod verify;
use crate::verify::{run_verify, VerifyArgs};

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
    FastExport(FastExportArgs),
    FastImport(FastImportArgs),
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
    #[arg(short = 'd', long = "delete-path", action = ArgAction::Append)]
    delete_paths: Vec<String>,
    #[arg(long = "regex")]
    regex_map: bool,
    #[arg(long)]
    rename_files: bool,
    #[arg(long)]
    preserve_case: bool,
    #[arg(long, short = 'i')]
    ignore_case: bool,
    #[arg(long, short = 'q', action = ArgAction::SetTrue, alias = "no-log")]
    quiet: bool,
}

#[derive(Args)]
struct FastExportArgs {
    #[arg(long)]
    repo: Option<PathBuf>,
    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Args)]
struct FastImportArgs {
    #[arg(long)]
    repo: PathBuf,
    #[arg(long)]
    input: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct RewriteOptions {
    new_name: String,
    new_email: String,
    old_name: String,
    old_emails: Vec<String>,
    blob_map: Vec<String>,
    exclude_patterns: Vec<String>,
    delete_paths: Vec<String>,
    regex_map: bool,
    preserve_case: bool,
    ignore_case: bool,
    rename_files: bool,
    quiet: bool,
}

#[derive(Clone, Debug)]
struct RemoteConfig {
    name: String,
    fetch_urls: Vec<String>,
    push_urls: Vec<String>,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Check { name } => run_check(&name, None),
        Commands::Report { path } => run_report(path.as_deref()),
        Commands::Push => run_push(None),
        Commands::Rewrite(args) => run_rewrite(args, None),
        Commands::FastExport(args) => run_fast_export(args),
        Commands::FastImport(args) => run_fast_import(args),
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
    let base = base_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let mut repos = list_child_repos(&base)?;
    if repos.is_empty() {
        if let Some(root) = find_git_root(&base) {
            repos.push(root);
        }
    }
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
            println!(
                "Found commits with '{}' in author or committer fields.",
                name
            );
        } else {
            println!("Nothing.");
        }
        println!();
    }
    Ok(0)
}

fn run_report(base_dir: Option<&Path>) -> Result<i32> {
    let base = base_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
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
    let base = base_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let repos = list_child_repos(&base)?;
    if repos.is_empty() {
        println!("No git repositories found in {}.", base.display());
        return Ok(1);
    }

    println!(
        "== Force-pushing current branches of all repos in {} ==",
        base.display()
    );
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
        delete_paths: split_comma_args(&args.delete_paths),
        regex_map: args.regex_map,
        preserve_case: args.preserve_case,
        ignore_case: args.ignore_case,
        rename_files: args.rename_files,
        quiet: args.quiet,
    };

    if opts.old_emails.is_empty() && opts.blob_map.is_empty() && opts.delete_paths.is_empty() {
        println!("Error: specify at least one identity rewrite (-o/-e), blob data mapping (-m), or delete path (-d/--delete-path).");
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

    let base = base_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
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

fn run_fast_export(args: FastExportArgs) -> Result<i32> {
    let repo = args
        .repo
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let mut writer: Box<dyn io::Write> = if let Some(path) = args.output {
        Box::new(BufWriter::new(fs::File::create(path)?))
    } else {
        Box::new(BufWriter::new(io::stdout()))
    };
    gix_export(&repo, &mut writer)?;
    writer.flush()?;
    Ok(0)
}

fn run_fast_import(args: FastImportArgs) -> Result<i32> {
    let mut reader: Box<dyn io::Read> = if let Some(path) = args.input {
        Box::new(BufReader::new(fs::File::open(path)?))
    } else {
        Box::new(BufReader::new(io::stdin()))
    };
    gix_import(&args.repo, &mut reader)?;
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
    println!(
        "Found {} organization repositories where you have push access",
        org_repos.len()
    );

    let all_repos = user_repos.into_iter().chain(org_repos).collect::<Vec<_>>();
    println!(
        "\nAnalyzing contributions across {} repositories...",
        all_repos.len()
    );

    let mut all_emails = BTreeSet::new();
    let mut repo_emails: HashMap<String, BTreeSet<String>> = HashMap::new();

    for (idx, repo) in all_repos.iter().enumerate() {
        let repo_owner = &repo.owner.login;
        let repo_name = &repo.name;
        println!(
            "[{}/{}] Checking {}/{}...",
            idx + 1,
            all_repos.len(),
            repo_owner,
            repo_name
        );
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

fn run_gix_rewrite(repo: &Path, options: &RewriteOptions) -> Result<()> {
    let config = RewriteConfig {
        new_name: to_opt(&options.new_name),
        new_email: to_opt(&options.new_email),
        old_name: to_opt(&options.old_name),
        old_emails: options.old_emails.clone(),
        blob_map: options.blob_map.clone(),
        exclude_patterns: options.exclude_patterns.clone(),
        delete_paths: options.delete_paths.clone(),
        regex_map: options.regex_map,
        preserve_case: options.preserve_case,
        ignore_case: options.ignore_case,
        rename_files: options.rename_files,
        quiet: options.quiet,
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
    println!("Deleted paths:              {}", opts.delete_paths.len());
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
        gitkat_rewrite::parse_mapping(entry)?;
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
        let fetch_urls =
            git_output_or_empty(["config", "--get-all", &format!("remote.{name}.url")], repo)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(|line| line.to_string())
                .collect::<Vec<_>>();
        let push_urls = git_output_or_empty(
            ["config", "--get-all", &format!("remote.{name}.pushurl")],
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
                run_git(
                    ["remote", "set-url", "--add", &remote.name, extra],
                    repo,
                    false,
                )?;
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
    if base_dir.join(".git").exists() {
        repos.push(base_dir.to_path_buf());
    }
    for entry in fs::read_dir(base_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join(".git").exists() {
            repos.push(path);
        }
    }
    repos.sort();
    repos.dedup();
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
    let args_vec: Vec<OsString> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect();
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
    let args_vec: Vec<OsString> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect();
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
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).to_string()
        }
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
            .query(&[
                ("per_page", "100"),
                ("page", &page.to_string()),
                ("affiliation", "owner"),
            ])
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
            .query(&[
                ("author", username),
                ("per_page", "100"),
                ("page", &page.to_string()),
            ])
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
            .query(&[
                ("state", "all"),
                ("per_page", "100"),
                ("page", &page.to_string()),
            ])
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
