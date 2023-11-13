mod common;

use bogrep::{json, utils, TargetBookmarks};
use std::{fs, io::Write};
use tempfile::tempdir;

#[test]
fn test_rename() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    let bookmarks_path = temp_path.join("bookmarks.json");
    let bookmarks_lock_path = temp_path.join("bookmarks-lock.json");

    {
        let bookmarks = TargetBookmarks::default();
        let bookmarks_json = json::serialize(&bookmarks).unwrap();
        let mut bookmarks_file = utils::open_file_in_read_write_mode(&bookmarks_path).unwrap();
        bookmarks_file.write_all(&bookmarks_json).unwrap();
    }

    let _bookmarks_file = utils::open_file_in_read_mode(&bookmarks_path).unwrap();
    let _bookmarks_lock_file = utils::open_and_truncate_file(&bookmarks_lock_path).unwrap();

    assert!(!bookmarks_path.exists());
    assert!(!bookmarks_lock_path.exists());

    let res = fs::rename(&bookmarks_lock_path, &bookmarks_path);
    assert!(res.is_ok(), "{}", res.unwrap_err());
}

#[test]
fn test_rename_drop() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    let bookmarks_path = temp_path.join("bookmarks.json");
    let bookmarks_lock_path = temp_path.join("bookmarks-lock.json");

    {
        let bookmarks = TargetBookmarks::default();
        let bookmarks_json = json::serialize(&bookmarks).unwrap();
        let mut bookmarks_file = utils::open_file_in_read_write_mode(&bookmarks_path).unwrap();
        bookmarks_file.write_all(&bookmarks_json).unwrap();
    }

    assert!(!bookmarks_path.exists());
    assert!(!bookmarks_lock_path.exists());

    let res = fs::rename(&bookmarks_lock_path, &bookmarks_path);
    assert!(res.is_ok(), "{}", res.unwrap_err());
}
