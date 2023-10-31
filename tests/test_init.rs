mod common;

use assert_cmd::Command;
use std::{
    fs::{self, File},
    io::Write,
};
use tempfile::tempdir;

#[tokio::test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
async fn test_init() {
    let mocks = common::start_mock_server(3).await;
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    let source_path = temp_path.join("test_data/source/");
    let source = &source_path.join("bookmarks_simple.txt");
    fs::create_dir_all(&source_path).unwrap();
    let mut file = File::create(source).unwrap();

    for url in mocks.keys() {
        writeln!(file, "{}", url).unwrap();
    }

    println!("Execute 'bogrep config --source {}'", source.display());
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["config", "--source", source.to_str().unwrap()]);
    cmd.output().unwrap();

    let bookmarks = common::test_bookmarks(&temp_dir);
    assert!(bookmarks.bookmarks.is_empty());

    println!("Execute 'bogrep init'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["init"]);
    cmd.output().unwrap();

    let bookmarks = common::test_bookmarks(&temp_dir);
    assert_eq!(bookmarks.bookmarks.len(), 3);
    for bookmark in &bookmarks.bookmarks {
        assert!(bookmark.last_cached.is_some());
    }

    // Verify cache
    for bookmark in bookmarks.bookmarks {
        let cache_path = temp_path.join(format!("cache/{}.txt", bookmark.id));
        let actual_content = fs::read_to_string(&cache_path).unwrap();
        let expected_content = mocks.get(&bookmark.url).unwrap();
        assert_eq!(&actual_content, expected_content);
    }
}
