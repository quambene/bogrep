use std::{env, process::Command};

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_configure() {
    let project_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let source_path = format!(
        "{}/test_data/source/bookmarks_simple.txt",
        project_dir.to_string_lossy()
    );

    let mut cmd = Command::new("target/debug/bogrep");
    cmd.env("BOGREP_HOME", "test_data/bogrep");
    cmd.args(["config", "--source", &source_path]);

    let res = cmd.output();
    assert!(res.is_ok(), "{}", res.unwrap_err());
}
