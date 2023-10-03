mod common;

use assert_cmd::Command;
use std::{fs, path::Path};
use tempfile::tempdir;

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_configure() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    let source = "./test_data/source/bookmarks_simple.txt";
    let source_path = fs::canonicalize(&source).unwrap();

    // Use `assert_cmd::Command::cargo_bin()` instead of `std::process::Command::new()`.
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["config", "--source", source]);

    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    let settings_path = temp_dir.path().join("settings.json");
    assert!(
        settings_path.exists(),
        "Missing path: {}",
        settings_path.display()
    );

    let bookmarks_path = temp_dir.path().join("bookmarks.json");
    assert!(
        bookmarks_path.exists(),
        "Missing path: {}",
        bookmarks_path.display()
    );

    let cache_path = temp_dir.path().join("cache");
    assert!(
        cache_path.exists(),
        "Missing path: {}",
        cache_path.display()
    );

    let (actual_settings, expected_settings) = common::compare_files(
        &settings_path,
        Path::new("test_data/configure/settings.json"),
    );
    let expected_settings =
        expected_settings.replace("path/to/bookmarks", source_path.to_str().unwrap());
    assert_eq!(actual_settings, expected_settings);

    let (actual_bookmarks, expected_bookmarks) = common::compare_files(
        &bookmarks_path,
        Path::new("test_data/target/bookmarks_empty.json"),
    );
    assert_eq!(actual_bookmarks, expected_bookmarks);
}
