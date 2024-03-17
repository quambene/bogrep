use super::{ReadBookmark, SelectSource, SourceOs};
use crate::{bookmarks::SourceBookmarkBuilder, utils, Source, SourceBookmarks, SourceType};
use anyhow::anyhow;
use log::{debug, trace};
use serde_json::{Map, Value};
use std::{
    fs::{self, DirEntry},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    time::SystemTime,
};

pub struct FirefoxSelector;

impl FirefoxSelector {
    pub fn new() -> Box<Self> {
        Box::new(FirefoxSelector)
    }

    /// Find the most recent bookmark file in the bookmark folder for Firefox.
    fn find_most_recent_file(bookmark_path: &Path) -> Result<PathBuf, anyhow::Error> {
        let entries = fs::read_dir(bookmark_path)?;

        let mut most_recent_entry: Option<DirEntry> = None;
        let mut most_recent_time: Option<SystemTime> = None;

        for entry in entries {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let modified_time = metadata.modified()?;

            if most_recent_time.is_none() || modified_time > most_recent_time.unwrap() {
                most_recent_time = Some(modified_time);
                most_recent_entry = Some(entry);
            }
        }

        if let Some(most_recent_entry) = most_recent_entry {
            let bookmark_path = most_recent_entry.path();

            if bookmark_path.is_file() {
                Ok(bookmark_path)
            } else {
                Err(anyhow!(
                    "Unexpected format for bookmark file: {}",
                    bookmark_path.display()
                ))
            }
        } else {
            Err(anyhow!(
                "Unexpected format for bookmark file: {}",
                bookmark_path.display()
            ))
        }
    }
}

impl SelectSource for FirefoxSelector {
    fn name(&self) -> SourceType {
        SourceType::Firefox
    }

    fn extension(&self) -> Option<&str> {
        Some("jsonlz4")
    }

    fn find_sources(
        &self,
        home_dir: &Path,
        source_os: &SourceOs,
    ) -> Result<Vec<PathBuf>, anyhow::Error> {
        debug!("Find sources for {}", self.name());

        let mut bookmark_dirs = vec![];
        let browser_dirs = match source_os {
            SourceOs::Linux => vec![
                // apt package
                home_dir.join(".mozilla/firefox"),
                // snap package
                home_dir.join("snap/firefox/common/.mozilla/firefox"),
            ],
            SourceOs::Windows => vec![],
            SourceOs::Macos => vec![home_dir.join("Library/Application Support/Firefox")],
        };

        for browser_dir in browser_dirs {
            let profiles_path = browser_dir.join("profiles.ini");

            if profiles_path.is_file() {
                let profiles_file = utils::open_file(&profiles_path)?;
                let buf_reader = BufReader::new(profiles_file);
                let lines = buf_reader.lines();

                let mut profiles = vec![];

                for line in lines {
                    let line = line?;

                    if let Some(path_index) = line.find("Path=") {
                        let profile = &line[(path_index + 5)..];
                        profiles.push(profile.to_owned());
                    }
                }

                for profile in profiles {
                    let bookmark_dir = browser_dir.join(profile).join("bookmarkbackups");

                    if bookmark_dir.is_dir() {
                        let mut entries = fs::read_dir(&bookmark_dir)?;

                        if entries.next().is_some() {
                            bookmark_dirs.push(bookmark_dir);
                        }
                    }
                }
            }
        }

        Ok(bookmark_dirs)
    }

    fn find_source_file(&self, source_dir: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        let path_str = source_dir
            .to_str()
            .ok_or(anyhow!("Invalid path: source path contains invalid UTF-8"))?;

        // On Linux, the path contains the lowercase identifier; on macOS,
        // uppercase identifier is required.
        if path_str.contains("firefox") || path_str.contains("Firefox") {
            // The Firefox bookmarks directory contains multiple bookmark files.
            let bookmark_path = Self::find_most_recent_file(source_dir)?;
            debug!(
                "Find source file {} for {}",
                bookmark_path.display(),
                self.name()
            );
            Ok(Some(bookmark_path))
        } else {
            Err(anyhow!(
                "Unexpected format for source directory: {}",
                source_dir.display()
            ))
        }
    }
}

