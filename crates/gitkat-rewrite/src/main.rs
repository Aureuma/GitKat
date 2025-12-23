use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use gitkat_rewrite::{rewrite_repo, RewriteConfig};

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

fn main() -> Result<()> {
    let args = Args::parse();
    let config = RewriteConfig {
        new_name: to_opt(args.new_name),
        new_email: to_opt(args.new_email),
        old_name: to_opt(args.old_name),
        old_emails: args.old_emails,
        blob_map: args.blob_map,
        exclude_patterns: args.exclude_patterns,
        preserve_case: args.preserve_case,
        ignore_case: args.ignore_case,
        rename_files: args.rename_files,
    };
    rewrite_repo(&args.repo, &config).map(|_| ())
}

fn to_opt(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
