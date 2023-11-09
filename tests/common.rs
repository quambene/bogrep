use bogrep::{json, utils, TargetBookmark, TargetBookmarks};
use std::{collections::HashMap, fs::File, io::Read, path::Path};
use tempfile::TempDir;
use wiremock::{
    matchers::{method, path},
    Mock, MockGuard, MockServer, ResponseTemplate,
};

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

#[allow(dead_code)]
pub fn test_bookmarks(temp_dir: &TempDir) -> Vec<TargetBookmark> {
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
    bookmarks.bookmarks
}

pub async fn start_mock_server() -> MockServer {
    let mock_server = MockServer::start().await;
    let bind_url = mock_server.uri();
    println!("Mock server running at {}", bind_url);
    mock_server
}

pub async fn mount_mocks(mock_server: &MockServer, num_mocks: u32) -> HashMap<String, String> {
    let mut mocks = HashMap::new();
    let bind_url = mock_server.uri();

    for i in 0..num_mocks {
        let endpoint = format!("endpoint_{}", i);
        let url = format!("{}/{}", bind_url, endpoint);
        let content = format!("Test content {}", i);
        let response = format!("<!DOCTYPE html><html><body>{}</body></html>", content);
        mocks.insert(url.clone(), content.clone());
        Mock::given(method("GET"))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(200).set_body_string(response))
            .mount(&mock_server)
            .await;
    }

    mocks
}

#[allow(dead_code)]
pub async fn mount_mock_scoped(
    mock_server: &MockServer,
    url_identifier: u32,
    content_identifier: u32,
) -> (HashMap<String, String>, MockGuard) {
    let mut mocks = HashMap::new();
    let bind_url = mock_server.uri();
    let endpoint = format!("endpoint_{}", url_identifier);
    let url = format!("{}/{}", bind_url, endpoint);
    let content = format!("Test content {}", content_identifier);
    let response = format!("<!DOCTYPE html><html><body>{}</body></html>", content);

    mocks.insert(url.clone(), content.clone());

    let mock_guard = Mock::given(method("GET"))
        .and(path(endpoint))
        .respond_with(ResponseTemplate::new(200).set_body_string(response))
        .mount_as_scoped(&mock_server)
        .await;

    (mocks, mock_guard)
}
