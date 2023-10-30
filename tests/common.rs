use bogrep::{json, utils, TargetBookmarks};
use std::{fs::File, io::Read, path::Path};
use tempfile::TempDir;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

const TEST_URL_1: &str = "/how-mathematical-curves-power-cryptography-20220919";
const TEST_URL_2: &str = "/how-galois-groups-used-polynomial-symmetries-to-reshape-math-20210803";
const TEST_URL_3: &str = "/computing-expert-says-programmers-need-more-math-20220517";
const TEST_CONTENT_1: &str = "<html>Test content 1</html>";
const TEST_CONTENT_2: &str = "<html>Test content 2</html>";
const TEST_CONTENT_3: &str = "<html>Test content 3</html>";

#[allow(dead_code)]
pub fn compare_files(actual_path: &Path, expected_path: &Path) -> (String, String) {
    let mut actual_file = File::open(&actual_path).unwrap();
    let mut actual = String::new();
    actual_file.read_to_string(&mut actual).unwrap();

    let mut expected_file = File::open(&expected_path).unwrap();
    let mut expected = String::new();
    expected_file.read_to_string(&mut expected).unwrap();

    (actual, expected)
}

pub fn test_bookmarks(temp_dir: &TempDir) -> TargetBookmarks {
    let bookmarks_path = temp_dir.path().join("bookmarks.json");
    assert!(
        bookmarks_path.exists(),
        "Missing path: {}",
        bookmarks_path.display()
    );
    let bookmarks = utils::read_file(&bookmarks_path).unwrap();
    let res = json::deserialize::<TargetBookmarks>(&bookmarks);
    assert!(
        res.is_ok(),
        "Can't deserialize bookmarks: {}\n{}",
        res.unwrap_err(),
        String::from_utf8(bookmarks).unwrap(),
    );

    let bookmarks = res.unwrap();
    println!("Bookmarks:  {bookmarks:#?}");
    bookmarks
}

pub async fn start_mock_server() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(TEST_URL_1))
        .respond_with(ResponseTemplate::new(200).set_body_string(TEST_CONTENT_1))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path(TEST_URL_2))
        .respond_with(ResponseTemplate::new(200).set_body_string(TEST_CONTENT_2))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path(TEST_URL_3))
        .respond_with(ResponseTemplate::new(200).set_body_string(TEST_CONTENT_3))
        .mount(&mock_server)
        .await;
}
