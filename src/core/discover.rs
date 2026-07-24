use anyhow::Result;
use std::path::Path;

/// Find all regular files in `dir` whose extension matches one of `extensions`
/// (case-insensitive). Hidden files (dotfiles) are skipped. Symlinks that
/// resolve to a regular file are included (renaming just renames the link
/// itself, which is the expected behavior).
pub fn discover_files(dir: &Path, extensions: &[String]) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();

    for entry in dir.read_dir()? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();

        // Skip non-files and hidden files.
        if !path.is_file()
            || path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.'))
        {
            continue;
        }

        // Empty extension list = include all files
        if extensions.is_empty() {
            files.push(path.clone());
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                files.push(path.clone());
            }
        }
    }

    files.sort();
    Ok(files)
}