/// A bookmark reader to read bookmarks in JSON format from Firefox.
#[derive(Debug)]
pub struct FirefoxReader;

impl FirefoxReader {
    pub fn new() -> Box<Self> {
        Box::new(Self)
    }

    fn select_bookmark(
        obj: &Map<String, Value>,
        source: &Source,
        bookmarks: &mut SourceBookmarks,
        bookmark_folder: &mut Option<String>,
    ) {
        trace!("json object: {obj:#?}");

        if let Some(Value::String(type_value)) = obj.get("type") {
            if type_value == "text/x-moz-place" {
                if let Some(Value::String(uri_value)) = obj.get("uri") {
                    if uri_value.contains("http") {
                        let source_bookmark = SourceBookmarkBuilder::new(uri_value)
                            .add_source(source.source_type.to_owned())
                            .add_folder_opt(
                                source.source_type.to_owned(),
                                bookmark_folder.to_owned(),
                            )
                            .build();
                        bookmarks.insert(source_bookmark);
                    }
                }
            }
        }
    }

    pub fn traverse_json(value: &Value, source: &Source, bookmarks: &mut SourceBookmarks) {
        let mut parent_folder = None;

        match value {
            Value::Object(obj) => {
                if source.folders.is_empty() {
                    Self::select_bookmark(obj, source, bookmarks, &mut parent_folder);

                    for (_, val) in obj {
                        Self::traverse_json(val, source, bookmarks);
                    }
                } else {
                    if let Some(Value::String(type_value)) = obj.get("type") {
                        if type_value == "text/x-moz-place-container" {
                            if let Some(Value::String(title_value)) = obj.get("title") {
                                parent_folder = Some(title_value.to_owned());

                                if source.folders.contains(title_value) {
                                    for (_, val) in obj {
                                        Self::traverse_children(
                                            val,
                                            source,
                                            bookmarks,
                                            &mut parent_folder,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    for (_, val) in obj {
                        Self::traverse_json(val, source, bookmarks);
                    }
                }
            }
            Value::Array(arr) => {
                for val in arr {
                    Self::traverse_json(val, source, bookmarks);
                }
            }
            Value::String(_) => (),
            Value::Number(_) => (),
            Value::Bool(_) => (),
            Value::Null => (),
        }
    }

    fn traverse_children(
        value: &Value,
        source: &Source,
        bookmarks: &mut SourceBookmarks,
        parent_folder: &mut Option<String>,
    ) {
        let mut folder = None;

        match value {
            Value::Object(obj) => {
                Self::select_bookmark(obj, source, bookmarks, parent_folder);

                if let Some(Value::String(type_value)) = obj.get("type") {
                    if type_value == "text/x-moz-place-container" {
                        if let Some(Value::String(title_value)) = obj.get("title") {
                            folder = Some(title_value.to_owned());
                        }
                    }
                }

                for (_, val) in obj {
                    Self::traverse_children(val, source, bookmarks, &mut folder);
                }
            }
            Value::Array(arr) => {
                for val in arr {
                    Self::traverse_children(val, source, bookmarks, parent_folder);
                }
            }
            Value::String(_) => (),
            Value::Number(_) => (),
            Value::Bool(_) => (),
            Value::Null => (),
        }
    }
}

impl<'a> ReadBookmark<'a> for FirefoxReader {
    type ParsedValue = serde_json::Value;

    fn name(&self) -> SourceType {
        SourceType::Firefox
    }

    fn extension(&self) -> Option<&str> {
        Some("json")
    }

    fn select_source(
        &self,
        _source_path: &Path,
        parsed_bookmarks: &Value,
    ) -> Result<Option<SourceType>, anyhow::Error> {
        match parsed_bookmarks {
            Value::Object(obj) => {
                if let Some(Value::String(type_value)) = obj.get("type") {
                    if type_value == "text/x-moz-place-container" {
                        Ok(Some(SourceType::Firefox))
                    } else {
                        Ok(None)
                    }
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
        Self::traverse_json(&parsed_bookmarks, source, source_bookmarks);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bookmark_reader::{
            source_reader::{CompressedJsonReader, JsonReader},
            ParsedBookmarks, ReadSource, SourceReader,
        },
        test_utils, utils,
    };
    use assert_matches::assert_matches;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_selector_name() {
        let selector = FirefoxSelector;
        assert_eq!(selector.name(), SourceType::Firefox);
    }

    #[test]
    fn test_find_sources_empty() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        let selector = FirefoxSelector;

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

    #[cfg(not(any(target_os = "windows")))]
    #[test]
    fn test_find_sources_linux() {
        let source_os = SourceOs::Linux;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        test_utils::tests::create_test_files(temp_path, &source_os);

        let selector = FirefoxSelector;
        let res = selector.find_sources(temp_path, &source_os);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let sources = res.unwrap();
        assert_eq!(sources.len(), 4);
        assert!(sources.contains(
            &temp_path
                .join("snap/firefox/common/.mozilla/firefox/profile1.default/bookmarkbackups")
        ));
        assert!(sources.contains(
            &temp_path
                .join("snap/firefox/common/.mozilla/firefox/profile2.username/bookmarkbackups")
        ));
        assert!(
            sources.contains(&temp_path.join(".mozilla/firefox/profile1.default/bookmarkbackups"))
        );
        assert!(
            sources.contains(&temp_path.join(".mozilla/firefox/profile2.username/bookmarkbackups"))
        );
    }

    #[cfg(not(any(target_os = "windows")))]
    #[test]
    fn test_find_sources_macos() {
        let source_os = SourceOs::Macos;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        test_utils::tests::create_test_files(temp_path, &source_os);

        let selector = FirefoxSelector;
        let res = selector.find_sources(temp_path, &source_os);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let sources = res.unwrap();
        assert_eq!(sources.len(), 2);
        assert!(sources.contains(&temp_path.join(
            "Library/Application Support/Firefox/Profiles/profile1.default/bookmarkbackups"
        )));
        assert!(sources.contains(&temp_path.join(
            "Library/Application Support/Firefox/Profiles/profile2.username/bookmarkbackups"
        )));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_find_sources_windows() {
        let source_os = SourceOs::Windows;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        test_utils::tests::create_test_files(temp_path, &source_os);

        let selector = FirefoxSelector;
        let res = selector.find_sources(temp_path, &source_os);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let sources = res.unwrap();
        assert!(sources.is_empty());
    }

    #[cfg(not(any(target_os = "windows")))]
    #[test]
    fn test_find_source_file_linux() {
        use std::{thread, time::Duration};

        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        let bookmark_dir =
            temp_path.join("snap/firefox/common/.mozilla/firefox/profile1.default/bookmarkbackups");
        fs::create_dir_all(&bookmark_dir).unwrap();
        utils::create_file(&bookmark_dir.join("bookmarks1.jsonlz4")).unwrap();
        thread::sleep(Duration::from_millis(10));
        utils::create_file(&bookmark_dir.join("bookmarks2.jsonlz4")).unwrap();

        let selector = FirefoxSelector;
        let res = selector.find_source_file(&bookmark_dir);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_file = res.unwrap();
        assert_eq!(
            bookmark_file.unwrap(),
            // We are expecting the file which was created more recently.
            temp_path.join("snap/firefox/common/.mozilla/firefox/profile1.default/bookmarkbackups/bookmarks2.jsonlz4")
        );
    }

    #[cfg(not(any(target_os = "windows")))]
    #[test]
    fn test_find_source_file_macos() {
        use std::{thread, time::Duration};

        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        let bookmark_dir = temp_path
            .join("Library/Application Support/Firefox/Profiles/profile1.default/bookmarkbackups");
        fs::create_dir_all(&bookmark_dir).unwrap();
        utils::create_file(&bookmark_dir.join("bookmarks1.jsonlz4")).unwrap();
        thread::sleep(Duration::from_millis(10));
        utils::create_file(&bookmark_dir.join("bookmarks2.jsonlz4")).unwrap();

        let selector = FirefoxSelector;
        let res = selector.find_source_file(&bookmark_dir);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_file = res.unwrap();
        assert_eq!(
            bookmark_file.unwrap(),
            // We are expecting the file which was created more recently.
            temp_path.join("Library/Application Support/Firefox/Profiles/profile1.default/bookmarkbackups/bookmarks2.jsonlz4")
        );
    }

    #[test]
    fn test_read_and_parse() {
        let source_path = Path::new("test_data/bookmarks_firefox.json");
        let mut reader = utils::open_file(source_path).unwrap();
        let source_reader = JsonReader;

        let res = source_reader.read_and_parse(&mut reader);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let parsed_bookmarks = res.unwrap();
        assert_matches!(parsed_bookmarks, ParsedBookmarks::Json(_));
    }

    #[test]
    fn test_read_and_parse_compressed() {
        let source_path = Path::new("test_data/bookmarks_firefox.jsonlz4");
        test_utils::create_compressed_json_file(source_path).unwrap();
        let mut reader = utils::open_file(source_path).unwrap();
        let source_reader = CompressedJsonReader;

        let res = source_reader.read_and_parse(&mut reader);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let parsed_bookmarks = res.unwrap();
        assert_matches!(parsed_bookmarks, ParsedBookmarks::Json(_));
    }

    #[test]
    fn test_import_all() {
        let decompressed_bookmark_path = Path::new("test_data/bookmarks_firefox.json");

        let mut source_bookmarks = SourceBookmarks::default();
        let source = Source::new(SourceType::Unknown, &PathBuf::from("dummy_path"), vec![]);
        let bookmark_file = utils::open_file(decompressed_bookmark_path).unwrap();
        let source_reader = Box::new(JsonReader);
        let mut source_reader = SourceReader::new(source, Box::new(bookmark_file), source_reader);

        let res = source_reader.import(&mut source_bookmarks);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let url1 = "https://www.mozilla.org/en-US/firefox/central/";
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
                        .add_source(SourceType::Firefox)
                        .build()
                ),
                (
                    url2.to_owned(),
                    SourceBookmarkBuilder::new(url2)
                        .add_source(SourceType::Firefox)
                        .build()
                ),
                (
                    url3.to_owned(),
                    SourceBookmarkBuilder::new(url3)
                        .add_source(SourceType::Firefox)
                        .build()
                ),
                (
                    url4.to_owned(),
                    SourceBookmarkBuilder::new(url4)
                        .add_source(SourceType::Firefox)
                        .build()
                )
            ])
        );
    }

    #[test]
    fn test_import_folder() {
        let decompressed_bookmark_path = Path::new("test_data/bookmarks_firefox.json");

        let mut source_bookmarks = SourceBookmarks::default();
        let source = Source::new(
            SourceType::Unknown,
            &PathBuf::from("dummy_path"),
            vec![String::from("dev")],
        );
        let bookmark_file = utils::open_file(decompressed_bookmark_path).unwrap();
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
                        .add_source(SourceType::Firefox)
                        .add_folder(SourceType::Firefox, "dev")
                        .build()
                ),
                (
                    url2.to_owned(),
                    SourceBookmarkBuilder::new(url2)
                        .add_source(SourceType::Firefox)
                        .add_folder(SourceType::Firefox, "rust")
                        .build()
                ),
            ])
        );
    }
}
