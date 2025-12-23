use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use gix::bstr::{BStr, BString};
use gix::hash::ObjectId;
use gix::object::tree::EntryKind;
use gix::actor::SignatureRef;
use gix::Reference;
use gix_object::{CommitRef, Kind as ObjectKind, Tree as TreeObject, Write};
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::bytes::{Regex, RegexBuilder};

const CTX_WORDS: usize = 2;
const COLOR_PATH: &str = "\x1b[95m";
const COLOR_MATCH: &str = "\x1b[31m";
const COLOR_REPL: &str = "\x1b[34m";
const COLOR_RESET: &str = "\x1b[0m";

fn commit_id_regex() -> &'static Regex {
    static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"[0-9a-f]{7,40}").expect("valid commit-id regex"))
}

#[derive(Clone, Debug, Default)]
pub struct RewriteConfig {
    pub new_name: Option<String>,
    pub new_email: Option<String>,
    pub old_name: Option<String>,
    pub old_emails: Vec<String>,
    pub blob_map: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub delete_paths: Vec<String>,
    pub preserve_case: bool,
    pub ignore_case: bool,
    pub rename_files: bool,
}

#[derive(Clone, Debug)]
pub struct RewriteStats {
    pub commits: usize,
    pub tags: usize,
}

#[derive(Clone, Debug)]
struct Pattern {
    regex: Regex,
    replacement: Vec<u8>,
}

#[derive(Debug)]
struct Options {
    new_name: Option<BString>,
    new_email: Option<BString>,
    old_name: Option<String>,
    old_emails: HashSet<String>,
    patterns: Vec<Pattern>,
    exclude: Option<GlobSet>,
    delete_paths: HashSet<String>,
    preserve_case: bool,
    rename_files: bool,
}

impl Options {
    fn from_config(config: &RewriteConfig) -> Result<Self> {
        let patterns = parse_patterns(&config.blob_map, config.ignore_case)?;
        let exclude = build_exclude_set(&config.exclude_patterns)?;
        let old_emails = config
            .old_emails
            .iter()
            .map(|email| email.trim().to_lowercase())
            .filter(|email| !email.is_empty())
            .collect::<HashSet<_>>();
        let old_name = config
            .old_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(|name| name.to_lowercase());
        let delete_paths = config
            .delete_paths
            .iter()
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty())
            .collect::<HashSet<_>>();
        let new_name = config
            .new_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(|name| BString::from(name.as_bytes()));
        let new_email = config
            .new_email
            .as_deref()
            .map(str::trim)
            .filter(|email| !email.is_empty())
            .map(|email| BString::from(email.as_bytes()));
        Ok(Self {
            new_name,
            new_email,
            old_name,
            old_emails,
            patterns,
            exclude,
            delete_paths,
            preserve_case: config.preserve_case,
            rename_files: config.rename_files,
        })
    }
}

fn parse_patterns(entries: &[String], ignore_case: bool) -> Result<Vec<Pattern>> {
    let mut patterns = Vec::new();
    for entry in entries {
        let (old, new) = entry
            .split_once(':')
            .ok_or_else(|| anyhow!("Invalid mapping '{entry}', expected old:new"))?;
        let escaped = regex::escape(old);
        let mut builder = RegexBuilder::new(&escaped);
        builder.case_insensitive(ignore_case).unicode(false);
        let regex = builder
            .build()
            .with_context(|| format!("Invalid regex for '{old}'"))?;
        patterns.push(Pattern {
            regex,
            replacement: new.as_bytes().to_vec(),
        });
    }
    Ok(patterns)
}

fn build_exclude_set(patterns: &[String]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if pattern.trim().is_empty() {
            continue;
        }
        builder.add(Glob::new(pattern)?);
    }
    Ok(Some(builder.build()?))
}

