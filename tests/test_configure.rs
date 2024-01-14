use assert_cmd::Command;
use bogrep::{json, utils, JsonBookmarks, Settings};
use tempfile::tempdir;

fn test_configure_source(source: &str) {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    println!("Execute 'bogrep config --source {source}'");
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
    let res = json::deserialize::<JsonBookmarks>(&bookmarks);
    assert!(res.is_ok());

    let bookmarks = res.unwrap();
    assert!(bookmarks.is_empty());
}

#[test]
fn test_configure_source_simple() {
    let source = "./test_data/bookmarks_simple.txt";
    test_configure_source(source);
}

#[test]
fn test_configure_source_firefox() {
    let source = "./test_data/bookmarks_firefox.json";
    test_configure_source(source);
}

#[test]
fn test_configure_source_chrome() {
    let source = "./test_data/bookmarks_chromium.json";
    test_configure_source(source);
}

#[test]
fn test_configure_source_chrome_no_extension() {
    let source = "./test_data/bookmarks_chromium_no_extension";
    test_configure_source(source);
}

#[test]
fn test_configure_ignored_urls() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    let url1 = "https://url1";
    let url2 = "https://url2";

    println!("Execute 'bogrep config --ignore {url1} {url2}'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["config", "--ignore", url1, url2]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    let settings_path = temp_dir.path().join("settings.json");
    let settings = utils::read_file(&settings_path).unwrap();
    let res = json::deserialize::<Settings>(&settings);
    assert!(res.is_ok());

    let settings = res.unwrap();
    assert!(!settings.ignored_urls.is_empty());
}
