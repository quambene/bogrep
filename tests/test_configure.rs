mod common;

use std::{
    env::{self},
    path::Path,
    process::Command,
};
use tempfile::tempdir;

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_configure() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    let project_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let source_path = format!(
        "{}/test_data/source/bookmarks_simple.txt",
        project_dir.to_string_lossy()
    );

    let mut cmd = Command::new("target/debug/bogrep");
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["config", "--source", &source_path]);

    let res = cmd.output();
    assert!(res.is_ok(), "{}", res.unwrap_err());

    let settings_path = temp_dir.path().join("settings.json");
    assert!(settings_path.exists(), "{}", settings_path.display());

    let bookmarks_path = temp_dir.path().join("bookmarks.json");
    assert!(bookmarks_path.exists(), "{}", bookmarks_path.display());

    let cache_path = temp_dir.path().join("cache");
    assert!(cache_path.exists(), "{}", cache_path.display());

    let (actual_settings, expected_settings) = common::compare_files(
        &settings_path,
        Path::new("test_data/configure/settings.json"),
    );
    let expected_settings = expected_settings.replace("path/to/bookmarks", &source_path);
    assert_eq!(actual_settings, expected_settings);

    let (actual_bookmarks, expected_bookmarks) = common::compare_files(
        &bookmarks_path,
        Path::new("test_data/target/bookmarks_empty.json"),
    );
    assert_eq!(actual_bookmarks, expected_bookmarks);
}
