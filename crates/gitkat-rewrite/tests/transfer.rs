use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use anyhow::{anyhow, Context, Result};
use gitkat_rewrite::{gix_export, gix_import};
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

fn git_fast_export(repo: &Path) -> Result<Vec<u8>> {
    let output = run_git(["fast-export", "--all"], repo)?;
    Ok(output.stdout)
}

fn git_fast_import(repo: &Path, data: &[u8]) -> Result<()> {
    let mut child = Command::new("git")
        .arg("fast-import")
        .arg("--quiet")
        .current_dir(repo)
        .stdin(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn git fast-import in {}", repo.display()))?;
    {
        let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("Missing stdin"))?;
        stdin.write_all(data)?;
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow!("git fast-import failed in {}", repo.display()));
    }
    Ok(())
}

fn setup_repo(repo: &Path) -> Result<()> {
    fs::create_dir_all(repo)?;
    run_git(["init", "-b", "main"], repo)?;
    run_git(["config", "user.name", "Test User"], repo)?;
    run_git(["config", "user.email", "test@example.com"], repo)?;

    fs::write(repo.join("README.md"), "hello\n")?;
    fs::write(repo.join("data.bin"), [0u8, 1, 2, 3, 4])?;
    run_git(["add", "."], repo)?;
    run_git(["commit", "-m", "initial"], repo)?;

    run_git(["checkout", "-b", "feature"], repo)?;
    fs::write(repo.join("feature.txt"), "feature\n")?;
    run_git(["add", "."], repo)?;
    run_git(["commit", "-m", "feature work"], repo)?;

    run_git(["checkout", "main"], repo)?;
    fs::write(repo.join("main.txt"), "main\n")?;
    run_git(["add", "."], repo)?;
    run_git(["commit", "-m", "main work"], repo)?;

    run_git(["merge", "--no-ff", "feature", "-m", "merge feature"], repo)?;
    run_git(["tag", "-a", "v1.0.0", "-m", "release"], repo)?;
    run_git(["branch", "dev", "HEAD~1"], repo)?;
    Ok(())
}

#[test]
fn gix_export_import_matches_git_fast_export() -> Result<()> {
    let temp = TempDir::new()?;
    let source = temp.path().join("source");
    setup_repo(&source)?;

    let git_export = git_fast_export(&source)?;

    let mut gix_stream = Vec::new();
    gix_export(&source, &mut gix_stream)?;

    let target = temp.path().join("gix-import");
    gix_import(&target, gix_stream.as_slice())?;
    let gix_exported = git_fast_export(&target)?;

    assert_eq!(git_export, gix_exported);
    Ok(())
}

#[test]
fn gix_import_matches_git_fast_import() -> Result<()> {
    let temp = TempDir::new()?;
    let source = temp.path().join("source");
    setup_repo(&source)?;

    let git_export = git_fast_export(&source)?;

    let git_target = temp.path().join("git-import");
    fs::create_dir_all(&git_target)?;
    run_git(["init", "-b", "main"], &git_target)?;
    git_fast_import(&git_target, &git_export)?;
    let git_import_exported = git_fast_export(&git_target)?;

    let mut gix_stream = Vec::new();
    gix_export(&source, &mut gix_stream)?;
    let gix_target = temp.path().join("gix-import");
    gix_import(&gix_target, gix_stream.as_slice())?;
    let gix_import_exported = git_fast_export(&gix_target)?;

    assert_eq!(git_export, git_import_exported);
    assert_eq!(git_export, gix_import_exported);
    Ok(())
}
