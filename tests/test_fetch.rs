mod common;

use assert_cmd::Command;
use tempfile::tempdir;

#[tokio::test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
async fn test_fetch() {
    common::start_mock_server().await;

    let source = "./test_data/source/bookmarks_simple_localhost.txt";
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    println!("Execute 'bogrep config --source {source}'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["config", "--source", source]);
    cmd.output().unwrap();

    let bookmarks = common::test_bookmarks(&temp_dir);
    assert!(bookmarks.bookmarks.is_empty());

    println!("Execute 'bogrep import'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["import"]);
    cmd.output().unwrap();

    let bookmarks = common::test_bookmarks(&temp_dir);
    assert_eq!(bookmarks.bookmarks.len(), 3);
    for bookmark in bookmarks.bookmarks {
        assert!(bookmark.last_cached.is_none());
    }

    println!("Execute 'bogrep fetch'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["fetch"]);
    cmd.output().unwrap();

    let bookmarks = common::test_bookmarks(&temp_dir);
    assert_eq!(bookmarks.bookmarks.len(), 3);
    for bookmark in bookmarks.bookmarks {
        assert!(bookmark.last_cached.is_some());
    }
}
