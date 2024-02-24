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

    fn source_os(&self) -> SourceOs {
        SourceOs::Linux
    }

    fn extension(&self) -> Option<&str> {
        Some("jsonlz4")
    }

    fn find_dir(&self, home_dir: &Path) -> Result<Vec<PathBuf>, anyhow::Error> {
        let mut bookmark_dirs = vec![];

        let browser_dirs = [
            // apt package
            home_dir.join(".mozilla/firefox"),
            // snap package
            home_dir.join("snap/firefox/common/.mozilla/firefox"),
        ];

        for browser_dir in browser_dirs {
            let profiles_file = utils::open_file(&browser_dir.join("profiles.ini"))?;
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
                let bookmark_dir = browser_dir.join(profile);
                bookmark_dirs.push(bookmark_dir);
            }
        }

        Ok(bookmark_dirs)
    }

    fn find_file(&self, source_dir: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        let path_str = source_dir
            .to_str()
            .ok_or(anyhow!("Invalid path: source path contains invalid UTF-8"))?;

        // On Linux, the path contains the lowercase identifier; on macOS,
        // uppercase identifier is required.
        if path_str.contains("firefox") || path_str.contains("Firefox") {
            // The Firefox bookmarks directory contains multiple bookmark files.
            let bookmark_path = Self::find_most_recent_file(source_dir)?;
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

    fn select_bookmark(obj: &Map<String, Value>, source: &Source, bookmarks: &mut SourceBookmarks) {
        trace!("json object: {obj:#?}");

        if let Some(Value::String(type_value)) = obj.get("type") {
            if type_value == "text/x-moz-place" {
                if let Some(Value::String(uri_value)) = obj.get("uri") {
                    if uri_value.contains("http") {
                        let source_bookmark = SourceBookmarkBuilder::new(uri_value)
                            .add_source(&source.source_type)
                            .build();
                        bookmarks.insert(source_bookmark);
                    }
                }
            }
        }
    }

    pub fn traverse_json(value: &Value, source: &Source, bookmarks: &mut SourceBookmarks) {
        match value {
            Value::Object(obj) => {
                if source.folders.is_empty() {
                    Self::select_bookmark(obj, source, bookmarks);

                    for (_, val) in obj {
                        Self::traverse_json(val, source, bookmarks);
                    }
                } else {
                    if let Some(Value::String(type_value)) = obj.get("type") {
                        if type_value == "text/x-moz-place-container" {
                            if let Some(Value::String(title_value)) = obj.get("title") {
                                if source.folders.contains(title_value) {
                                    for (_, val) in obj {
                                        Self::traverse_children(val, source, bookmarks);
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

    fn traverse_children(value: &Value, source: &Source, bookmarks: &mut SourceBookmarks) {
        match value {
            Value::Object(obj) => {
                Self::select_bookmark(obj, source, bookmarks);

                for (_, val) in obj {
                    Self::traverse_children(val, source, bookmarks);
                }
            }
            Value::Array(arr) => {
                for val in arr {
                    Self::traverse_children(val, source, bookmarks);
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
    use std::{collections::HashMap, fs::File, io::Write};
    use tempfile::tempdir;

    #[test]
    fn test_find_dir() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        let browser_dir = temp_path.join("snap/firefox/common/.mozilla/firefox");
        fs::create_dir_all(browser_dir.join("profile.default")).unwrap();
        fs::create_dir_all(browser_dir.join("profile.username")).unwrap();
        let mut file = File::create(browser_dir.join("profiles.ini")).unwrap();
        let content = r#"
            [Profile1]
            Name=bene
            IsRelative=1
            Path=profile1.username
            Default=1
            
            [Profile0]
            Name=default
            IsRelative=1
            Path=profile1.default
        "#;
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();

        let browser_dir = temp_path.join(".mozilla/firefox");
        fs::create_dir_all(browser_dir.join("profile.default")).unwrap();
        fs::create_dir_all(browser_dir.join("profile.username")).unwrap();
        let mut file = File::create(browser_dir.join("profiles.ini")).unwrap();
        let content = r#"
            [Profile1]
            Name=bene
            IsRelative=1
            Path=profile2.username
            Default=1
            
            [Profile0]
            Name=default
            IsRelative=1
            Path=profile2.default
        "#;
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();

        let selector = FirefoxSelector;
        assert_eq!(selector.name(), SourceType::Firefox);

        let res = selector.find_dir(temp_path);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_dirs = res.unwrap();
        assert_eq!(bookmark_dirs.len(), 4);
        assert!(bookmark_dirs
            .contains(&temp_path.join("snap/firefox/common/.mozilla/firefox/profile1.username")));
        assert!(bookmark_dirs
            .contains(&temp_path.join("snap/firefox/common/.mozilla/firefox/profile1.default")));
        assert!(bookmark_dirs.contains(&temp_path.join(".mozilla/firefox/profile2.username")));
        assert!(bookmark_dirs.contains(&temp_path.join(".mozilla/firefox/profile2.username")));
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
                        .add_source(&SourceType::Firefox)
                        .build()
                ),
                (
                    url2.to_owned(),
                    SourceBookmarkBuilder::new(url2)
                        .add_source(&SourceType::Firefox)
                        .build()
                ),
                (
                    url3.to_owned(),
                    SourceBookmarkBuilder::new(url3)
                        .add_source(&SourceType::Firefox)
                        .build()
                ),
                (
                    url4.to_owned(),
                    SourceBookmarkBuilder::new(url4)
                        .add_source(&SourceType::Firefox)
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
                        .add_source(&SourceType::Firefox)
                        .build()
                ),
                (
                    url2.to_owned(),
                    SourceBookmarkBuilder::new(url2)
                        .add_source(&SourceType::Firefox)
                        .build()
                ),
            ])
        );
    }
}
