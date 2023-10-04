use assert_cmd::Command;
use bogrep::{json, utils, Settings, TargetBookmarks};
use tempfile::tempdir;

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_configure() {
    let source = "./test_data/source/bookmarks_simple.txt";
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["config", "--source", source]);

    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    let settings_path = temp_dir.path().join("settings.json");
    assert!(
        settings_path.exists(),
        "Missing path: {}",
        settings_path.display()
    );

    let bookmarks_path = temp_dir.path().join("bookmarks.json");
    assert!(
        bookmarks_path.exists(),
        "Missing path: {}",
        bookmarks_path.display()
    );

    let cache_path = temp_dir.path().join("cache");
    assert!(
        cache_path.exists(),
        "Missing path: {}",
        cache_path.display()
    );

    let settings = utils::read_file(&settings_path).unwrap();
    let res = json::deserialize::<Settings>(&settings);
    assert!(res.is_ok());

    let settings = res.unwrap();
    assert!(!settings.sources.is_empty());

    let bookmarks = utils::read_file(&bookmarks_path).unwrap();
    let res = json::deserialize::<TargetBookmarks>(&bookmarks);
    assert!(res.is_ok());

    let bookmarks = res.unwrap();
    assert!(bookmarks.bookmarks.is_empty());
}