pub fn rewrite_repo(repo_path: &Path, config: &RewriteConfig) -> Result<RewriteStats> {
    let options = Options::from_config(config)?;
    let repo = gix::open(repo_path).with_context(|| format!("Open repo at {}", repo_path.display()))?;
    let tips = collect_tips(&repo)?;
    if tips.is_empty() {
        return Err(anyhow!("No references found to rewrite"));
    }

    let mut commit_map = HashMap::<ObjectId, ObjectId>::new();
    let mut commit_hex = Vec::<(String, String)>::new();
    let mut tag_map = HashMap::<ObjectId, ObjectId>::new();
    let commit_ids = collect_commits(&repo, tips)?;
    for commit_id in commit_ids {
        let new_id = rewrite_commit(&repo, commit_id, &options, &commit_map, &commit_hex)?;
        commit_map.insert(commit_id, new_id);
        commit_hex.push((commit_id.to_string(), new_id.to_string()));
    }

    rewrite_references(&repo, &commit_map, &mut tag_map)?;
    delete_remote_refs(&repo)?;
    Ok(RewriteStats {
        commits: commit_map.len(),
        tags: tag_map.len(),
    })
}

fn collect_tips(repo: &gix::Repository) -> Result<Vec<ObjectId>> {
    let platform = repo.references()?;
    let mut tips = Vec::new();
    for reference in platform.all()? {
        let mut reference = reference.map_err(|err| anyhow!(err.to_string()))?;
        let name = reference.name().as_bstr();
        if name.starts_with(b"refs/original/") || name.starts_with(b"refs/replace/") {
            continue;
        }
        if let Ok(commit) = reference.peel_to_commit() {
            tips.push(commit.id);
        }
    }
    tips.sort();
    tips.dedup();
    Ok(tips)
}

