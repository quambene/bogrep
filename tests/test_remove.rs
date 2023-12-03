use assert_cmd::Command;
use bogrep::{json, utils, BookmarksJson};
use predicates::str;
use tempfile::tempdir;

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_remove() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    let url1 = "https://test_url1.com";
    let url2 = "https://test_url2.com";
    let url3 = "https://test_url3.com";

    println!("Execute 'bogrep add {url1} {url2} {url3}'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["add", url1, url2, url3]);
    cmd.output().unwrap();

    println!("Execute 'bogrep remove {url1} {url2}'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["remove", url1, url2]);
    // Info messages are logged to stderr.
    cmd.assert()
        .success()
        .stdout(str::contains("Removed 2 bookmarks"));

    // Lock file was cleaned up.
    let bookmarks_lock_path = temp_path.join("bookmarks-lock.json");
    assert!(!bookmarks_lock_path.exists());

    let bookmarks_path = temp_path.join("bookmarks.json");
    assert!(
        bookmarks_path.exists(),
        "Missing path: {}",
        bookmarks_path.display()
    );

    let bookmarks = utils::read_file(&bookmarks_path).unwrap();
    let res = json::deserialize::<BookmarksJson>(&bookmarks);
    assert!(res.is_ok());

    let bookmarks = res.unwrap();
    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks.get(0).unwrap().url, url3);
}
