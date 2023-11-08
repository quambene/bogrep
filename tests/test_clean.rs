use assert_cmd::Command;
use std::{
    fs::{self, File},
    path::PathBuf,
};
use tempfile::tempdir;
use uuid::Uuid;

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_clean() {
    todo!()
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
