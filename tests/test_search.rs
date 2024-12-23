mod common;

use assert_cmd::Command;
use predicates::{boolean::PredicateBooleanExt, str};
use std::{
    fs::{self, File},
    io::Write,
};
use tempfile::tempdir;

#[tokio::test]
async fn test_search_case_sensitive() {
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

    println!("Execute 'bogrep \"Test content 1\"'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.arg("Test content 1");
    cmd.assert()
        .success()
        .stdout(
            str::contains("Match in bookmark")
                .and(str::contains("endpoint_1"))
                .and(str::contains("Test content 1")),
        )
        .stderr("");

    println!("Execute 'bogrep \"Test content 4\"'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.arg("Test content 4");
    cmd.assert()
        .success()
        .stdout("No matches in bookmarks\n")
        .stderr("");
}

#[tokio::test]
async fn test_search_case_insensitive() {
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

    println!("Execute 'bogrep -i \"test content 1\"'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["-i", "test content 1"]);
    cmd.assert()
        .success()
        .stdout(
            str::contains("Match in bookmark")
                .and(str::contains("endpoint_1"))
                .and(str::contains("Test content 1")),
        )
        .stderr("");

    println!("Execute 'bogrep -i \"test content 4\"'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["-i", "test content 4"]);
    cmd.assert()
        .success()
        .stdout("No matches in bookmarks\n")
        .stderr("");
}

#[tokio::test]
async fn test_search_no_content() {
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

    println!("Execute 'bogrep -l \"Test content 1\"'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["-l", "Test content 1"]);
    cmd.assert()
        .success()
        .stdout(
            str::contains("Match in bookmark")
                .and(str::contains("endpoint_1"))
                .and(str::contains("Test content 1").not()),
        )
        .stderr("");

    println!("Execute 'bogrep -l \"Test content 4\"'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["-l", "Test content 4"]);
    cmd.assert()
        .success()
        .stdout("No matches in bookmarks\n")
        .stderr("");
}
