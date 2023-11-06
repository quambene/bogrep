use assert_cmd::Command;
use bogrep::{json, test_utils, utils, TargetBookmarks};
use predicates::str;
use std::{fs, path::Path};
use tempfile::tempdir;

fn test_import(source: &str, temp_path: &Path) {
    println!("Execute 'bogrep config --source {source}'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["config", "--source", source]);
    cmd.output().unwrap();

    println!("Execute 'bogrep import'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["import"]);
    // Info messages are logged to stderr.
    cmd.assert()
        .success()
        .stderr(str::contains("Imported 4 bookmarks from 1 source"));

    let bookmarks_path = temp_path.join("bookmarks.json");
    assert!(
        bookmarks_path.exists(),
        "Missing path: {}",
        bookmarks_path.display()
    );

    let bookmarks = utils::read_file(&bookmarks_path).unwrap();
    let res = json::deserialize::<TargetBookmarks>(&bookmarks);
    assert!(res.is_ok());

    let bookmarks = res.unwrap();
    assert_eq!(bookmarks.bookmarks.len(), 4);
}

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_import_simple() {
    let source = "./test_data/bookmarks_simple.txt";
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    test_import(source, temp_path);
}

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_import_firefox() {
    let source = "./test_data/bookmarks_firefox.json";
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    test_import(source, temp_path);
}

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_import_firefox_compressed() {
    let source = "./test_data/bookmarks_firefox.jsonlz4";
    test_utils::create_compressed_bookmarks(Path::new(source));
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    test_import(source, temp_path);
}

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_import_firefox_bookmark_folder_ubuntu() {
    let source_path = "./test_data/bookmarks_firefox.jsonlz4";
    test_utils::create_compressed_bookmarks(Path::new(source_path));
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    fs::create_dir_all(temp_path.join("firefox")).unwrap();
    fs::copy(
        source_path,
        temp_path.join("firefox").join("bookmarks_firefox.jsonlz4"),
    )
    .unwrap();

    test_import(temp_path.join("firefox").to_str().unwrap(), temp_path);
}

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_import_firefox_bookmark_folder_macos() {
    let source_path = "./test_data/bookmarks_firefox.jsonlz4";
    test_utils::create_compressed_bookmarks(Path::new(source_path));
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    fs::create_dir_all(temp_path.join("Firefox")).unwrap();
    fs::copy(
        source_path,
        temp_path.join("Firefox").join("bookmarks_firefox.jsonlz4"),
    )
    .unwrap();

    test_import(temp_path.join("Firefox").to_str().unwrap(), temp_path);
}

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_import_chrome() {
    let source = "./test_data/bookmarks_chrome.json";
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    test_import(source, temp_path);
}
