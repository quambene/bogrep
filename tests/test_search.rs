use assert_cmd::Command;
use predicates::str;
use tempfile::tempdir;

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_search() {
    let source = "./test_data/source/bookmarks_simple.txt";
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    println!("Execute 'bogrep config --source {source}'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["config", "--source", source]);
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

    println!("Execute 'bogrep \"test content 1\"'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.arg("test content 1");
    // Info messages are logged to stderr.
    cmd.assert()
        .success()
        .stderr(str::contains("Match in bookmark"));
}
