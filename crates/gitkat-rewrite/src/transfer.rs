use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context, Result};
use gix::bstr::BString;
use gix::hash::ObjectId;
use gix::refs::transaction::PreviousValue;
use gix_object::{
    tree::EntryKind, CommitRef, Kind as ObjectKind, TagRef, TreeRefIter, Write as ObjectWrite,
};

const EXPORT_HEADER: &str = "gix-fast-export-1";
const END_REFS: &str = "end-refs";
const END_OBJECTS: &str = "end-objects";
const LOG_MESSAGE: &str = "gix-import";

#[derive(Clone, Debug)]
struct ExportRef {
    name: String,
    target: ObjectId,
}

pub fn gix_export(repo_path: &std::path::Path, mut out: impl Write) -> Result<()> {
    let repo =
        gix::open(repo_path).with_context(|| format!("Open repo at {}", repo_path.display()))?;
    let workdir = repo_workdir(&repo);
    let mut refs = collect_refs(&repo)?;
    refs.sort_by(|a, b| a.name.cmp(&b.name));

    let objects = collect_objects(&repo, &workdir, &refs)?;

    writeln!(out, "{EXPORT_HEADER}")?;
    for export_ref in &refs {
        writeln!(out, "ref {} {}", export_ref.name, export_ref.target)?;
    }
    writeln!(out, "{END_REFS}")?;

    for oid in objects {
        let (kind, data) = load_object(&repo, &workdir, oid)?;
        let kind = object_kind_label(kind)?;
        writeln!(out, "object {} {} {}", oid, kind, data.len())?;
        out.write_all(&data)?;
        out.write_all(b"\n")?;
    }
    writeln!(out, "{END_OBJECTS}")?;
    Ok(())
}

pub fn gix_import(repo_path: &std::path::Path, input: impl Read) -> Result<()> {
    let mut repo = match gix::open(repo_path) {
        Ok(repo) => repo,
        Err(_) => gix::init(repo_path)
            .with_context(|| format!("Initialize repo at {}", repo_path.display()))?,
    };

    let mut reader = BufReader::new(input);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if line.trim_end() != EXPORT_HEADER {
        return Err(anyhow!("Invalid export header: expected '{EXPORT_HEADER}'"));
    }

    let mut refs = Vec::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Err(anyhow!("Unexpected end of stream while reading refs"));
        }
        let trimmed = line.trim_end();
        if trimmed == END_REFS {
            break;
        }
        let export_ref = parse_ref_line(trimmed)?;
        refs.push(export_ref);
    }

    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Err(anyhow!("Unexpected end of stream while reading objects"));
        }
        let trimmed = line.trim_end();
        if trimmed == END_OBJECTS {
            break;
        }
        let (expected_id, kind, size) = parse_object_header(trimmed)?;
        let mut data = vec![0u8; size];
        reader.read_exact(&mut data)?;
        let mut newline = [0u8; 1];
        reader.read_exact(&mut newline)?;
        if newline[0] != b'\n' {
            return Err(anyhow!("Expected newline after object data"));
        }
        let actual_id = repo.write_buf(kind, &data).map_err(|err| anyhow!(err))?;
        if actual_id != expected_id {
            return Err(anyhow!(
                "Object id mismatch for {expected_id}: wrote {actual_id}"
            ));
        }
    }

    repo.committer_or_set_generic_fallback()?;
    for export_ref in refs {
        repo.reference(
            export_ref.name.as_str(),
            export_ref.target,
            PreviousValue::Any,
            BString::from(LOG_MESSAGE),
        )?;
    }

    Ok(())
}

fn collect_refs(repo: &gix::Repository) -> Result<Vec<ExportRef>> {
    let platform = repo.references()?;
    let mut refs = Vec::new();
    for reference in platform.all()? {
        let reference = reference.map_err(|err| anyhow!(err.to_string()))?;
        let name = reference.name().as_bstr();
        if name.starts_with(b"refs/original/") || name.starts_with(b"refs/replace/") {
            continue;
        }
        let Some(target) = reference.target().try_id().map(|id| id.to_owned()) else {
            continue;
        };
        let name = String::from_utf8_lossy(name).to_string();
        refs.push(ExportRef { name, target });
    }
    Ok(refs)
}

fn collect_objects(
    repo: &gix::Repository,
    workdir: &Path,
    refs: &[ExportRef],
) -> Result<Vec<ObjectId>> {
    let mut seen = HashSet::new();
    let mut stack = Vec::new();
    for export_ref in refs {
        stack.push(export_ref.target);
    }

    while let Some(oid) = stack.pop() {
        if !seen.insert(oid) {
            continue;
        }
        let (kind, data) = load_object(repo, workdir, oid)?;
        match kind {
            ObjectKind::Commit => {
                let commit = CommitRef::from_bytes(&data)?;
                let tree_id = ObjectId::from_hex(commit.tree.as_ref())?;
                stack.push(tree_id);
                for parent_hex in commit.parents {
                    let parent_id = ObjectId::from_hex(parent_hex.as_ref())?;
                    stack.push(parent_id);
                }
            }
            ObjectKind::Tree => {
                for entry in TreeRefIter::from_bytes(&data) {
                    let entry = entry?;
                    if entry.mode.kind() == EntryKind::Commit {
                        // Submodules point at commits not stored in this repo; fast-export skips them.
                        continue;
                    }
                    stack.push(entry.oid.to_owned());
                }
            }
            ObjectKind::Tag => {
                let tag = TagRef::from_bytes(&data)?;
                let target_id = ObjectId::from_hex(tag.target.as_ref())?;
                stack.push(target_id);
            }
            ObjectKind::Blob => {}
        }
    }

    let mut objects: Vec<ObjectId> = seen.into_iter().collect();
    objects.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    Ok(objects)
}