fn collect_commits(repo: &gix::Repository, _tips: Vec<ObjectId>) -> Result<Vec<ObjectId>> {
    let workdir = repo.workdir().map(PathBuf::from).unwrap_or_else(|| repo.path().to_path_buf());
    let output = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .arg("rev-list")
        .arg("--reverse")
        .arg("--topo-order")
        .arg("--all")
        .output()
        .context("Failed to run git rev-list")?;
    if !output.status.success() {
        return Err(anyhow!("git rev-list failed"));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();
    for line in stdout.lines() {
        let id = ObjectId::from_hex(line.as_bytes())
            .with_context(|| format!("Invalid commit id from rev-list: {line}"))?;
        commits.push(id);
    }
    Ok(commits)
}

fn rewrite_commit(
    repo: &gix::Repository,
    commit_id: ObjectId,
    options: &Options,
    commit_map: &HashMap<ObjectId, ObjectId>,
    commit_hex: &[(String, String)],
) -> Result<ObjectId> {
    let object = repo.find_object(commit_id)?;
    if object.kind != ObjectKind::Commit {
        return Err(anyhow!("Expected commit object, got {:?}", object.kind));
    }
    let commit_ref = CommitRef::from_bytes(&object.data)?;

    let author_sig = SignatureRef::from_bytes::<gix_object::decode::ParseError>(commit_ref.author.as_ref())
        .map_err(|err| anyhow!("Invalid author signature: {err:?}"))?;
    let committer_sig =
        SignatureRef::from_bytes::<gix_object::decode::ParseError>(commit_ref.committer.as_ref())
            .map_err(|err| anyhow!("Invalid committer signature: {err:?}"))?;

    let (author_line, _author_changed) = rewrite_signature_line(commit_ref.author, author_sig, options)?;
    let (committer_line, _committer_changed) =
        rewrite_signature_line(commit_ref.committer, committer_sig, options)?;

    let mut message = commit_ref.message.to_owned();
    message = rewrite_commit_ids(message, commit_map, commit_hex);

    let tree_id = ObjectId::from_hex(commit_ref.tree.as_ref())?;
    let new_tree = rewrite_tree(repo, tree_id, options)?;

    let mut parents = Vec::with_capacity(commit_ref.parents.len());
    for parent_hex in &commit_ref.parents {
        let parent_id = ObjectId::from_hex(parent_hex.as_ref())?;
        let new_parent = commit_map
            .get(&parent_id)
            .copied()
            .ok_or_else(|| anyhow!("Missing rewritten parent {parent_id}"))?;
        parents.push(new_parent);
    }

    let commit_bytes = build_commit_bytes(
        new_tree,
        &parents,
        &author_line,
        &committer_line,
        commit_ref.encoding,
        &message,
    );
    let new_id = repo
        .write_buf(ObjectKind::Commit, &commit_bytes)
        .map_err(|err| anyhow!(err))?;
    Ok(new_id)
}

fn rewrite_commit_ids(
    message: BString,
    commit_map: &HashMap<ObjectId, ObjectId>,
    commit_hex: &[(String, String)],
) -> BString {
    if commit_map.is_empty() {
        return message;
    }
    let replaced = commit_id_regex().replace_all(message.as_ref(), |caps: &regex::bytes::Captures| {
        let bytes = caps.get(0).expect("capture").as_bytes();
        if bytes.len() == 40 {
            let Ok(old_id) = ObjectId::from_hex(bytes) else {
                return Cow::Owned(bytes.to_vec());
            };
            return if let Some(new_id) = commit_map.get(&old_id) {
                Cow::Owned(new_id.to_string().into_bytes())
            } else {
                Cow::Owned(bytes.to_vec())
            };
        }

        let Ok(needle) = std::str::from_utf8(bytes) else {
            return Cow::Owned(bytes.to_vec());
        };
        let mut replacement: Option<&str> = None;
        for (old_hex, new_hex) in commit_hex {
            if old_hex.starts_with(needle) {
                if replacement.is_some() {
                    return Cow::Owned(bytes.to_vec());
                }
                replacement = Some(new_hex);
            }
        }
        if let Some(new_hex) = replacement {
            Cow::Owned(new_hex[..needle.len()].as_bytes().to_vec())
        } else {
            Cow::Owned(bytes.to_vec())
        }
    });
    BString::from(replaced.into_owned())
}

fn rewrite_signature_line<'a>(
    raw: &'a BStr,
    signature: SignatureRef<'a>,
    options: &Options,
) -> Result<(Cow<'a, [u8]>, bool)> {
    if options.old_emails.is_empty() || options.new_email.is_none() {
        return Ok((Cow::Borrowed(raw.as_ref()), false));
    }
    let trimmed = signature.trim();
    let email_lower = lower_bytes(trimmed.email.as_ref());
    let name_lower = lower_bytes(trimmed.name.as_ref());
    let matches_email = options.old_emails.contains(&email_lower);
    let matches_name = options
        .old_name
        .as_ref()
        .map(|name| name == &name_lower)
        .unwrap_or(true);
    if !(matches_email && matches_name) {
        return Ok((Cow::Borrowed(raw.as_ref()), false));
    }

    let mut out = Vec::new();
    if let Some(name) = &options.new_name {
        out.extend_from_slice(name.as_ref());
    } else {
        out.extend_from_slice(signature.name.as_ref());
    }
    out.extend_from_slice(b" <");
    out.extend_from_slice(options.new_email.as_ref().expect("new email").as_ref());
    out.extend_from_slice(b"> ");
    out.extend_from_slice(signature.time.as_bytes());
    Ok((Cow::Owned(out), true))
}

fn lower_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_lowercase()
}

fn build_commit_bytes(
    tree: ObjectId,
    parents: &[ObjectId],
    author: &[u8],
    committer: &[u8],
    encoding: Option<&BStr>,
    message: &BString,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"tree ");
    out.extend_from_slice(tree.to_string().as_bytes());
    out.push(b'\n');
    for parent in parents {
        out.extend_from_slice(b"parent ");
        out.extend_from_slice(parent.to_string().as_bytes());
        out.push(b'\n');
    }
    out.extend_from_slice(b"author ");
    out.extend_from_slice(author);
    out.push(b'\n');
    out.extend_from_slice(b"committer ");
    out.extend_from_slice(committer);
    out.push(b'\n');
    if let Some(encoding) = encoding {
        out.extend_from_slice(b"encoding ");
        out.extend_from_slice(encoding.as_ref());
        out.push(b'\n');
    }
    out.push(b'\n');
    out.extend_from_slice(message.as_ref());
    out
}

