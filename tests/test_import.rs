use assert_cmd::Command;
use bogrep::{json, test_utils, utils, TargetBookmarks};
use predicates::str;
use std::path::Path;
use tempfile::tempdir;

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_import_simple() {
    let source = "./test_data/source/bookmarks_simple.txt";
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

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
        .stderr(str::contains("Imported 3 bookmarks from 1 source"));

    let bookmarks_path = temp_dir.path().join("bookmarks.json");
    assert!(
        bookmarks_path.exists(),
        "Missing path: {}",
        bookmarks_path.display()
    );

    let bookmarks = utils::read_file(&bookmarks_path).unwrap();
    let res = json::deserialize::<TargetBookmarks>(&bookmarks);
    assert!(res.is_ok());

    let bookmarks = res.unwrap();
    assert_eq!(bookmarks.bookmarks.len(), 3);
}

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_import_firefox() {
    let source = "./test_data/source/bookmarks_firefox.jsonlz4";
    test_utils::create_compressed_bookmarks(Path::new(source));
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

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

    let bookmarks_path = temp_dir.path().join("bookmarks.json");
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
fn test_import_chrome() {
    let source = "./test_data/source/bookmarks_google-chrome.json";
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

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

    let bookmarks_path = temp_dir.path().join("bookmarks.json");
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
