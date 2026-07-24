use std::fs;
use std::path::Path;

use namid::config::RenameConfig;
use namid::core::{discover, rename};

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("create temp dir")
}

fn touch(dir: &Path, name: &str) {
    fs::write(dir.join(name), "").expect("write test file");
}

fn default_config() -> RenameConfig {
    RenameConfig {
        dir: Path::new(".").to_path_buf(),
        title: "Title".into(),
        prefix: String::new(),
        suffix: String::new(),
        extensions: vec!["mp4".into()],
        dry_run: true,
        collision_auto_num: false,
    }
}

fn run_test<F>(
    title: &str,
    files: &[&str],
    customize: F,
) -> (tempfile::TempDir, Vec<rename::RenameOp>)
where
    F: FnOnce(&mut RenameConfig),
{
    let d = temp_dir();
    for f in files {
        touch(d.path(), f);
    }
    let mut config = RenameConfig {
        dir: d.path().to_path_buf(),
        title: title.into(),
        ..default_config()
    };
    customize(&mut config);
    let found = discover::discover_files(d.path(), &config.extensions).unwrap();
    let ops = rename::generate_plan(&found, &config).unwrap();
    (d, ops)
}

// discover::discover_files

#[test]
fn discover_finds_matching_extensions() {
    let d = temp_dir();
    touch(d.path(), "vid_01.mp4");
    touch(d.path(), "vid_02.mkv");
    touch(d.path(), "ignored.txt");
    touch(d.path(), ".hidden.mp4");
    let exts = &["mp4".to_string(), "mkv".to_string()];
    let files = discover::discover_files(d.path(), exts).unwrap();
    assert_eq!(files.len(), 2);
    assert!(
        files
            .iter()
            .all(|p| p.extension().unwrap() == "mp4" || p.extension().unwrap() == "mkv")
    );
}

#[test]
fn discover_case_insensitive() {
    let d = temp_dir();
    touch(d.path(), "vid_01.MP4");
    touch(d.path(), "vid_02.Mkv");
    let exts = &["mp4".to_string(), "mkv".to_string()];
    let files = discover::discover_files(d.path(), exts).unwrap();
    assert_eq!(files.len(), 2);
}

#[test]
fn discover_empty_dir() {
    let d = temp_dir();
    let exts = &["mp4".to_string()];
    let files = discover::discover_files(d.path(), exts).unwrap();
    assert!(files.is_empty());
}

#[test]
fn discover_skips_hidden_files() {
    let d = temp_dir();
    touch(d.path(), ".hidden.mp4");
    touch(d.path(), "visible.mp4");
    let exts = &["mp4".to_string()];
    let files = discover::discover_files(d.path(), exts).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].file_name().unwrap() != ".hidden.mp4");
}

#[test]
fn discover_empty_extensions_matches_every_file() {
    let d = temp_dir();
    touch(d.path(), "video.mp4");
    touch(d.path(), "notes.txt");
    touch(d.path(), "cover.jpg");
    let files = discover::discover_files(d.path(), &[]).unwrap();
    assert_eq!(files.len(), 3);
}

// rename::generate_plan

#[test]
fn plan_strips_prefix() {
    let (_d, mut ops) = run_test("Title", &["PREFIX_SERIES_E01_720p.mp4"], |c| {
        c.prefix = "PREFIX_SERIES_".into()
    });
    rename::execute_plan(&mut ops, true).unwrap();
    assert_eq!(
        ops[0].to.file_name().unwrap().to_string_lossy(),
        "Title - E01.mp4"
    );
}

#[test]
fn plan_strips_suffix() {
    let (_d, ops) = run_test("Series", &["Vid_01_eng.mp4"], |c| c.suffix = "_eng".into());
    assert_eq!(
        ops[0].to.file_name().unwrap().to_string_lossy(),
        "Series - Vid 01.mp4"
    );
}

#[test]
fn plan_strips_quality_tags() {
    let (_d, ops) = run_test("Series", &["Ep01_1080p_eng.mp4"], |_| {});
    assert_eq!(
        ops[0].to.file_name().unwrap().to_string_lossy(),
        "Series - Ep01.mp4"
    );
}

#[test]
fn plan_preserves_underscore_separated_words() {
    let (_d, ops) = run_test("Series", &["sp_ep_2.mp4"], |_| {});
    assert_eq!(
        ops[0].to.file_name().unwrap().to_string_lossy(),
        "Series - sp ep 2.mp4"
    );
}

#[test]
fn plan_skips_unchanged() {
    let (_d, ops) = run_test("Title", &["Title - 01.mp4"], |c| {
        c.prefix = "Title - ".into()
    });
    assert!(matches!(ops[0].status, rename::RenameStatus::SkipNoChange));
}

