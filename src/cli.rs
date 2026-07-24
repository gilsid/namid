use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::config::RenameConfig;
use crate::core::{discover, rename};

/// One‑shot CLI arguments — also used to pre‑fill the TUI form.
#[derive(Parser, Debug)]
#[command(
    name = "namid",
    version,
    about = "Batch rename files with a clean format"
)]
pub struct Cli {
    /// Target directory with files to rename
    #[arg(short, long)]
    pub dir: Option<PathBuf>,

    /// Clean series title (e.g. "My Series")
    #[arg(short, long)]
    pub title: Option<String>,

    /// Literal prefix to strip from each filename start
    #[arg(short, long)]
    pub prefix: Option<String>,

    /// Literal suffix to strip before the extension
    #[arg(short, long)]
    pub suffix: Option<String>,

    /// Pipe‑separated file extensions to match (pass "" explicitly to match every file)
    #[arg(short, long, default_value = "mp4|mkv|webm|avi")]
    pub extensions: String,

    /// Actually rename files (default: dry‑run / simulate)
    #[arg(short = 'x', long)]
    pub exec: bool,

    /// Force TUI mode (even when CLI flags are present)
    #[arg(long)]
    pub tui: bool,

    /// Skip the confirmation prompt before executing (for scripting)
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// On a name collision, append " (2)", " (3)", … instead of skipping
    #[arg(short = 'a', long)]
    pub auto_number: bool,
}

/// Build a RenameConfig from CLI arguments, applying defaults where omitted.
impl From<&Cli> for RenameConfig {
    fn from(args: &Cli) -> Self {
        let mut config = RenameConfig::default();
        if let Some(dir) = &args.dir {
            config.dir = crate::config::expand_tilde(dir);
        }
        if let Some(title) = &args.title {
            config.title = title.clone();
        }
        if let Some(prefix) = &args.prefix {
            config.prefix = prefix.clone();
        }
        if let Some(suffix) = &args.suffix {
            config.suffix = suffix.clone();
        }
        config.extensions = crate::config::parse_extensions(&args.extensions);
        config.dry_run = !args.exec;
        config.collision_auto_num = args.auto_number;
        config
    }
}

pub fn run_cli(args: &Cli) -> Result<()> {
    let config: RenameConfig = args.into();

    if config.title.is_empty() {
        anyhow::bail!("--title is required in CLI mode");
    }

    let dir = config.dir.clone();
    if !dir.exists() {
        anyhow::bail!("directory not found: {}", dir.display());
    }

    let files = discover::discover_files(&dir, &config.extensions)?;
    if files.is_empty() {
        let hint = if config.extensions.is_empty() {
            "No files found in directory".to_string()
        } else {
            format!(
                "No files found matching *.{ext}",
                ext = config.extensions.join("|")
            )
        };
        eprintln!("{hint}");
        return Ok(());
    }

    let mut ops = rename::generate_plan(&files, &config)?;

    let mode_label = if config.dry_run {
        "SIMULATION"
    } else {
        "EXECUTION"
    };

    println!("=== namid: Batch Rename ===");
    println!("Dir:     {}", dir.display());
    println!("Title:   {}", config.title);
    if !config.prefix.is_empty() {
        println!("Prefix:  {}", config.prefix);
    }
    if !config.suffix.is_empty() {
        println!("Suffix:  {}", config.suffix);
    }
    println!("Mode:    {mode_label}");
    if config.collision_auto_num {
        println!("Collide: auto-number duplicates instead of skipping");
    }
    if config.dry_run {
        println!("         (add --exec to execute)");
    }
    println!();

    for (i, op) in ops.iter().enumerate() {
        match &op.status {
            rename::RenameStatus::Pending | rename::RenameStatus::Success => {
                println!(
                    "  {i:>3}/{len}  {}  →  {}",
                    op.from.file_name().unwrap_or_default().to_string_lossy(),
                    op.to.file_name().unwrap_or_default().to_string_lossy(),
                    len = ops.len()
                );
            }
            rename::RenameStatus::SkipNoChange => {}
            rename::RenameStatus::SkipEmptyName => {
                println!(
                    "  {i:>3}/{len}  {}  →  [SKIP] name became empty after strip rules",
                    op.from.file_name().unwrap_or_default().to_string_lossy(),
                    len = ops.len()
                );
            }
            rename::RenameStatus::SkipExists => {
                println!(
                    "  {i:>3}/{len}  {}  →  [SKIP] target exists",
                    op.from.file_name().unwrap_or_default().to_string_lossy(),
                    len = ops.len()
                );
            }
            rename::RenameStatus::SkipCollision => {
                println!(
                    "  {i:>3}/{len}  {}  →  [SKIP] collision in batch",
                    op.from.file_name().unwrap_or_default().to_string_lossy(),
                    len = ops.len()
                );
            }
            rename::RenameStatus::Error(msg) => {
                println!(
                    "  {i:>3}/{len}  {}  →  [ERR] {msg}",
                    op.from.file_name().unwrap_or_default().to_string_lossy(),
                    len = ops.len()
                );
            }
        }
    }

    if !config.dry_run {
        if !args.yes {
            use std::io::Write;
            print!(
                "\nProceed with execution ({} files, PERMANENT, cannot be undone)? [y/N] ",
                ops.iter()
                    .filter(|o| matches!(o.status, rename::RenameStatus::Pending))
                    .count()
            );
            std::io::stdout().flush().ok();
            let mut answer = String::new();
            std::io::stdin().read_line(&mut answer)?;
            let answer = answer.trim().to_lowercase();
            if answer != "y" && answer != "yes" {
                println!("Cancelled.");
                return Ok(());
            }
        }
        let stats = rename::execute_plan(&mut ops, false)?;
        println!(
            "\nResult: {} renamed · {} skipped · {} errors  ({:.2?})",
            stats.renamed, stats.skipped, stats.errors, stats.duration
        );
    } else {
        let pending = ops
            .iter()
            .filter(|op| matches!(op.status, rename::RenameStatus::Pending))
            .count();
        let skipped = ops.len() - pending;
        println!("\nPlan: {pending} to rename · {skipped} skip");
    }

    Ok(())
}
