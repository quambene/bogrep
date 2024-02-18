use assert_cmd::Command;
use bogrep::{json, utils, JsonBookmarks, Settings};
use std::path::Path;
use tempfile::tempdir;

fn test_configure_source(temp_path: &Path, source: &str, folder: Option<&str>) {
    if let Some(folder) = folder {
        println!("Execute 'bogrep config --source {source} --folders {folder}'");
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        cmd.env("BOGREP_HOME", temp_path);
        cmd.args(["config", "--source", source, "--folders", folder]);
        let res = cmd.output();
        assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());
    } else {
        println!("Execute 'bogrep config --source {source}'");
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        cmd.env("BOGREP_HOME", temp_path);
        cmd.args(["config", "--source", source]);
        let res = cmd.output();
        assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());
    }

    let settings_path = temp_path.join("settings.json");
    assert!(
        settings_path.exists(),
        "Missing path: {}",
        settings_path.display()
    );

    let bookmarks_path = temp_path.join("bookmarks.json");
    assert!(
        bookmarks_path.exists(),
        "Missing path: {}",
        bookmarks_path.display()
    );

    let cache_path = temp_path.join("cache");
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

    if folder.is_some() {
        for source in settings.sources {
            assert!(!source.folders.is_empty());
        }
    }

    let bookmarks = utils::read_file(&bookmarks_path).unwrap();
    let res = json::deserialize::<JsonBookmarks>(&bookmarks);
    assert!(res.is_ok());

    let bookmarks = res.unwrap();
    assert!(bookmarks.is_empty());
}

#[test]
fn test_configure_source_simple() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    let source = "./test_data/bookmarks_simple.txt";
    test_configure_source(temp_path, source, None);
}

#[test]
fn test_configure_source_firefox() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    let source = "./test_data/bookmarks_firefox.json";
    test_configure_source(temp_path, source, None);
}

#[test]
fn test_configure_source_chrome() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    let source = "./test_data/bookmarks_chromium.json";
    test_configure_source(temp_path, source, None);
}

#[test]
fn test_configure_source_chrome_no_extension() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    let source = "./test_data/bookmarks_chromium_no_extension";
    test_configure_source(temp_path, source, None);
}

#[test]
fn test_configure_source_firefox_consecutive() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    let source = "./test_data/bookmarks_firefox.json";
    let folder = Some("science");
    test_configure_source(temp_path, source, folder);

    let folder = Some("dev");
    test_configure_source(temp_path, source, folder);
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

#[test]
fn test_configure_underlying_urls() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    let url1 = "https://news.ycombinator.com";
    let url2 = "https://www.reddit.com";

    println!("Execute 'bogrep config --underlying {url1} {url2}'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["config", "--underlying", url1, url2]);
    let res = cmd.output();
    assert!(res.is_ok(), "Can't execute command: {}", res.unwrap_err());

    let settings_path = temp_dir.path().join("settings.json");
    let settings = utils::read_file(&settings_path).unwrap();
    let res = json::deserialize::<Settings>(&settings);
    assert!(res.is_ok());

    let settings = res.unwrap();
    assert!(!settings.underlying_urls.is_empty());
}