fn rewrite_tree(repo: &gix::Repository, tree_id: ObjectId, options: &Options) -> Result<ObjectId> {
    if options.patterns.is_empty() && !options.rename_files && options.delete_paths.is_empty() {
        return Ok(tree_id);
    }

    let tree = repo.find_tree(tree_id)?;
    let mut editor = tree.edit()?;
    let mut path = Vec::new();
    let mut changed = false;

    walk_tree(repo, &tree, &mut path, &mut |path_bytes, entry| {
        let mut path_str = String::from_utf8_lossy(path_bytes).to_string();
        if options.delete_paths.contains(&path_str) {
            editor.remove(BString::from(path_bytes))?;
            changed = true;
            return Ok(());
        }
        if let Some(exclude) = &options.exclude {
            if exclude.is_match(path_str.as_str()) {
                return Ok(());
            }
        }

        let mut new_path = path_bytes.to_vec();
        if options.rename_files {
            if let Some(updated) = apply_patterns(&new_path, options) {
                if updated != new_path {
                    let new_path_str = String::from_utf8_lossy(&updated).to_string();
                    log_rename(&path_str, &new_path_str);
                    new_path = updated;
                    path_str = new_path_str;
                }
            }
        }

        if entry.mode().is_blob() {
            let entry_id = entry.oid().to_owned();
            let blob = repo.find_blob(entry_id)?;
            if is_binary(&blob.data) {
                if new_path != path_bytes {
                    editor.remove(BString::from(path_bytes))?;
                    editor.upsert(BString::from(new_path), EntryKind::from(entry.mode()), entry_id)?;
                    changed = true;
                }
                return Ok(());
            }
            let mut new_blob_id = entry_id;
            if let Some(updated) = apply_patterns_with_log(&blob.data, &path_str, options) {
                if updated != blob.data {
                    new_blob_id = repo.write_blob(&updated)?.detach();
                    changed = true;
                }
            }

            if new_path != path_bytes || new_blob_id != entry_id {
                if new_path != path_bytes {
                    editor.remove(BString::from(path_bytes))?;
                }
                editor.upsert(BString::from(new_path), EntryKind::from(entry.mode()), new_blob_id)?;
                changed = true;
            }
        }
        Ok(())
    })?;

    if changed {
        let new_tree = editor.write()?.detach();
        normalize_tree(repo, new_tree)
    } else {
        Ok(tree_id)
    }
}

fn normalize_tree(repo: &gix::Repository, tree_id: ObjectId) -> Result<ObjectId> {
    let tree = repo.find_tree(tree_id)?;
    let mut entries = Vec::new();
    let mut changed = false;

    for entry in tree.iter() {
        let entry = entry?;
        let mut oid = entry.oid().to_owned();
        if entry.mode().is_tree() {
            let new_subtree = normalize_tree(repo, oid)?;
            if new_subtree != oid {
                changed = true;
                oid = new_subtree;
            }
        }

        let mode_value = entry.mode().value();
        let normalized_mode = gix_object::tree::EntryMode::try_from(mode_value as u32).unwrap_or(entry.mode());
        if normalized_mode != entry.mode() {
            changed = true;
        }

        entries.push(gix_object::tree::Entry {
            mode: normalized_mode,
            filename: entry.filename().to_owned(),
            oid,
        });
    }

    if !changed {
        return Ok(tree_id);
    }

    entries.sort();
    let tree_obj = TreeObject { entries };
    let new_id = repo
        .write_object(&tree_obj)
        .map_err(|err| anyhow!(err))?
        .detach();
    Ok(new_id)
}

fn walk_tree(
    repo: &gix::Repository,
    tree: &gix::Tree<'_>,
    path: &mut Vec<u8>,
    visitor: &mut impl FnMut(&[u8], &gix::object::tree::EntryRef<'_, '_>) -> Result<()>,
) -> Result<()> {
    for entry in tree.iter() {
        let entry = entry?;
        let name = entry.filename().as_ref();
        let original_len = path.len();
        if !path.is_empty() {
            path.push(b'/');
        }
        path.extend_from_slice(name);
        if entry.mode().is_tree() {
            let subtree = repo.find_tree(entry.oid())?;
            walk_tree(repo, &subtree, path, visitor)?;
        } else {
            visitor(path, &entry)?;
        }
        path.truncate(original_len);
    }
    Ok(())
}

