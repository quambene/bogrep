mod common;

use assert_cmd::Command;
use std::{
    fs::{self, File},
    io::Write,
};
use tempfile::tempdir;

#[tokio::test]
async fn test_sync() {
    let request_throttling = "1";
    let mock_server = common::start_mock_server().await;
    let mocks = common::mount_mocks(&mock_server, 3).await;
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    let source_path = temp_path.join("test_data");
    let source = &source_path.join("bookmarks_simple.txt");
    fs::create_dir_all(&source_path).unwrap();
    let mut file = File::create(source).unwrap();

    for url in mocks.keys() {
        writeln!(file, "{}", url).unwrap();
    }

    println!(
        "Execute 'bogrep config --source {} --request-throttling {request_throttling}'",
        source_path.display()
    );
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args([
        "config",
        "--source",
        source.to_str().unwrap(),
        "--request-throttling",
        request_throttling,
    ]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    let bookmarks = common::test_bookmarks(temp_path);
    assert!(bookmarks.is_empty());

    println!("Execute 'bogrep import'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["import"]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    let bookmarks = common::test_bookmarks(temp_path);
    assert_eq!(bookmarks.len(), 3);

    println!("Execute 'bogrep fetch'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["fetch"]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    let bookmarks = common::test_bookmarks(temp_path);
    assert_eq!(bookmarks.len(), 3);
    for bookmark in &bookmarks {
        assert!(bookmark.last_cached.is_some());
    }

    println!("Execute 'bogrep sync'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["sync"]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    let bookmarks = common::test_bookmarks(temp_path);
    assert_eq!(bookmarks.len(), 3);
    for bookmark in &bookmarks {
        assert!(bookmark.last_cached.is_some());
    }

    // Verify cache
    for bookmark in bookmarks {
        let cache_path = temp_path.join(format!("cache/{}.txt", bookmark.id));
        let actual_content = fs::read_to_string(&cache_path).unwrap();
        let expected_content = mocks.get(&bookmark.url).unwrap();
        assert_eq!(&actual_content, expected_content);
    }
}
