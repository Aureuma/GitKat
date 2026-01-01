use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context, Result};
use clap::Args;
use gitkat_rewrite::{rewrite_repo, RewriteConfig};
use regex::Regex;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

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
regex_map = os.environ.get("GITKIT_REGEX_MAP", "0") == "1"
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
        if regex_map:
            state["patterns"] = [(re.compile(old, re_flags), new) for old, new in pairs]
        else:
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

#[derive(Args)]
pub(crate) struct VerifyArgs {
    #[arg(long)]
    workdir: Option<PathBuf>,
    #[arg(long)]
    keep_workdir: bool,
    #[arg(long)]
    with_blob: bool,
    #[arg(long)]
    with_regex: bool,
    #[arg(long)]
    with_bfg: bool,
    #[arg(long)]
    bfg_jar: Option<PathBuf>,
    #[arg(long)]
    ci: bool,
    repos: Vec<String>,
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
    regex_map: bool,
}

pub(crate) fn run_verify(args: VerifyArgs) -> Result<i32> {
    let VerifyArgs {
        workdir,
        keep_workdir,
        with_blob,
        with_regex,
        with_bfg,
        bfg_jar,
        ci,
        repos,
    } = args;
    if !has_filter_repo() {
        println!("git-filter-repo is required for verification");
        return Ok(1);
    }

    if !repos.is_empty() && ci {
        println!("Provide explicit repos or use --ci, not both");
        return Ok(1);
    }

    let repos: Vec<String> = if !repos.is_empty() {
        repos
    } else if ci {
        CI_REPOS.iter().map(|repo| repo.to_string()).collect()
    } else {
        REPOS.iter().map(|repo| repo.to_string()).collect()
    };

    if repos.is_empty() {
        println!("No repositories provided");
        return Ok(1);
    }

    let with_blob = with_blob || with_regex || with_bfg;
    let bfg_jar = if with_bfg {
        Some(resolve_bfg_jar(bfg_jar)?)
    } else {
        None
    };

    let mut temp_dir: Option<TempDir> = None;
    let workdir = if let Some(workdir) = workdir {
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
        verify_repo(
            &url,
            &workdir,
            with_blob,
            with_regex,
            with_bfg,
            bfg_jar.as_deref(),
        )?;
        println!("OK: {}", url);
    }

    if keep_workdir {
        if let Some(temp_dir) = temp_dir.take() {
            let _ = temp_dir.keep();
        }
    }

    Ok(0)
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

fn resolve_bfg_jar(bfg_jar: Option<PathBuf>) -> Result<PathBuf> {
    let candidate = if let Some(path) = bfg_jar {
        path
    } else if let Ok(env_path) = env::var("BFG_JAR") {
        PathBuf::from(env_path)
    } else {
        return Err(anyhow!("Provide --bfg-jar or set BFG_JAR for --with-bfg"));
    };

    if !candidate.is_file() {
        return Err(anyhow!("BFG jar not found at {}", candidate.display()));
    }
    Ok(candidate)
}

fn reset_dir(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).with_context(|| format!("Failed to remove {}", path.display()))?;
    }
    Ok(())
}

fn verify_with_filter_repo(
    source: &Path,
    workdir: &Path,
    name: &str,
    label: &str,
    options: &VerifyOptions,
) -> Result<()> {
    let gix_repo = workdir.join(format!("{name}-{label}-gix"));
    let filter_repo = workdir.join(format!("{name}-{label}-filter"));
    reset_dir(&gix_repo)?;
    reset_dir(&filter_repo)?;
    clone_local(source, &gix_repo, workdir)?;
    clone_local(source, &filter_repo, workdir)?;

    run_gitkat(&gix_repo, options)?;
    run_filter_repo(&filter_repo, options)?;

    let gix_hash = fast_export_hash(&gix_repo)?;
    let filter_hash = fast_export_hash(&filter_repo)?;
    if gix_hash != filter_hash {
        return Err(anyhow!(
            "Mismatch for {name} ({label}): {gix_hash} != {filter_hash}"
        ));
    }

    Ok(())
}