#[test]
fn plan_detects_collision() {
    let (_d, ops) = run_test("Series", &["Ep01_x264.mp4", "Ep01_720p.mp4"], |_| {});
    assert_eq!(
        ops.iter()
            .filter(|o| matches!(o.status, rename::RenameStatus::Pending))
            .count(),
        1
    );
    assert_eq!(
        ops.iter()
            .filter(|o| matches!(o.status, rename::RenameStatus::SkipCollision))
            .count(),
        1
    );
}

#[test]
fn plan_skips_exists() {
    let (_d, ops) = run_test("Series", &["Ep01.mp4", "Series - Ep01.mp4"], |_| {});
    assert!(matches!(ops[0].status, rename::RenameStatus::SkipExists));
}

// rename::execute_plan

#[test]
fn execute_renames_files() {
    let (d, mut ops) = run_test("Series", &["Ep01_720p.mp4"], |_| {});
    rename::execute_plan(&mut ops, false).unwrap();
    assert!(!d.path().join("Ep01_720p.mp4").exists());
    assert!(d.path().join("Series - Ep01.mp4").exists());
}

#[test]
fn execute_dry_run_does_not_rename() {
    let (d, mut ops) = run_test("Series", &["Ep01_720p.mp4"], |_| {});
    rename::execute_plan(&mut ops, true).unwrap();
    assert!(d.path().join("Ep01_720p.mp4").exists());
}

// Full integration

#[test]
fn full_cycle_with_prefix() {
    let (d, mut ops) = run_test(
        "Title",
        &[
            "PREFIX_TITLE_E01_720p.mp4",
            "PREFIX_TITLE_E02_720p.mp4",
            "PREFIX_TITLE_E11_End_1080p.mp4",
        ],
        |c| c.prefix = "PREFIX_TITLE_".into(),
    );
    rename::execute_plan(&mut ops, false).unwrap();
    assert!(d.path().join("Title - E01.mp4").exists());
    assert!(d.path().join("Title - E02.mp4").exists());
    assert!(d.path().join("Title - E11 End.mp4").exists());
}

#[test]
fn full_cycle_with_prefix_suffix() {
    let (d, mut ops) = run_test("Title", &["SITE_SHOW_sp_ep_2_LANG.mp4"], |c| {
        c.prefix = "SITE_SHOW_".into();
        c.suffix = "_LANG".into();
    });
    rename::execute_plan(&mut ops, false).unwrap();
    assert!(d.path().join("Title - sp ep 2.mp4").exists());
}

// Regression

#[test]
fn empty_name_after_strip_is_not_silently_dropped() {
    let (_d, ops) = run_test("Some Title", &["REMOVEME.mp4"], |c| {
        c.prefix = "REMOVEME".into()
    });
    assert_eq!(ops.len(), 1);
    assert!(matches!(ops[0].status, rename::RenameStatus::SkipEmptyName));
}

#[test]
fn collision_auto_number_renames_instead_of_skipping() {
    let (_d, ops) = run_test("Series", &["Ep01_x264.mp4", "Ep01_720p.mp4"], |c| {
        c.collision_auto_num = true
    });
    let pending: Vec<_> = ops
        .iter()
        .filter(|o| matches!(o.status, rename::RenameStatus::Pending))
        .collect();
    assert_eq!(pending.len(), 2);
    let names: std::collections::HashSet<String> = pending
        .iter()
        .map(|o| o.to.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(names.contains("Series - Ep01.mp4"));
    assert!(names.contains("Series - Ep01 (2).mp4"));
}

#[test]
fn collision_auto_number_also_avoids_names_already_on_disk() {
    let (_d, ops) = run_test(
        "Series",
        &[
            "Ep01_720p.mp4",
            "Series - Ep01.mp4",
            "Series - Ep01 (2).mp4",
        ],
        |c| c.collision_auto_num = true,
    );
    let op = ops
        .iter()
        .find(|o| o.from.file_name().unwrap() == "Ep01_720p.mp4")
        .unwrap();
    assert!(matches!(op.status, rename::RenameStatus::Pending));
    assert_eq!(
        op.to.file_name().unwrap().to_string_lossy(),
        "Series - Ep01 (3).mp4"
    );
}

#[test]
fn collision_detection_is_case_insensitive() {
    let (_d, ops) = run_test("Title", &["ep1.mp4", "EP1.MP4"], |_| {});
    assert_eq!(ops.len(), 2);
    assert_eq!(
        ops.iter()
            .filter(|o| matches!(o.status, rename::RenameStatus::SkipCollision))
            .count(),
        1
    );
}

// Misc

#[test]
fn default_config_extensions_are_not_empty() {
    let config = RenameConfig::default();
    assert!(!config.extensions.is_empty());
    assert!(config.extensions.contains(&"mp4".to_string()));
}
