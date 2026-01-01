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
    /// Delete files matching these globs (repeatable).
    #[arg(long = "delete-path")]
    delete_paths: Vec<String>,
    /// Treat blob mapping patterns as regex.
    #[arg(long = "regex")]
    regex_map: bool,
    /// Preserve casing in replacements.
    #[arg(long)]
    preserve_case: bool,
    /// Match replacements case-insensitively.
    #[arg(long)]
    ignore_case: bool,
    /// Apply mappings to file paths.
    #[arg(long)]
    rename_files: bool,
    /// Suppress per-match logging.
    #[arg(long, short = 'q')]
    quiet: bool,
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
        delete_paths: args.delete_paths,
        regex_map: args.regex_map,
        preserve_case: args.preserve_case,
        ignore_case: args.ignore_case,
        rename_files: args.rename_files,
        quiet: args.quiet,
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
