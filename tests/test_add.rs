use assert_cmd::Command;
use bogrep::{json, utils, JsonBookmarks, SourceType};
use predicates::str;
use tempfile::tempdir;

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_add() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    let url1 = "https://test_url1.com";
    let url2 = "https://test_url2.com";

    println!("Execute 'bogrep add {url1} {url2}'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["add", url1, url2]);
    cmd.assert()
        .success()
        .stdout(str::contains("Added 2 bookmarks"));

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
    let res = json::deserialize::<JsonBookmarks>(&bookmarks);
    assert!(res.is_ok());

    let bookmarks = res.unwrap();
    assert_eq!(bookmarks.len(), 2);

    for bookmark in bookmarks {
        assert!(bookmark.sources.contains(&SourceType::Internal))
    }
}
