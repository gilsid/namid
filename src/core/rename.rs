use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;

use crate::config::RenameConfig;

/// One file operation in a rename plan.
#[derive(Debug, Clone)]
pub struct RenameOp {
    pub from: PathBuf,
    pub to: PathBuf,
    pub status: RenameStatus,
}

#[derive(Debug, Clone)]
pub enum RenameStatus {
    Pending,
    SkipExists,
    SkipCollision,
    SkipNoChange,
    SkipEmptyName,
    Success,
    Error(String),
}

/// Aggregate result after executing a plan.
#[derive(Debug, Default, Clone)]
pub struct FinalStats {
    pub renamed: usize,
    pub skipped: usize,
    pub errors: usize,
    pub duration: std::time::Duration,
}

/// Quality/language tags stripped from the *end* of filenames.
/// Matches tags preceded by `_`, `.`, or `-` as separator.
const STRIP_TAGS: &[&str] = &[
    "720p", "1080p", "2160p", "4K", "HD", "FHD", "UHD", "2K", "BD", "BluRay", "WEBRip", "WEB-DL",
    "BRRip", "HDR", "x264", "x265", "HEVC", "AVC", "Complete", "Batch", "eng", "indo", "jpn", "en",
    "id", "jp", "sub", "dub", "multi",
];

fn strip_tags(s: &str) -> String {
    let mut s = s.to_string();
    loop {
        let before = s.len();
        for tag in STRIP_TAGS {
            for sep in ['.', '_', '-'] {
                let pattern = format!("{sep}{tag}");
                while s.ends_with(&pattern) {
                    s = s[..s.len() - pattern.len()].to_string();
                }
            }
        }
        if s.len() == before {
            break;
        }
    }
    s
}

/// Generate a rename plan: for each file, determine the new name and check
/// for collisions. File "to" paths are set for every op, but no disk writes
/// happen here.
pub fn generate_plan(files: &[PathBuf], config: &RenameConfig) -> Result<Vec<RenameOp>> {
    let mut used_names: HashSet<String> = HashSet::new();
    // Per-base_name counter so auto-number resumes where the last colliding
    // file left off, avoiding O(n²) scan through already-taken numbers.
    let mut next_num: HashMap<String, u32> = HashMap::new();
    let mut ops = Vec::with_capacity(files.len());

    for file in files {
        let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");

        let mut base = stem.to_string();

        // 1. Strip literal prefix from start
        if !config.prefix.is_empty() {
            if let Some(rest) = base.strip_prefix(&config.prefix).map(String::from) {
                base = rest;
            }
        }

        // 2. Strip literal suffix from end
        if !config.suffix.is_empty() {
            if let Some(rest) = base.strip_suffix(&config.suffix).map(String::from) {
                base = rest;
            }
        }

        // 3. Strip quality/lang tags from end repeatedly (now handles . - _ separators)
        base = strip_tags(&base);

        // 4. Normalize separators: underscore → space, collapse whitespace, trim
        base = base.replace('_', " ");
        base = base.split_whitespace().collect::<Vec<_>>().join(" ");
        base = base
            .trim_matches(|c: char| c == '-' || c == ' ')
            .to_string();

        if base.is_empty() {
            ops.push(RenameOp {
                from: file.clone(),
                to: file.clone(),
                status: RenameStatus::SkipEmptyName,
            });
            continue;
        }

        let base_name = format!("{} - {}.{}", config.title, base, ext);
        let original_name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if base_name == original_name {
            ops.push(RenameOp {
                from: file.clone(),
                to: file.clone(),
                status: RenameStatus::SkipNoChange,
            });
            continue;
        }

        let mut new_name = base_name.clone();
        let mut to = file.with_file_name(&new_name);
        let mut status = RenameStatus::Pending;

        if to.exists() || used_names.contains(&new_name.to_lowercase()) {
            if config.collision_auto_num {
                let start = next_num.get(&base_name).copied().unwrap_or(2);
                let mut found = false;
                for n in start..=9999u32 {
                    let candidate = format!("{} - {} ({}).{}", config.title, base, n, ext);
                    let candidate_to = file.with_file_name(&candidate);
                    if !candidate_to.exists() && !used_names.contains(&candidate.to_lowercase()) {
                        new_name = candidate;
                        to = candidate_to;
                        next_num.insert(base_name.clone(), n + 1);
                        found = true;
                        break;
                    }
                }
                if !found {
                    status = RenameStatus::SkipCollision;
                }
            } else {
                status = if to.exists() {
                    RenameStatus::SkipExists
                } else {
                    RenameStatus::SkipCollision
                };
            }
        }

        if matches!(status, RenameStatus::Pending) {
            used_names.insert(new_name.to_lowercase());
        }

        ops.push(RenameOp {
            from: file.clone(),
            to,
            status,
        });
    }

    Ok(ops)
}

/// Execute the rename plan: rename files on disk (unless dry_run).
/// Idempotent: only acts on ops with `Pending` status.
/// Returns aggregate stats.
pub fn execute_plan(ops: &mut [RenameOp], dry_run: bool) -> Result<FinalStats> {
    let start = Instant::now();
    let mut stats = FinalStats::default();

    for op in ops.iter_mut() {
        if !matches!(op.status, RenameStatus::Pending) {
            stats.skipped += 1;
            continue;
        }

        if dry_run {
            op.status = RenameStatus::Success;
            stats.renamed += 1;
            continue;
        }

        match std::fs::rename(&op.from, &op.to) {
            Ok(()) => {
                op.status = RenameStatus::Success;
                stats.renamed += 1;
            }
            Err(e) => {
                op.status = RenameStatus::Error(e.to_string());
                stats.errors += 1;
            }
        }
    }

    stats.duration = start.elapsed();
    Ok(stats)
}
