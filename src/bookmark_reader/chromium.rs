use super::{ReadBookmark, SelectSource, SourceOs};
use crate::{
    bookmarks::{Source, SourceBookmarkBuilder},
    SourceBookmarks, SourceType,
};
use anyhow::anyhow;
use log::{debug, trace};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub type JsonBookmarkReader<'a> = Box<dyn ReadBookmark<'a, ParsedValue = serde_json::Value>>;

pub struct ChromiumSelector;

impl ChromiumSelector {
    pub fn new() -> Box<Self> {
        Box::new(ChromiumSelector)
    }

    pub fn find_profile_dirs(browser_dirs: &[PathBuf]) -> Vec<PathBuf> {
        let mut bookmark_dirs = vec![];

        for browser_dir in browser_dirs {
            let bookmark_dir = browser_dir.join("Default");

            if bookmark_dir.is_dir() {
                bookmark_dirs.push(bookmark_dir);
            }

            // Sane people will have less than 100 profiles.
            for i in 1..=100 {
                let bookmark_dir = browser_dir.join(format!("Profile {i}"));

                if bookmark_dir.is_dir() {
                    bookmark_dirs.push(bookmark_dir);
                }
            }
        }

        bookmark_dirs
    }
}

impl SelectSource for ChromiumSelector {
    fn name(&self) -> SourceType {
        SourceType::Chromium
    }

    fn extension(&self) -> Option<&str> {
        Some("json")
    }