fn repo_workdir(repo: &gix::Repository) -> PathBuf {
    repo.workdir()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| repo.path().to_path_buf())
}

fn load_object(
    repo: &gix::Repository,
    workdir: &Path,
    oid: ObjectId,
) -> Result<(ObjectKind, Vec<u8>)> {
    match repo.find_object(oid) {
        Ok(object) => {
            let detached = object.detach();
            Ok((detached.kind, detached.data))
        }
        Err(err) => git_cat_file_object(workdir, oid).with_context(|| {
            format!("git cat-file fallback failed for {oid} after gix error: {err}")
        }),
    }
}

fn git_cat_file_object(workdir: &Path, oid: ObjectId) -> Result<(ObjectKind, Vec<u8>)> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .arg("cat-file")
        .arg("--batch")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Spawn git cat-file in {}", workdir.display()))?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Missing git cat-file stdin"))?;
        writeln!(stdin, "{oid}")?;
    }

    let output = child
        .wait_with_output()
        .context("Read git cat-file output")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git cat-file failed in {}: {}",
            workdir.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let newline = output
        .stdout
        .iter()
        .position(|&byte| byte == b'\n')
        .ok_or_else(|| anyhow!("git cat-file output missing header newline"))?;
    let header = String::from_utf8_lossy(&output.stdout[..newline]);
    let header = header.trim_end();
    if header.ends_with(" missing") {
        return Err(anyhow!("git cat-file reports {header}"));
    }

    let mut parts = header.splitn(3, ' ');
    let header_oid = parts.next().unwrap_or_default();
    let kind = parts.next().ok_or_else(|| anyhow!("Missing object kind"))?;
    let size = parts.next().ok_or_else(|| anyhow!("Missing object size"))?;
    if header_oid != oid.to_string() {
        return Err(anyhow!(
            "git cat-file returned {header_oid} while requesting {oid}"
        ));
    }
    let size = size
        .parse::<usize>()
        .with_context(|| format!("Bad object size: {size}"))?;
    let data_start = newline + 1;
    let data_end = data_start + size;
    if output.stdout.len() < data_end + 1 {
        return Err(anyhow!("git cat-file output truncated for {oid}"));
    }
    let data = output.stdout[data_start..data_end].to_vec();
    if output.stdout[data_end] != b'\n' {
        return Err(anyhow!("Missing newline after git cat-file data"));
    }
    let kind = ObjectKind::from_bytes(kind.as_bytes())
        .with_context(|| format!("Bad object kind: {kind}"))?;
    Ok((kind, data))
}

fn object_kind_label(kind: ObjectKind) -> Result<&'static str> {
    match kind {
        ObjectKind::Commit => Ok("commit"),
        ObjectKind::Tree => Ok("tree"),
        ObjectKind::Blob => Ok("blob"),
        ObjectKind::Tag => Ok("tag"),
    }
}

fn parse_ref_line(line: &str) -> Result<ExportRef> {
    let mut parts = line.splitn(3, ' ');
    let token = parts.next().unwrap_or_default();
    if token != "ref" {
        return Err(anyhow!("Invalid ref line: {line}"));
    }
    let name = parts.next().ok_or_else(|| anyhow!("Missing ref name"))?;
    let oid = parts.next().ok_or_else(|| anyhow!("Missing ref target"))?;
    let target =
        ObjectId::from_hex(oid.as_bytes()).with_context(|| format!("Bad ref oid: {oid}"))?;
    Ok(ExportRef {
        name: name.to_string(),
        target,
    })
}

fn parse_object_header(line: &str) -> Result<(ObjectId, ObjectKind, usize)> {
    let mut parts = line.splitn(4, ' ');
    let token = parts.next().unwrap_or_default();
    if token != "object" {
        return Err(anyhow!("Invalid object line: {line}"));
    }
    let oid = parts.next().ok_or_else(|| anyhow!("Missing object id"))?;
    let kind = parts.next().ok_or_else(|| anyhow!("Missing object kind"))?;
    let size = parts.next().ok_or_else(|| anyhow!("Missing object size"))?;

    let object_id =
        ObjectId::from_hex(oid.as_bytes()).with_context(|| format!("Bad object id: {oid}"))?;
    let kind = ObjectKind::from_bytes(kind.as_bytes())
        .with_context(|| format!("Bad object kind: {kind}"))?;
    let size = size
        .parse::<usize>()
        .with_context(|| format!("Bad object size: {size}"))?;
    Ok((object_id, kind, size))
}
