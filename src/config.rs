use std::path::PathBuf;

/// Parse pipe-separated extension string into lowercase, trimmed Vec<String>.
pub fn parse_extensions(s: &str) -> Vec<String> {
    s.split('|')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Expand '~' to home dir, otherwise return as-is.
pub fn expand_tilde(p: &std::path::Path) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix('~') {
        if let Some(home) = std::env::var_os("HOME") {
            let rest = rest.strip_prefix('/').unwrap_or(rest);
            return PathBuf::from(home).join(rest);
        }
    }
    p.to_path_buf()
}

pub struct RenameConfig {
    pub dir: PathBuf,
    pub title: String,
    pub prefix: String,
    pub suffix: String,
    pub extensions: Vec<String>,
    pub dry_run: bool,
    /// When a target name collides (already exists on disk, or with another
    /// file in this same batch), append " (2)", " (3)", … until a free name
    /// is found, instead of skipping the file.
    pub collision_auto_num: bool,
}

impl Default for RenameConfig {
    fn default() -> Self {
        Self {
            dir: PathBuf::from("."),
            title: String::new(),
            prefix: String::new(),
            suffix: String::new(),
            extensions: vec!["mp4".into(), "mkv".into(), "webm".into(), "avi".into()],
            dry_run: true,
            collision_auto_num: false,
        }
    }
}