fn apply_patterns(input: &[u8], options: &Options) -> Option<Vec<u8>> {
    if options.patterns.is_empty() {
        return None;
    }
    let mut data = input.to_vec();
    let mut changed = false;
    for pattern in &options.patterns {
        if !pattern.regex.is_match(&data) {
            continue;
        }
        let mut matched = false;
        let replaced = pattern.regex.replace_all(&data, |caps: &regex::bytes::Captures| {
            matched = true;
            if options.preserve_case {
                Cow::Owned(preserve_case(caps.get(0).unwrap().as_bytes(), &pattern.replacement))
            } else {
                Cow::Borrowed(pattern.replacement.as_slice())
            }
        });
        if matched {
            data = replaced.into_owned();
            changed = true;
        }
    }
    if changed { Some(data) } else { None }
}

fn apply_patterns_with_log(input: &[u8], path: &str, options: &Options) -> Option<Vec<u8>> {
    if options.patterns.is_empty() {
        return None;
    }
    let mut data = input.to_vec();
    let mut changed = false;

    for pattern in &options.patterns {
        let mut matched = false;
        let mut new_data = Vec::with_capacity(data.len());
        let mut last = 0;

        for mat in pattern.regex.find_iter(&data) {
            matched = true;
            let start = mat.start();
            let end = mat.end();
            let matched_bytes = &data[start..end];
            let repl = if options.preserve_case {
                Cow::Owned(preserve_case(matched_bytes, &pattern.replacement))
            } else {
                Cow::Borrowed(pattern.replacement.as_slice())
            };
            new_data.extend_from_slice(&data[last..start]);
            new_data.extend_from_slice(repl.as_ref());
            log_replacement(path, &data, start, end, repl.as_ref());
            last = end;
        }

        if matched {
            new_data.extend_from_slice(&data[last..]);
            data = new_data;
            changed = true;
        }
    }

    if changed { Some(data) } else { None }
}

fn preserve_case(matched: &[u8], replacement: &[u8]) -> Vec<u8> {
    if replacement.is_empty() {
        return Vec::new();
    }
    if is_upper(matched) {
        return replacement.iter().map(|b| b.to_ascii_uppercase()).collect();
    }
    if is_lower(matched) {
        return replacement.iter().map(|b| b.to_ascii_lowercase()).collect();
    }
    if is_title(matched) {
        let mut out = replacement.iter().map(|b| b.to_ascii_lowercase()).collect::<Vec<_>>();
        if let Some(first) = out.get_mut(0) {
            *first = first.to_ascii_uppercase();
        }
        return out;
    }
    let mut out = Vec::with_capacity(replacement.len());
    for (idx, byte) in replacement.iter().enumerate() {
        if let Some(src) = matched.get(idx) {
            if src.is_ascii_uppercase() {
                out.push(byte.to_ascii_uppercase());
            } else if src.is_ascii_lowercase() {
                out.push(byte.to_ascii_lowercase());
            } else {
                out.push(*byte);
            }
        } else {
            out.push(*byte);
        }
    }
    out
}

fn log_rename(old_path: &str, new_path: &str) {
    println!("{}{}{}", COLOR_PATH, old_path, COLOR_RESET);
    println!(
        "{}{}{} -> {}{}{}",
        COLOR_PATH, old_path, COLOR_RESET, COLOR_PATH, new_path, COLOR_RESET
    );
}

