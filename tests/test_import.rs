use assert_cmd::Command;
use bogrep::{json, test_utils, utils, JsonBookmarks};
use predicates::{prelude::PredicateBooleanExt, str};
use std::{collections::HashSet, fs, io::Write, path::Path};
use tempfile::tempdir;

fn test_import(source_path: &str, home_path: &Path) {
    println!("Execute 'bogrep -vv config --source {source_path}'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", home_path);
    cmd.args(["-vv", "config", "--source", source_path]);
    cmd.output().unwrap();

    println!("Execute 'bogrep -vv import'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", home_path);
    cmd.args(["-vv", "import"]);
    // Info messages are logged to stderr.
    cmd.assert()
        .success()
        .stdout(str::contains("Imported 4 bookmarks from 1 source"));

    let bookmarks_path = home_path.join("bookmarks.json");
    assert!(
        bookmarks_path.exists(),
        "Missing path: {}",
        bookmarks_path.display()
    );
    // Lock file was cleaned up.
    let bookmarks_lock_path = home_path.join("bookmarks-lock.json");
    assert!(!bookmarks_lock_path.exists());

    let bookmarks = utils::read_file(&bookmarks_path).unwrap();
    let res = json::deserialize::<JsonBookmarks>(&bookmarks);
    assert!(res.is_ok());

    let bookmarks = res.unwrap();
    assert_eq!(bookmarks.len(), 4);
}

#[test]
fn test_import_simple() {
    let source_path = "./test_data/bookmarks_simple.txt";
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    test_import(source_path, temp_path);
}

#[test]
fn test_import_firefox() {
    let source_path = "./test_data/bookmarks_firefox.json";
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    test_import(source_path, temp_path);
}

#[test]
fn test_import_firefox_compressed() {
    let source_path = "./test_data/bookmarks_firefox.jsonlz4";
    test_utils::create_compressed_json_file(Path::new(source_path)).unwrap();
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    test_import(source_path, temp_path);
}

#[test]
fn test_import_firefox_bookmark_folder_ubuntu() {
    let source_path = "./test_data/bookmarks_firefox.jsonlz4";
    test_utils::create_compressed_json_file(Path::new(source_path)).unwrap();
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    fs::create_dir_all(temp_path.join("firefox")).unwrap();
    fs::copy(
        source_path,
        temp_path.join("firefox").join("bookmarks_firefox.jsonlz4"),
    )
    .unwrap();

    test_import(temp_path.join("firefox").to_str().unwrap(), temp_path);
}

#[test]
fn test_import_firefox_bookmark_folder_macos() {
    let source_path = "./test_data/bookmarks_firefox.jsonlz4";
    test_utils::create_compressed_json_file(Path::new(source_path)).unwrap();
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    fs::create_dir_all(temp_path.join("Firefox")).unwrap();
    fs::copy(
        source_path,
        temp_path.join("Firefox").join("bookmarks_firefox.jsonlz4"),
    )
    .unwrap();

    test_import(temp_path.join("Firefox").to_str().unwrap(), temp_path);
}

#[test]
fn test_import_chrome() {
    let source_path = "./test_data/bookmarks_chromium.json";
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    test_import(source_path, temp_path);
}

// Test renaming of `bookmarks-lock.json` to `bookmarks.json`.
#[test]
fn test_import_consecutive() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    let source_path = temp_path.join("bookmarks_simple.txt");
    let mut source_file = utils::open_and_truncate_file(&source_path).unwrap();

    println!("Execute 'bogrep config --source {}'", source_path.display());
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["config", "--source", source_path.to_str().unwrap()]);
    cmd.output().unwrap();

    let source_bookmarks: HashSet<String> = HashSet::from_iter([
        "https://www.deepl.com/translator".to_owned(),
        "https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"
            .to_owned(),
        "https://en.wikipedia.org/wiki/Design_Patterns".to_owned(),
        "https://doc.rust-lang.org/book/title-page.html".to_owned(),
    ]);

    for bookmark in source_bookmarks.iter() {
        writeln!(source_file, "{}", bookmark).unwrap();
    }

    println!("Execute 'bogrep import'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["import"]);
    // Info messages are logged to stderr.
    cmd.assert()
        .success()
        .stdout(str::contains("Imported 4 bookmarks from 1 source"));

    // Truncate file and simulate change of source bookmarks.
    let mut source_file = utils::open_and_truncate_file(&source_path).unwrap();
    let source_bookmarks: HashSet<String> =
        HashSet::from_iter(["https://doc.rust-lang.org/book/title-page.html".to_owned()]);

    for bookmark in source_bookmarks.iter() {
        writeln!(source_file, "{}", bookmark).unwrap();
    }

    println!("Execute 'bogrep import'");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.env("BOGREP_HOME", temp_path);
    cmd.args(["import"]);
    // Info messages are logged to stderr.
    cmd.assert().success().stdout(
        str::contains("Imported 0 bookmarks from 1 source")
            .and(str::contains("Removed 3 bookmarks")),
    );

    let bookmarks_path = temp_path.join("bookmarks.json");
    assert!(bookmarks_path.exists());
    // Lock file was cleaned up.
    let bookmarks_lock_path = temp_path.join("bookmarks-lock.json");
    assert!(!bookmarks_lock_path.exists());

    let bookmarks = utils::read_file(&bookmarks_path).unwrap();
    let bookmarks = json::deserialize::<JsonBookmarks>(&bookmarks).unwrap();
    assert_eq!(bookmarks.len(), 1);
}
