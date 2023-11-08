mod common;

use assert_cmd::Command;
use bogrep::{json, utils, TargetBookmarks};
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};
use tempfile::tempdir;
use uuid::Uuid;

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

    println!("Execute 'bogrep fetch --mode markdown'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["fetch", "--mode", "markdown"]);
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

        let cache_path = temp_path.join(format!("cache/{}.md", bookmark.id));
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

        let cache_path = temp_path.join(format!("cache/{}.md", bookmark.id));
        assert!(cache_path.exists());

        let cache_path = temp_path.join(format!("cache/{}.html", bookmark.id));
        assert!(cache_path.exists());
    }
}

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_clean_all() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    let cache_path = temp_path.join("cache");
    fs::create_dir(&cache_path).unwrap();
    assert!(
        cache_path.exists(),
        "Missing path: {}",
        cache_path.display()
    );

    let text_file_name = PathBuf::from(Uuid::new_v4().to_string()).with_extension("txt");
    let markdown_file_name = PathBuf::from(Uuid::new_v4().to_string()).with_extension("md");
    let html_file_name = PathBuf::from(Uuid::new_v4().to_string()).with_extension("html");

    let text_file_path = cache_path.join(&text_file_name);
    let markdown_file_path = cache_path.join(&markdown_file_name);
    let html_file_path = cache_path.join(&html_file_name);

    File::create(&text_file_path).unwrap();
    File::create(&markdown_file_path).unwrap();
    File::create(&html_file_path).unwrap();

    assert!(text_file_path.exists());
    assert!(markdown_file_path.exists());
    assert!(html_file_path.exists());

    println!("Execute 'bogrep clean --all'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["clean", "--all"]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    assert!(!cache_path.exists());
    assert!(!text_file_path.exists());
    assert!(!markdown_file_path.exists());
    assert!(!html_file_path.exists());
}