fn log_replacement(path: &str, snapshot: &[u8], start: usize, end: usize, repl: &[u8]) {
    let line_start = snapshot[..start]
        .iter()
        .rposition(|&b| b == b'\n')
        .map(|pos| pos + 1)
        .unwrap_or(0);
    let line_end = snapshot[end..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|pos| end + pos)
        .unwrap_or(snapshot.len());

    let prefix = String::from_utf8_lossy(&snapshot[line_start..start]);
    let suffix = String::from_utf8_lossy(&snapshot[end..line_end]);
    let match_text = String::from_utf8_lossy(&snapshot[start..end]);
    let repl_text = String::from_utf8_lossy(repl);

    let (left_line, right_line) = extract_context(&prefix, &suffix, &match_text, &repl_text);
    println!("{}{}{}", COLOR_PATH, path, COLOR_RESET);
    println!("{left_line} -> {right_line}");
}

fn extract_context(prefix: &str, suffix: &str, match_text: &str, repl_text: &str) -> (String, String) {
    let pre_words: Vec<&str> = prefix.split_whitespace().collect();
    let post_words: Vec<&str> = suffix.split_whitespace().collect();

    let left_words = if pre_words.len() > CTX_WORDS {
        &pre_words[pre_words.len() - CTX_WORDS..]
    } else {
        pre_words.as_slice()
    };
    let right_words = if post_words.len() > CTX_WORDS {
        &post_words[..CTX_WORDS]
    } else {
        post_words.as_slice()
    };

    let left = left_words.join(" ");
    let right = right_words.join(" ");
    let left = if left.is_empty() { String::new() } else { format!("{left} ") };
    let right = if right.is_empty() { String::new() } else { format!(" {right}") };

    let left_line = format!("{left}{COLOR_MATCH}{match_text}{COLOR_RESET}{right}");
    let right_line = format!("{left}{COLOR_REPL}{repl_text}{COLOR_RESET}{right}");

    (left_line.trim().to_string(), right_line.trim().to_string())
}

fn is_upper(bytes: &[u8]) -> bool {
    let mut has_alpha = false;
    for byte in bytes {
        if byte.is_ascii_lowercase() {
            return false;
        }
        if byte.is_ascii_uppercase() {
            has_alpha = true;
        }
    }
    has_alpha
}

fn is_lower(bytes: &[u8]) -> bool {
    let mut has_alpha = false;
    for byte in bytes {
        if byte.is_ascii_uppercase() {
            return false;
        }
        if byte.is_ascii_lowercase() {
            has_alpha = true;
        }
    }
    has_alpha
}

fn is_title(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    is_upper(&bytes[..1]) && is_lower(&bytes[1..])
}

fn is_binary(data: &[u8]) -> bool {
    data.contains(&0)
}

fn rewrite_references(
    repo: &gix::Repository,
    commit_map: &HashMap<ObjectId, ObjectId>,
    tag_map: &mut HashMap<ObjectId, ObjectId>,
) -> Result<()> {
    let platform = repo.references()?;
    for reference in platform.all()? {
        let mut reference = reference.map_err(|err| anyhow!(err.to_string()))?;
        let name = reference.name().as_bstr();
        if name.starts_with(b"refs/original/") || name.starts_with(b"refs/replace/") {
            continue;
        }
        let Some(target) = reference.target().try_id().map(|id| id.to_owned()) else {
            continue;
        };
        let object = repo.find_object(target)?;
        let new_target = match object.kind {
            ObjectKind::Commit => commit_map
                .get(&target)
                .copied()
                .ok_or_else(|| anyhow!("Missing rewritten commit for {target}"))?,
            ObjectKind::Tag => rewrite_tag(repo, target, commit_map, tag_map)?,
            _ => continue,
        };
        if new_target != target {
            update_reference(&mut reference, new_target)?;
        }
    }
    Ok(())
}

fn delete_remote_refs(repo: &gix::Repository) -> Result<()> {
    let platform = repo.references()?;
    for reference in platform.all()? {
        let reference = reference.map_err(|err| anyhow!(err.to_string()))?;
        let name = reference.name().as_bstr();
        if name.starts_with(b"refs/remotes/") {
            reference.delete()?;
        }
    }
    Ok(())
}

