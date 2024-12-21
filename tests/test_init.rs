use assert_cmd::Command;
use predicates::str;
use std::{env, fs, path::Path};
use tempfile::tempdir;

#[test]
fn test_init() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    let home_path = temp_path.join("home");
    let source_file = Path::new("./test_data/bookmarks_chromium_no_extension");
    let bookmark_dir = home_path.join("snap/chromium/common/chromium/Default");
    let bookmark_file = bookmark_dir.join("Bookmarks");

    fs::create_dir_all(&bookmark_dir).unwrap();
    fs::copy(source_file, bookmark_file).unwrap();

    println!("Execute 'bogrep init'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path)
        .env("HOME", home_path)
        .args(["init"])
        .write_stdin("yes")
        .assert()
        .success()
        .stdout(str::contains("Found sources:"));
}