fn verify_with_bfg(
    source: &Path,
    workdir: &Path,
    name: &str,
    options: &VerifyOptions,
    bfg_jar: &Path,
) -> Result<()> {
    if options.blob_map.is_empty() {
        return Ok(());
    }
    if options.regex_map || options.ignore_case || options.preserve_case || options.rename_files {
        return Err(anyhow!("BFG comparison only supports literal blob maps"));
    }

    let gix_repo = workdir.join(format!("{name}-bfg-gix"));
    let bfg_repo = workdir.join(format!("{name}-bfg"));
    reset_dir(&gix_repo)?;
    reset_dir(&bfg_repo)?;
    clone_mirror(source, &gix_repo, workdir)?;
    clone_mirror(source, &bfg_repo, workdir)?;

    let blob_only = VerifyOptions {
        new_name: String::new(),
        new_email: String::new(),
        old_email: String::new(),
        old_name: String::new(),
        blob_map: options.blob_map.clone(),
        exclude: Vec::new(),
        ignore_case: false,
        preserve_case: false,
        rename_files: false,
        regex_map: false,
    };
    run_gitkat(&gix_repo, &blob_only)?;
    run_bfg(&bfg_repo, &options.blob_map, bfg_jar)?;

    verify_blob_replacements(&gix_repo, &options.blob_map, "gitkat")?;
    verify_blob_replacements(&bfg_repo, &options.blob_map, "bfg")?;

    Ok(())
}

fn verify_repo(
    url: &str,
    workdir: &Path,
    with_blob: bool,
    with_regex: bool,
    with_bfg: bool,
    bfg_jar: Option<&Path>,
) -> Result<()> {
    let name = url
        .rsplit('/')
        .next()
        .unwrap_or(url)
        .trim_end_matches(".git");
    let source = workdir.join(name);
    reset_dir(&source)?;
    clone_repo(url, &source, workdir)?;

    let (_old_author_name, old_email, old_name) = pick_identity(&source)?;
    let blob_map = if with_blob {
        pick_blob_map(&source)?
    } else {
        Vec::new()
    };
    let base_options = VerifyOptions {
        new_name: "GitKat Rewrite".to_string(),
        new_email: "rewrite@example.test".to_string(),
        old_email,
        old_name,
        blob_map,
        exclude: Vec::new(),
        ignore_case: false,
        preserve_case: false,
        rename_files: false,
        regex_map: false,
    };

    verify_with_filter_repo(&source, workdir, name, "literal", &base_options)?;

    if with_regex {
        let regex_options = VerifyOptions {
            blob_map: regexify_blob_map(&base_options.blob_map),
            regex_map: true,
            ..base_options.clone()
        };
        verify_with_filter_repo(&source, workdir, name, "regex", &regex_options)?;
    }

    if with_bfg {
        let bfg_jar = bfg_jar.ok_or_else(|| anyhow!("BFG jar path is required for --with-bfg"))?;
        verify_with_bfg(&source, workdir, name, &base_options, bfg_jar)?;
    }

    Ok(())
}

fn run_gitkat(repo: &Path, options: &VerifyOptions) -> Result<()> {
    let remotes = crate::capture_remotes(repo)?;
    let config = RewriteConfig {
        new_name: crate::to_opt(&options.new_name),
        new_email: crate::to_opt(&options.new_email),
        old_name: crate::to_opt(&options.old_name),
        old_emails: vec![options.old_email.clone()],
        blob_map: options.blob_map.clone(),
        exclude_patterns: options.exclude.clone(),
        delete_paths: Vec::new(),
        regex_map: options.regex_map,
        preserve_case: options.preserve_case,
        ignore_case: options.ignore_case,
        rename_files: options.rename_files,
        quiet: true,
    };
    rewrite_repo(repo, &config)?;
    crate::restore_remotes(repo, &remotes)?;
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
        "GITKIT_REGEX_MAP".to_string(),
        if options.regex_map { "1" } else { "0" }.to_string(),
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
        let (old, new) = gitkat_rewrite::parse_mapping(entry)?;
        lines.push(format!("{}\t{}", old, new));
    }
    Ok(lines.join("\n"))
}

fn regexify_blob_map(entries: &[String]) -> Vec<String> {
    entries
        .iter()
        .filter_map(|entry| {
            let (old, new) = gitkat_rewrite::parse_mapping(entry).ok()?;
            let escaped = regex::escape(&old);
            Some(format!("{escaped}:{new}"))
        })
        .collect()
}

fn build_bfg_replace_text(entries: &[String]) -> Result<String> {
    let mut lines = Vec::new();
    for entry in entries {
        let (old, new) = gitkat_rewrite::parse_mapping(entry)?;
        lines.push(format!("{old}==>{new}"));
    }
    Ok(lines.join("\n"))
}