    fn find_sources(
        &self,
        home_dir: &Path,
        source_os: &SourceOs,
    ) -> Result<Vec<PathBuf>, anyhow::Error> {
        debug!("Find sources for {}", self.name());

        let browser_dirs = match source_os {
            SourceOs::Linux => vec![
                // snap package
                home_dir.join("snap/chromium/common/chromium"),
            ],
            SourceOs::Windows => vec![],
            SourceOs::Macos => vec![],
        };
        let bookmark_dirs = ChromiumSelector::find_profile_dirs(&browser_dirs);
        let bookmark_files = bookmark_dirs
            .into_iter()
            .filter_map(|bookmark_dir| {
                let bookmark_file = bookmark_dir.join("Bookmarks");

                if bookmark_file.is_file() {
                    Some(bookmark_file)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(bookmark_files)
    }
}

/// A bookmark reader to read bookmarks in JSON format from Chromium or Chrome.
#[derive(Debug)]
pub struct ChromiumReader;

impl ChromiumReader {
    pub fn new() -> Box<Self> {
        Box::new(Self)
    }

    fn select_bookmark(
        obj: &Map<String, Value>,
        source_bookmarks: &mut SourceBookmarks,
        source: &Source,
    ) {
        trace!("json object: {obj:#?}");

        if let Some(Value::String(type_value)) = obj.get("type") {
            if type_value == "url" {
                if let Some(Value::String(url_value)) = obj.get("url") {
                    if url_value.contains("http") {
                        let source_bookmark = SourceBookmarkBuilder::new(url_value)
                            .add_source(&source.source_type)
                            .build();
                        source_bookmarks.insert(source_bookmark);
                    }
                }
            }
        }
    }

    fn traverse_json(source: &Source, value: &Value, source_bookmarks: &mut SourceBookmarks) {
        match value {
            Value::Object(obj) => {
                if source.folders.is_empty() {
                    Self::select_bookmark(obj, source_bookmarks, source);

                    for (_, val) in obj {
                        Self::traverse_json(source, val, source_bookmarks);
                    }
                } else {
                    if let Some(Value::String(type_value)) = obj.get("type") {
                        if type_value == "folder" {
                            if let Some(Value::String(name_value)) = obj.get("name") {
                                if source.folders.contains(name_value) {
                                    for (_, val) in obj {
                                        Self::traverse_children(source, val, source_bookmarks);
                                    }
                                }
                            }
                        }
                    }

                    for (_, val) in obj {
                        Self::traverse_json(source, val, source_bookmarks);
                    }
                }
            }
            Value::Array(arr) => {
                for val in arr {
                    Self::traverse_json(source, val, source_bookmarks);
                }
            }
            Value::String(_) => (),
            Value::Number(_) => (),
            Value::Bool(_) => (),
            Value::Null => (),
        }
    }

    fn traverse_children(source: &Source, value: &Value, source_bookmarks: &mut SourceBookmarks) {
        match value {
            Value::Object(obj) => {
                Self::select_bookmark(obj, source_bookmarks, source);

                for (_, val) in obj {
                    Self::traverse_children(source, val, source_bookmarks);
                }
            }
            Value::Array(arr) => {
                for val in arr {
                    Self::traverse_children(source, val, source_bookmarks);
                }
            }
            Value::String(_) => (),
            Value::Number(_) => (),
            Value::Bool(_) => (),
            Value::Null => (),
        }
    }
}

impl<'a> ReadBookmark<'a> for ChromiumReader {
    type ParsedValue = serde_json::Value;

    fn name(&self) -> SourceType {
        SourceType::Chromium
    }

    fn extension(&self) -> Option<&str> {
        Some("json")
    }

    fn select_source(
        &self,
        source_path: &Path,
        parsed_bookmarks: &Value,
    ) -> Result<Option<SourceType>, anyhow::Error> {
        match parsed_bookmarks {
            Value::Object(obj) => {
                if obj.get("checksum").is_some()
                    && obj.get("roots").is_some()
                    && obj.get("version").is_some()
                {
                    let path_str = source_path
                        .to_str()
                        .ok_or(anyhow!("Invalid path: source path contains invalid UTF-8"))?;
                    let source_type =
                        if path_str.contains("chromium") || path_str.contains("Chromium") {
                            SourceType::Chromium
                        } else if path_str.contains("chrome") || path_str.contains("Chrome") {
                            SourceType::Chrome
                        } else if path_str.contains("edge") || path_str.contains("Edge") {
                            SourceType::Edge
                        } else {
                            SourceType::ChromiumDerivative
                        };
                    Ok(Some(source_type))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn import(
        &self,
        source: &Source,
        parsed_bookmarks: Value,
        source_bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        debug!("Import bookmarks from {:#?}", self.name());
        Self::traverse_json(source, &parsed_bookmarks, source_bookmarks);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bookmark_reader::{
            source_reader::{JsonReader, JsonReaderNoExtension},
            ParsedBookmarks, ReadSource, SourceReader,
        },
        test_utils::tests,
        utils,
    };
    use assert_matches::assert_matches;
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
    };
    use tempfile::tempdir;

    #[test]
    fn test_selector_name() {
        let selector = ChromiumSelector;
        assert_eq!(selector.name(), SourceType::Chromium);
    }

    #[test]
    fn test_find_sources_empty() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        let selector = ChromiumSelector;

        let res = selector.find_sources(temp_path, &SourceOs::Linux);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let sources = res.unwrap();
        assert!(sources.is_empty());

        let res = selector.find_sources(temp_path, &SourceOs::Macos);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let sources = res.unwrap();
        assert!(sources.is_empty());

        let res = selector.find_sources(temp_path, &SourceOs::Windows);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let sources = res.unwrap();
        assert!(sources.is_empty());
    }

    #[test]
    fn test_find_sources_linux() {
        let source_os = SourceOs::Linux;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        tests::create_test_files(temp_path, &source_os);

        let selector = ChromiumSelector;
        let res = selector.find_sources(temp_path, &source_os);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_dirs = res.unwrap();
        assert_eq!(bookmark_dirs.len(), 2);
        assert!(bookmark_dirs
            .contains(&temp_path.join("snap/chromium/common/chromium/Default/Bookmarks")));
        assert!(bookmark_dirs
            .contains(&temp_path.join("snap/chromium/common/chromium/Profile 1/Bookmarks")));
    }

    #[test]
    fn test_find_sources_macos() {
        let source_os = SourceOs::Macos;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        tests::create_test_files(temp_path, &source_os);

        let selector = ChromiumSelector;
        let res = selector.find_sources(temp_path, &source_os);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_dirs = res.unwrap();
        assert!(bookmark_dirs.is_empty());
    }

    #[test]
    fn test_find_sources_windows() {
        let source_os = SourceOs::Windows;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        tests::create_test_files(temp_path, &source_os);

        let selector = ChromiumSelector;
        let res = selector.find_sources(temp_path, &source_os);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_dirs = res.unwrap();
        assert!(bookmark_dirs.is_empty());
    }

    #[test]
    fn test_read_and_parse() {
        let source_path = Path::new("test_data/bookmarks_chromium.json");
        let mut reader = utils::open_file(source_path).unwrap();
        let source_reader = JsonReader;

        let res = source_reader.read_and_parse(&mut reader);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let parsed_bookmarks = res.unwrap();
        assert_matches!(parsed_bookmarks, ParsedBookmarks::Json(_));
    }

    #[test]
    fn test_read_and_parse_no_extension() {
        let source_path = Path::new("test_data/bookmarks_chromium_no_extension");
        let mut reader = utils::open_file(source_path).unwrap();
        let source_reader = JsonReaderNoExtension;

        let res = source_reader.read_and_parse(&mut reader);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let parsed_bookmarks = res.unwrap();
        assert_matches!(parsed_bookmarks, ParsedBookmarks::Json(_));
    }

    #[test]
    fn test_import_all() {
        let source_path = Path::new("test_data/bookmarks_chromium.json");
        assert!(source_path.exists());

        let mut source_bookmarks = SourceBookmarks::default();
        let source = Source::new(SourceType::Unknown, &PathBuf::from("dummy_path"), vec![]);
        let bookmark_file = utils::open_file(source_path).unwrap();
        let source_reader = Box::new(JsonReader);
        let mut source_reader = SourceReader::new(source, Box::new(bookmark_file), source_reader);

        let res = source_reader.import(&mut source_bookmarks);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let url1 = "https://www.deepl.com/translator";
        let url2 =
            "https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/";
        let url3 = "https://en.wikipedia.org/wiki/Design_Patterns";
        let url4 = "https://doc.rust-lang.org/book/title-page.html";

        assert_eq!(
            source_bookmarks.inner(),
            HashMap::from_iter([
                (
                    url1.to_owned(),
                    SourceBookmarkBuilder::new(url1)
                        .add_source(&SourceType::ChromiumDerivative)
                        .build()
                ),
                (
                    url2.to_owned(),
                    SourceBookmarkBuilder::new(url2)
                        .add_source(&SourceType::ChromiumDerivative)
                        .build()
                ),
                (
                    url3.to_owned(),
                    SourceBookmarkBuilder::new(url3)
                        .add_source(&SourceType::ChromiumDerivative)
                        .build()
                ),
                (
                    url4.to_owned(),
                    SourceBookmarkBuilder::new(url4)
                        .add_source(&SourceType::ChromiumDerivative)
                        .build()
                )
            ])
        );
    }

    #[test]
    fn test_import_folder() {
        let source_path = Path::new("test_data/bookmarks_chromium.json");
        assert!(source_path.exists());

        let mut source_bookmarks = SourceBookmarks::default();
        let source = Source::new(
            SourceType::Unknown,
            &PathBuf::from("dummy_path"),
            vec!["dev".to_owned()],
        );
        let bookmark_file = utils::open_file(source_path).unwrap();
        let source_reader = Box::new(JsonReader);
        let mut source_reader = SourceReader::new(source, Box::new(bookmark_file), source_reader);

        let res = source_reader.import(&mut source_bookmarks);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let url1 = "https://en.wikipedia.org/wiki/Design_Patterns";
        let url2 = "https://doc.rust-lang.org/book/title-page.html";

        assert_eq!(
            source_bookmarks.inner(),
            HashMap::from_iter([
                (
                    url1.to_owned(),
                    SourceBookmarkBuilder::new(url1)
                        .add_source(&SourceType::ChromiumDerivative)
                        .build()
                ),
                (
                    url2.to_owned(),
                    SourceBookmarkBuilder::new(url2)
                        .add_source(&SourceType::ChromiumDerivative)
                        .build()
                ),
            ])
        );
    }
}
