mod common;

use assert_cmd::Command;
use bogrep::utils;
use predicates::{prelude::PredicateBooleanExt, str};
use std::{
    fs::{self, File},
    io::Write,
};
use tempfile::tempdir;

#[tokio::test]
async fn test_fetch() {
    let request_throttling = "1";
    let mock_server = common::start_mock_server().await;
    let mocks = common::mount_mocks(&mock_server, 3).await;
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    let source_path = temp_path.join("test_data");
    fs::create_dir_all(&source_path).unwrap();
    let source_path = &source_path.join("bookmarks_simple.txt");
    let mut file = File::create(source_path).unwrap();

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
        "-v",
        "config",
        "--source",
        source_path.to_str().unwrap(),
        "--request-throttling",
        request_throttling,
    ]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    let bookmarks = common::test_bookmarks(temp_path);
    assert!(bookmarks.is_empty());

    println!("Execute 'bogrep -v import'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["-v", "import"]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    let bookmarks = common::test_bookmarks(temp_path);
    assert_eq!(bookmarks.len(), 3);
    for bookmark in bookmarks {
        assert!(bookmark.last_cached.is_none());
    }

    println!("Execute 'bogrep -v fetch'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["-v", "fetch"]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    let bookmarks_before = common::test_bookmarks(temp_path);
    assert_eq!(bookmarks_before.len(), 3);
    for bookmark in &bookmarks_before {
        assert!(bookmark.last_cached.is_some());
    }

    // Verify cache
    for bookmark in &bookmarks_before {
        let cache_path = temp_path.join(format!("cache/{}.txt", bookmark.id));
        let actual_content = fs::read_to_string(&cache_path).unwrap();
        let expected_content = mocks.get(&bookmark.url).unwrap();
        assert_eq!(&actual_content, expected_content);
    }

    // Truncate file and simulate change of source bookmarks.
    let mut source_file = utils::open_and_truncate_file(source_path).unwrap();
    for url in mocks.keys().take(1) {
        writeln!(source_file, "{}", url).unwrap();
    }

    println!("Execute 'bogrep -v import'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["-v", "import"]);
    // Info messages are logged to stderr.
    cmd.assert().success().stdout(
        str::contains("Imported 0 bookmarks from 1 source")
            .and(str::contains("Removed 2 bookmarks")),
    );

    let bookmarks_after = common::test_bookmarks(temp_path);
    assert_eq!(bookmarks_after.len(), 1);
    for bookmark in &bookmarks_after {
        assert!(bookmark.last_cached.is_some());
        let cache_path = temp_path.join(format!("cache/{}.txt", bookmark.id));
        assert!(cache_path.exists());
    }

    let mut bookmarks_before = bookmarks_before;
    bookmarks_before
        .bookmarks
        .retain(|bookmark| !bookmarks_after.bookmarks.contains(bookmark));

    // Bookmarks are removed from cache
    for bookmark in &bookmarks_before {
        let cache_path = temp_path.join(format!("cache/{}.txt", bookmark.id));
        assert!(!cache_path.exists());
    }
}

#[tokio::test]
async fn test_fetch_diff() {
    let request_throttling = "1";
    let mock_server = common::start_mock_server().await;
    let mock_website_1 = common::mount_mock_scoped(&mock_server, 1, 10).await;
    let mock_website_2 = common::mount_mock_scoped(&mock_server, 2, 20).await;
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    let source_path = temp_path.join("test_data");
    let source = &source_path.join("bookmarks_simple.txt");
    fs::create_dir_all(&source_path).unwrap();
    let mut bookmarks_file = File::create(source).unwrap();

    writeln!(bookmarks_file, "{}", mock_website_1.url).unwrap();
    writeln!(bookmarks_file, "{}", mock_website_2.url).unwrap();

    println!(
        "Execute 'bogrep config --source {} --request-throttling {request_throttling}'",
        source.display()
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

    println!("Execute 'bogrep import'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["import"]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    println!("Execute 'bogrep fetch'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["fetch"]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    // Change the content for the mock website to simulate a diff.
    drop(mock_website_1);
    drop(mock_website_2);
    let mock_website_1 = common::mount_mock_scoped(&mock_server, 1, 11).await;
    let mock_website_2 = common::mount_mock_scoped(&mock_server, 2, 21).await;
    writeln!(bookmarks_file, "{}", mock_website_1.url).unwrap();
    writeln!(bookmarks_file, "{}", mock_website_2.url).unwrap();

    let bookmarks = common::test_bookmarks(temp_path);

    println!("Execute 'bogrep fetch --diff'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args([
        "fetch",
        "--diff",
        &bookmarks.get(0).unwrap().url,
        &bookmarks.get(1).unwrap().url,
    ]);
    cmd.assert().success().stdout(
        str::contains("-Test content 10+Test content 11")
            .and(str::contains("-Test content 20+Test content 21")),
    );
}

// Test fetching if the cache directory was removed manually, leading to
// inconsistent state as the target bookmarks are still marked as last cached.
#[tokio::test]
async fn test_fetch_empty_cache() {
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
        source.display()
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
    for bookmark in bookmarks {
        assert!(bookmark.last_cached.is_none());
    }

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

    // Verify cache
    for bookmark in &bookmarks {
        let cache_path = temp_path.join(format!("cache/{}.txt", bookmark.id));
        let actual_content = fs::read_to_string(&cache_path).unwrap();
        let expected_content = mocks.get(&bookmark.url).unwrap();
        assert_eq!(&actual_content, expected_content);

        // Clear cache
        fs::remove_file(cache_path).unwrap();
    }

    let cache_dir = temp_path.join("cache");
    let entries = fs::read_dir(cache_dir);
    assert!(entries.is_ok_and(|mut file| file.next().is_none()));

    println!("Execute 'bogrep fetch'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["fetch"]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    // Verify cache
    for bookmark in &bookmarks {
        let cache_path = temp_path.join(format!("cache/{}.txt", bookmark.id));
        let actual_content = fs::read_to_string(&cache_path).unwrap();
        let expected_content = mocks.get(&bookmark.url).unwrap();
        assert_eq!(&actual_content, expected_content);
    }
}