fn run_bfg(repo: &Path, blob_map: &[String], bfg_jar: &Path) -> Result<()> {
    let replacements = build_bfg_replace_text(blob_map)?;
    let temp_dir = TempDir::new()?;
    let replace_path = temp_dir.path().join("bfg-replacements.txt");
    fs::write(&replace_path, replacements)?;

    let status = Command::new("java")
        .arg("-jar")
        .arg(bfg_jar)
        .arg("--replace-text")
        .arg(&replace_path)
        .arg("--no-blob-protection")
        .arg(repo)
        .current_dir(repo)
        .status()
        .context("Failed to run BFG")?;
    if !status.success() {
        return Err(anyhow!("BFG failed"));
    }

    crate::run_git_status(["reflog", "expire", "--expire=now", "--all"], repo)?;
    crate::run_git_status(["gc", "--prune=now", "--aggressive"], repo)?;
    Ok(())
}

fn clone_repo(url: &str, dest: &Path, workdir: &Path) -> Result<()> {
    crate::run_git(
        ["clone", "--quiet", url, dest.to_str().unwrap_or("")],
        workdir,
        true,
    )?;
    Ok(())
}

fn clone_local(source: &Path, dest: &Path, workdir: &Path) -> Result<()> {
    crate::run_git(
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

fn clone_mirror(source: &Path, dest: &Path, workdir: &Path) -> Result<()> {
    crate::run_git(
        [
            "clone",
            "--quiet",
            "--mirror",
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
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Missing git fast-export stdout"))?;
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
    let line = crate::git_output(["log", "-n", "1", "--format=%an%x00%ae"], repo)?;
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

fn verify_blob_replacements(repo: &Path, blob_map: &[String], label: &str) -> Result<()> {
    if blob_map.is_empty() {
        return Ok(());
    }

    let mut rules = Vec::new();
    for entry in blob_map {
        let (old, new) = gitkat_rewrite::parse_mapping(entry)?;
        rules.push((old.into_bytes(), new.into_bytes()));
    }

    let output = crate::git_output(["rev-list", "--all", "--objects"], repo)?;
    let mut blob_ids = Vec::new();
    for line in output.lines() {
        if let Some((id, path)) = line.split_once(' ') {
            if !path.trim().is_empty() {
                blob_ids.push(id.to_string());
            }
        }
    }
    if blob_ids.is_empty() {
        return Ok(());
    }

    let mut found_old = vec![false; rules.len()];
    let mut found_new = rules
        .iter()
        .map(|(_, new)| new.is_empty())
        .collect::<Vec<_>>();

    let mut child = Command::new("git")
        .arg("cat-file")
        .arg("--batch")
        .current_dir(repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to spawn git cat-file")?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Missing cat-file stdin"))?;
        for id in &blob_ids {
            stdin.write_all(id.as_bytes())?;
            stdin.write_all(b"\n")?;
        }
    }

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Missing cat-file stdout"))?;
    for _ in 0..blob_ids.len() {
        let mut header = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            if stdout.read_exact(&mut byte).is_err() {
                return Err(anyhow!("Unexpected end of cat-file output"));
            }
            if byte[0] == b'\n' {
                break;
            }
            header.push(byte[0]);
        }
        let header_str = String::from_utf8_lossy(&header);
        let mut header_parts = header_str.split_whitespace();
        let _id = header_parts.next().unwrap_or_default();
        let kind = header_parts.next().unwrap_or_default();
        let size = header_parts
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);

        if kind != "blob" {
            continue;
        }

        let mut data = vec![0u8; size];
        stdout.read_exact(&mut data)?;
        stdout.read_exact(&mut byte)?;

        for (idx, (old, new)) in rules.iter().enumerate() {
            if !found_old[idx] && data.windows(old.len()).any(|win| win == old.as_slice()) {
                found_old[idx] = true;
            }
            if !found_new[idx]
                && !new.is_empty()
                && data.windows(new.len()).any(|win| win == new.as_slice())
            {
                found_new[idx] = true;
            }
        }
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow!("git cat-file --batch failed"));
    }

    for (idx, (_old, _new)) in rules.iter().enumerate() {
        if found_old[idx] {
            return Err(anyhow!(
                "Old blob token still present after {label} rewrite"
            ));
        }
        if !found_new[idx] {
            return Err(anyhow!("New blob token missing after {label} rewrite"));
        }
    }

    Ok(())
}
