use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use anyhow::{anyhow, Context, Result};
use gitkat_rewrite::{rewrite_repo, RewriteConfig};
use tempfile::TempDir;

fn run_git<I, S>(args: I, cwd: &Path) -> Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("Failed to spawn git in {}", cwd.display()))?;
    if !output.status.success() {
        return Err(anyhow!(
            "git failed in {}: {}",
            cwd.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output)
}

fn init_repo(repo: &Path) -> Result<()> {
    fs::create_dir_all(repo)?;
    run_git(["init", "-b", "main"], repo)?;
    run_git(["config", "user.name", "Test User"], repo)?;
    run_git(["config", "user.email", "test@example.com"], repo)?;
    Ok(())
}

fn commit_all(repo: &Path, message: &str) -> Result<()> {
    run_git(["add", "."], repo)?;
    run_git(["commit", "-m", message], repo)?;
    Ok(())
}

fn commit_remove(repo: &Path, message: &str) -> Result<()> {
    run_git(["add", "-u"], repo)?;
    run_git(["commit", "-m", message], repo)?;
    Ok(())
}

fn assert_no_history_for_path(repo: &Path, path: &str) -> Result<()> {
    let output = run_git(["rev-list", "--all", "--", path], repo)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        return Err(anyhow!("Path still present in history: {path}"));
    }
    Ok(())
}

#[test]
fn delete_path_removes_history() -> Result<()> {
    let temp = TempDir::new()?;
    let repo = temp.path().join("repo");
    init_repo(&repo)?;

    fs::create_dir_all(repo.join("ml/res"))?;
    fs::write(
        repo.join("ml/res/wily_graph_cyclomatic.html"),
        "<html>report</html>\n",
    )?;
    fs::write(repo.join("keep.txt"), "keep v1\n")?;
    commit_all(&repo, "add report")?;

    fs::write(repo.join("keep.txt"), "keep v2\n")?;
    commit_all(&repo, "update keep")?;

    fs::remove_file(repo.join("ml/res/wily_graph_cyclomatic.html"))?;
    commit_remove(&repo, "remove report")?;

    let config = RewriteConfig {
        delete_paths: vec!["ml/res/wily_graph_cyclomatic.html".to_string()],
        quiet: true,
        ..Default::default()
    };
    rewrite_repo(&repo, &config)?;

    assert_no_history_for_path(&repo, "ml/res/wily_graph_cyclomatic.html")?;

    let output = run_git(["show", "HEAD:keep.txt"], &repo)?;
    let content = String::from_utf8_lossy(&output.stdout);
    assert_eq!(content, "keep v2\n");
    Ok(())
}

#[test]
fn delete_path_glob_matches_multiple_locations() -> Result<()> {
    let temp = TempDir::new()?;
    let repo = temp.path().join("repo");
    init_repo(&repo)?;

    fs::create_dir_all(repo.join("ml/res"))?;
    fs::create_dir_all(repo.join("reports"))?;
    fs::write(
        repo.join("ml/res/wily_graph_cyclomatic.html"),
        "ml report\n",
    )?;
    fs::write(repo.join("reports/wily_graph_cyclomatic.html"), "reports\n")?;
    fs::write(repo.join("keep.txt"), "keep\n")?;
    commit_all(&repo, "add reports")?;

    let config = RewriteConfig {
        delete_paths: vec!["**/wily_graph_cyclomatic.html".to_string()],
        quiet: true,
        ..Default::default()
    };
    rewrite_repo(&repo, &config)?;

    assert_no_history_for_path(&repo, "ml/res/wily_graph_cyclomatic.html")?;
    assert_no_history_for_path(&repo, "reports/wily_graph_cyclomatic.html")?;
    Ok(())
}