fn rewrite_tag(
    repo: &gix::Repository,
    tag_id: ObjectId,
    commit_map: &HashMap<ObjectId, ObjectId>,
    tag_map: &mut HashMap<ObjectId, ObjectId>,
) -> Result<ObjectId> {
    if let Some(existing) = tag_map.get(&tag_id) {
        return Ok(*existing);
    }
    let object = repo.find_object(tag_id)?;
    if object.kind != ObjectKind::Tag {
        return Err(anyhow!("Expected tag object, got {:?}", object.kind));
    }
    let raw_tag = parse_tag_object(&object.data)?;
    let mut target = raw_tag.target;
    let mut target_kind = raw_tag.target_kind;

    match raw_tag.target_kind {
        ObjectKind::Commit => {
            target = commit_map
                .get(&target)
                .copied()
                .ok_or_else(|| anyhow!("Missing rewritten commit for tag target"))?;
        }
        ObjectKind::Tag => {
            target = rewrite_tag(repo, target, commit_map, tag_map)?;
            target_kind = ObjectKind::Tag;
        }
        _ => {}
    }
    let tag_bytes = build_tag_bytes(target, target_kind, raw_tag.name, raw_tag.tagger, raw_tag.message);
    let new_id = repo
        .write_buf(ObjectKind::Tag, &tag_bytes)
        .map_err(|err| anyhow!(err))?;
    tag_map.insert(tag_id, new_id);
    Ok(new_id)
}

struct RawTag<'a> {
    target: ObjectId,
    target_kind: ObjectKind,
    name: &'a [u8],
    tagger: Option<&'a [u8]>,
    message: &'a [u8],
}

fn parse_tag_object(data: &[u8]) -> Result<RawTag<'_>> {
    let header_end = data
        .windows(2)
        .position(|win| win == b"\n\n")
        .ok_or_else(|| anyhow!("Invalid tag object: missing header terminator"))?;
    let header = &data[..header_end];
    let body = &data[header_end + 2..];

    let mut target: Option<ObjectId> = None;
    let mut target_kind: Option<ObjectKind> = None;
    let mut name: Option<&[u8]> = None;
    let mut tagger: Option<&[u8]> = None;

    for line in header.split(|b| *b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let mut iter = line.splitn(2, |b| *b == b' ');
        let key = iter.next().unwrap_or_default();
        let value = iter.next().unwrap_or_default();
        match key {
            b"object" => target = Some(ObjectId::from_hex(value)?),
            b"type" => {
                target_kind = Some(ObjectKind::from_bytes(value)?)
            }
            b"tag" => name = Some(value),
            b"tagger" => tagger = Some(value),
            _ => {}
        }
    }

    let marker = b"\n-----BEGIN PGP SIGNATURE-----";
    let message = if let Some(pos) = body.windows(marker.len()).position(|win| win == marker) {
        &body[..pos + 1]
    } else {
        body
    };

    Ok(RawTag {
        target: target.ok_or_else(|| anyhow!("Invalid tag object: missing object"))?,
        target_kind: target_kind.ok_or_else(|| anyhow!("Invalid tag object: missing type"))?,
        name: name.ok_or_else(|| anyhow!("Invalid tag object: missing tag name"))?,
        tagger,
        message,
    })
}

fn build_tag_bytes(
    target: ObjectId,
    target_kind: ObjectKind,
    name: &[u8],
    tagger: Option<&[u8]>,
    message: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"object ");
    out.extend_from_slice(target.to_string().as_bytes());
    out.push(b'\n');
    out.extend_from_slice(b"type ");
    out.extend_from_slice(target_kind.as_bytes());
    out.push(b'\n');
    out.extend_from_slice(b"tag ");
    out.extend_from_slice(name);
    out.push(b'\n');
    if let Some(tagger) = tagger {
        out.extend_from_slice(b"tagger ");
        out.extend_from_slice(tagger);
        out.push(b'\n');
    }
    out.push(b'\n');
    out.extend_from_slice(message);
    out
}

fn update_reference(reference: &mut Reference<'_>, new_target: ObjectId) -> Result<()> {
    reference
        .set_target_id(new_target, "gitkat rewrite")
        .with_context(|| format!("Update reference {}", reference.name().as_bstr()))?;
    Ok(())
}
