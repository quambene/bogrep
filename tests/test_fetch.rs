mod common;

use assert_cmd::Command;
use predicates::str;
use std::{
    fs::{self, File},
    io::Write,
};
use tempfile::tempdir;

#[tokio::test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
async fn test_fetch() {
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

    println!("Execute 'bogrep config --source {}'", source.display());
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["config", "--source", source.to_str().unwrap()]);
    cmd.output().unwrap();

    let bookmarks = common::test_bookmarks(&temp_dir);
    assert!(bookmarks.is_empty());

    println!("Execute 'bogrep import'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["import"]);
    cmd.output().unwrap();

    let bookmarks = common::test_bookmarks(&temp_dir);
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

    let bookmarks = common::test_bookmarks(&temp_dir);
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

#[tokio::test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
async fn test_fetch_diff() {
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

    println!("Execute 'bogrep config --source {}'", source.display());
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["config", "--source", source.to_str().unwrap()]);
    cmd.output().unwrap();

    println!("Execute 'bogrep import'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["import"]);
    cmd.output().unwrap();

    println!("Execute 'bogrep fetch'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["fetch"]);
    cmd.output().unwrap();

    // Change the content for the mock website to simulate a diff.
    drop(mock_website_1);
    drop(mock_website_2);
    let mock_website_1 = common::mount_mock_scoped(&mock_server, 1, 11).await;
    let mock_website_2 = common::mount_mock_scoped(&mock_server, 2, 21).await;
    writeln!(bookmarks_file, "{}", mock_website_1.url).unwrap();
    writeln!(bookmarks_file, "{}", mock_website_2.url).unwrap();

    let bookmarks = common::test_bookmarks(&temp_dir);

    println!("Execute 'bogrep fetch --diff'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["fetch", "--diff", &bookmarks[0].url, &bookmarks[1].url]);
    cmd.assert().success().stdout(str::contains(
        "-Test content 10+Test content 11-Test content 20+Test content 21",
    ));
}
