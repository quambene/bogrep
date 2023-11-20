mod common;

use bogrep::{json, utils, BookmarksJson, TargetBookmark};
use chrono::Utc;
use std::{collections::HashSet, io::Write};
use tempfile::tempdir;

#[test]
fn test_rename() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    let bookmarks_path = temp_path.join("bookmarks.json");
    let bookmarks_lock_path = temp_path.join("bookmarks-lock.json");

    {
        let bookmarks_json = BookmarksJson::default();
        let buf = json::serialize(&bookmarks_json).unwrap();
        let mut bookmarks_file = utils::create_file(&bookmarks_path).unwrap();
        bookmarks_file.write_all(&buf).unwrap();

        let mut bookmarks_json = BookmarksJson::default();
        bookmarks_json.bookmarks.push(TargetBookmark::new(
            "https://test_url.com".to_owned(),
            Utc::now(),
            None,
            HashSet::new(),
        ));
        let buf = json::serialize(&bookmarks_json).unwrap();
        let mut bookmarks_lock_file = utils::open_and_truncate_file(&bookmarks_lock_path).unwrap();
        bookmarks_lock_file.write_all(&buf).unwrap();
    }

    assert!(bookmarks_path.exists());
    assert!(bookmarks_lock_path.exists());

    let bookmarks_file = utils::open_file_in_read_mode(&bookmarks_path).unwrap();
    let bookmarks_lock_file = utils::open_and_truncate_file(&bookmarks_lock_path).unwrap();

    let res = utils::close_and_rename(
        (bookmarks_lock_file, &bookmarks_lock_path),
        (bookmarks_file, &bookmarks_path),
    );
    assert!(res.is_ok(), "{}", res.unwrap_err());
}
