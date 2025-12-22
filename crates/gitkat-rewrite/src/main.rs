use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use gix::bstr::BString;
use gix::hash::ObjectId;
use gix::object::tree::EntryKind;
use gix::Reference;
use gix_object::{Commit as CommitObject, Kind as ObjectKind, Tag as TagObject};
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::bytes::{Regex, RegexBuilder};

fn commit_id_regex() -> &'static Regex {
    static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"[0-9a-f]{7,40}").expect("valid commit-id regex"))
}

#[derive(Parser, Debug)]
#[command(name = "gitkat-rewrite")]
struct Args {
    /// Path to the repository to rewrite.
    #[arg(long)]
    repo: PathBuf,
    /// New author/committer name.
    #[arg(long, default_value = "")]
    new_name: String,
    /// New author/committer email.
    #[arg(long, default_value = "")]
    new_email: String,
    /// Require this old author/committer name (case-insensitive).
    #[arg(long, default_value = "")]
    old_name: String,
    /// Old emails to match (repeatable).
    #[arg(long = "old-email")]
    old_emails: Vec<String>,
    /// Blob mapping old:new (repeatable).
    #[arg(long = "map")]
    blob_map: Vec<String>,
    /// Exclude file globs from blob rewrites (repeatable).
    #[arg(long = "exclude")]
    exclude_patterns: Vec<String>,
    /// Preserve casing in replacements.
    #[arg(long)]
    preserve_case: bool,
    /// Match replacements case-insensitively.
    #[arg(long)]
    ignore_case: bool,
    /// Apply mappings to file paths.
    #[arg(long)]
    rename_files: bool,
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
    preserve_case: bool,
    rename_files: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let options = Options::from_args(&args)?;
    rewrite_repo(&args.repo, &options)
}

impl Options {
    fn from_args(args: &Args) -> Result<Self> {
        let patterns = parse_patterns(&args.blob_map, args.ignore_case)?;
        let exclude = build_exclude_set(&args.exclude_patterns)?;
        let old_emails = args
            .old_emails
            .iter()
            .map(|email| email.trim().to_lowercase())
            .filter(|email| !email.is_empty())
            .collect::<HashSet<_>>();
        let old_name = if args.old_name.trim().is_empty() {
            None
        } else {
            Some(args.old_name.trim().to_lowercase())
        };
        let new_name = if args.new_name.trim().is_empty() {
            None
        } else {
            Some(BString::from(args.new_name.as_bytes()))
        };
        let new_email = if args.new_email.trim().is_empty() {
            None
        } else {
            Some(BString::from(args.new_email.as_bytes()))
        };
        Ok(Self {
            new_name,
            new_email,
            old_name,
            old_emails,
            patterns,
            exclude,
            preserve_case: args.preserve_case,
            rename_files: args.rename_files,
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

fn rewrite_repo(repo_path: &PathBuf, options: &Options) -> Result<()> {
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
        let new_id = rewrite_commit(&repo, commit_id, options, &commit_map, &commit_hex)?;
        commit_map.insert(commit_id, new_id);
        commit_hex.push((commit_id.to_string(), new_id.to_string()));
    }

    rewrite_references(&repo, &commit_map, &mut tag_map)?;
    delete_remote_refs(&repo)?;
    Ok(())
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
    let commit = repo.find_commit(commit_id)?;
    let decoded = commit.decode()?;
    let mut owned: CommitObject = decoded.to_owned()?;
    // Match git fast-export/import behavior by dropping extra headers like gpgsig.
    owned.extra_headers.clear();

    let (author, author_changed) = rewrite_signature(owned.author, options);
    let (committer, committer_changed) = rewrite_signature(owned.committer, options);
    let identity_changed = author_changed || committer_changed;
    owned.author = author;
    owned.committer = committer;

    if identity_changed {
        owned.message = cleanup_dco(owned.message);
    }

    owned.message = rewrite_commit_ids(owned.message, commit_map, commit_hex);

    owned.tree = rewrite_tree(repo, owned.tree, options)?;
    let old_parents = owned.parents.clone();
    owned.parents = old_parents
        .iter()
        .map(|parent| {
            commit_map
                .get(parent)
                .copied()
                .ok_or_else(|| anyhow!("Missing rewritten parent {parent}"))
        })
        .collect::<Result<_>>()?;

    let new_id = repo.write_object(&owned)?;
    Ok(new_id.detach())
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

fn rewrite_signature(mut signature: gix::actor::Signature, options: &Options) -> (gix::actor::Signature, bool) {
    if options.old_emails.is_empty() || options.new_email.is_none() {
        return (signature, false);
    }
    let email_lower = lower_bytes(signature.email.as_ref());
    let name_lower = lower_bytes(signature.name.as_ref());
    let matches_email = options.old_emails.contains(&email_lower);
    let matches_name = options
        .old_name
        .as_ref()
        .map(|name| name == &name_lower)
        .unwrap_or(true);
    if matches_email && matches_name {
        if let Some(name) = &options.new_name {
            signature.name = name.clone();
        }
        signature.email = options.new_email.clone().expect("new email");
        (signature, true)
    } else {
        (signature, false)
    }
}

fn lower_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_lowercase()
}

fn cleanup_dco(message: BString) -> BString {
    let regex = Regex::new(r"(?im)^\s*(signed-off-by|co-authored-by|reviewed-by|acked-by|tested-by|reported-by|suggested-by):.*\n?")
        .expect("valid DCO regex");
    let cleaned = regex.replace_all(message.as_ref(), b"".as_slice());
    BString::from(cleaned.into_owned())
}

fn rewrite_tree(repo: &gix::Repository, tree_id: ObjectId, options: &Options) -> Result<ObjectId> {
    if options.patterns.is_empty() && !options.rename_files {
        return Ok(tree_id);
    }

    let tree = repo.find_tree(tree_id)?;
    let mut editor = tree.edit()?;
    let mut path = Vec::new();
    let mut changed = false;

    walk_tree(repo, &tree, &mut path, &mut |path_bytes, entry| {
        let path_str = String::from_utf8_lossy(path_bytes);
        if let Some(exclude) = &options.exclude {
            if exclude.is_match(path_str.as_ref()) {
                return Ok(());
            }
        }

        let mut new_path = path_bytes.to_vec();
        if options.rename_files {
            if let Some(updated) = apply_patterns(&new_path, options) {
                if updated != new_path {
                    new_path = updated;
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
            if let Some(updated) = apply_patterns(&blob.data, options) {
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
        Ok(editor.write()?.detach())
    } else {
        Ok(tree_id)
    }
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
    let tag = repo.find_tag(tag_id)?;
    let decoded = tag.decode()?;
    let mut target = decoded.target();
    let mut target_kind = decoded.target_kind;

    match decoded.target_kind {
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

    let tagger = decoded.tagger()?.map(|sig| sig.to_owned()).transpose()?;
    let new_tag = TagObject {
        target,
        target_kind,
        name: decoded.name.to_owned(),
        tagger,
        message: decoded.message.to_owned(),
        pgp_signature: None,
    };
    let new_id = repo.write_object(&new_tag)?.detach();
    tag_map.insert(tag_id, new_id);
    Ok(new_id)
}

fn update_reference(reference: &mut Reference<'_>, new_target: ObjectId) -> Result<()> {
    reference
        .set_target_id(new_target, "gitkat rewrite")
        .with_context(|| format!("Update reference {}", reference.name().as_bstr()))?;
    Ok(())
}
