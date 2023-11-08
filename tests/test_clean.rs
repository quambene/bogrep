mod common;

use assert_cmd::Command;
use bogrep::{json, utils, TargetBookmarks};
use std::{
    fs::{self, File},
    io::Write,
};
use tempfile::tempdir;

#[tokio::test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
async fn test_clean() {
    let mocks = common::start_mock_server(3).await;
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

    println!("Execute 'bogrep import'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["import"]);
    cmd.output().unwrap();

    println!("Execute 'bogrep fetch --mode text'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["fetch", "--mode", "text"]);
    cmd.output().unwrap();

    println!("Execute 'bogrep fetch --mode html'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["fetch", "--mode", "html"]);
    cmd.output().unwrap();

    let bookmarks_path = temp_dir.path().join("bookmarks.json");
    let bookmarks = utils::read_file(&bookmarks_path).unwrap();
    let bookmarks = json::deserialize::<TargetBookmarks>(&bookmarks).unwrap();

    for bookmark in &bookmarks.bookmarks {
        let cache_path = temp_path.join(format!("cache/{}.txt", bookmark.id));
        assert!(cache_path.exists());

        let cache_path = temp_path.join(format!("cache/{}.html", bookmark.id));
        assert!(cache_path.exists());
    }

    println!("Execute 'bogrep clean'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["clean"]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    for bookmark in &bookmarks.bookmarks {
        let cache_path = temp_path.join(format!("cache/{}.txt", bookmark.id));
        // Text files are now deleted.
        assert!(!cache_path.exists());

        let cache_path = temp_path.join(format!("cache/{}.html", bookmark.id));
        assert!(cache_path.exists());
    }
}

#[tokio::test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
async fn test_clean_all() {
    let mocks = common::start_mock_server(3).await;
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    let cache_path = temp_path.join("cache");
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

    println!("Execute 'bogrep import'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["import"]);
    cmd.output().unwrap();

    println!("Execute 'bogrep fetch --mode text'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["fetch", "--mode", "text"]);
    cmd.output().unwrap();

    println!("Execute 'bogrep fetch --mode html'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["fetch", "--mode", "html"]);
    cmd.output().unwrap();

    let bookmarks_path = temp_dir.path().join("bookmarks.json");
    let bookmarks = utils::read_file(&bookmarks_path).unwrap();
    let bookmarks = json::deserialize::<TargetBookmarks>(&bookmarks).unwrap();

    for bookmark in &bookmarks.bookmarks {
        let cache_file = temp_path.join(format!("cache/{}.txt", bookmark.id));
        assert!(cache_file.exists());

        let cache_file = temp_path.join(format!("cache/{}.html", bookmark.id));
        assert!(cache_file.exists());
    }

    println!("Execute 'bogrep clean --all'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["clean", "--all"]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());
    assert!(cache_path.exists());

    for bookmark in &bookmarks.bookmarks {
        let cache_file = temp_path.join(format!("cache/{}.txt", bookmark.id));
        // Text files are now deleted.
        assert!(!cache_file.exists());

        let cache_file = temp_path.join(format!("cache/{}.html", bookmark.id));
        // HTML files are now deleted.
        assert!(!cache_file.exists());
    }
}
