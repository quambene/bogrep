use super::{ReadBookmark, ReaderName};
use crate::{bookmarks::SourceBookmarkBuilder, utils, Source, SourceBookmarks, SourceType};
use anyhow::anyhow;
use log::{debug, trace};
use lz4::block;
use serde_json::{Map, Value};
use std::{
    fs::{self, DirEntry},
    io::Read,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub struct Firefox;

impl Firefox {
    fn select_bookmark(
        obj: &Map<String, Value>,
        source_bookmarks: &mut SourceBookmarks,
        source: &Source,
    ) {
        trace!("json object: {obj:#?}");

        if let Some(Value::String(type_value)) = obj.get("type") {
            if type_value == "text/x-moz-place" {
                if let Some(Value::String(uri_value)) = obj.get("uri") {
                    if uri_value.contains("http") {
                        let source_bookmark = SourceBookmarkBuilder::new(uri_value)
                            .add_source(&source.name)
                            .build();
                        source_bookmarks.insert(source_bookmark);
                    }
                }
            }
        }
    }

    pub fn traverse_json(value: &Value, source_bookmarks: &mut SourceBookmarks, source: &Source) {
        match value {
            Value::Object(obj) => {
                if source.folders.is_empty() {
                    Self::select_bookmark(obj, source_bookmarks, source);

                    for (_, val) in obj {
                        Self::traverse_json(val, source_bookmarks, source);
                    }
                } else {
                    if let Some(Value::String(type_value)) = obj.get("type") {
                        if type_value == "text/x-moz-place-container" {
                            if let Some(Value::String(title_value)) = obj.get("title") {
                                if source.folders.contains(title_value) {
                                    for (_, val) in obj {
                                        Self::traverse_children(val, source_bookmarks, source);
                                    }
                                }
                            }
                        }
                    }

                    for (_, val) in obj {
                        Self::traverse_json(val, source_bookmarks, source);
                    }
                }
            }
            Value::Array(arr) => {
                for val in arr {
                    Self::traverse_json(val, source_bookmarks, source);
                }
            }
            Value::String(_) => (),
            Value::Number(_) => (),
            Value::Bool(_) => (),
            Value::Null => (),
        }
    }

    fn traverse_children(value: &Value, source_bookmarks: &mut SourceBookmarks, source: &Source) {
        match value {
            Value::Object(obj) => {
                Self::select_bookmark(obj, source_bookmarks, source);

                for (_, val) in obj {
                    Self::traverse_children(val, source_bookmarks, source);
                }
            }
            Value::Array(arr) => {
                for val in arr {
                    Self::traverse_children(val, source_bookmarks, source);
                }
            }
            Value::String(_) => (),
            Value::Number(_) => (),
            Value::Bool(_) => (),
            Value::Null => (),
        }
    }

    /// Find the most recent bookmark file in the bookmark folder for Firefox.
    pub fn find_most_recent_file(bookmark_path: &Path) -> Result<PathBuf, anyhow::Error> {
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

/// A bookmark reader to read bookmarks in JSON format from Firefox.
#[derive(Debug)]
pub struct FirefoxBookmarkReader;

impl ReadBookmark for FirefoxBookmarkReader {
    fn name(&self) -> ReaderName {
        ReaderName::Firefox
    }

    fn extension(&self) -> Option<&str> {
        Some("json")
    }

    fn select_file(&self, source_dir: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        let path_str = source_dir
            .to_str()
            .ok_or(anyhow!("Invalid path: source path contains invalid UTF-8"))?;

        // On Linux, the path contains the lowercase identifier; on macOS,
        // uppercase identifier is required.
        if path_str.contains("firefox") || path_str.contains("Firefox") {
            // The Firefox bookmarks directory contains multiple bookmark files.
            let bookmark_path = Firefox::find_most_recent_file(source_dir)?;
            Ok(Some(bookmark_path))
        } else {
            Err(anyhow!(
                "Unexpected format for source directory: {}",
                source_dir.display()
            ))
        }
    }

    fn select_source(&self, source_file: &Path) -> Result<Option<SourceType>, anyhow::Error> {
        let raw_bookmarks = utils::read_file_to_string(source_file)?;
        let value: Value = serde_json::from_str(&raw_bookmarks)?;

        match value {
            Value::Object(obj) => {
                if let Some(Value::String(type_value)) = obj.get("type") {
                    if type_value == "text/x-moz-place-container" {
                        let path_str = source_file
                            .to_str()
                            .ok_or(anyhow!("Invalid path: source path contains invalid UTF-8"))?;

                        // On Linux, the path contains the lowercase identifier; on macOS,
                        // uppercase identifier is required.
                        let source_type =
                            if path_str.contains("firefox") || path_str.contains("Firefox") {
                                SourceType::Firefox
                            } else {
                                SourceType::Others
                            };
                        Ok(Some(source_type))
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

    fn read(&self, reader: &mut dyn Read) -> Result<String, anyhow::Error> {
        debug!("Read bookmarks from {}", self.name());

        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;

        Ok(String::from_utf8(buf)?)
    }

    fn parse(
        &self,
        raw_bookmarks: &str,
        source: &Source,
        bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        debug!("Parse bookmarks from {}", self.name());
        let value: Value = serde_json::from_str(raw_bookmarks)?;
        Firefox::traverse_json(&value, bookmarks, source);
        Ok(())
    }
}

/// A bookmark reader to read compressed bookmarks in JSON format from Firefox.
#[derive(Debug)]
pub struct FirefoxCompressedBookmarkReader;

impl ReadBookmark for FirefoxCompressedBookmarkReader {
    fn name(&self) -> ReaderName {
        ReaderName::FirefoxCompressed
    }

    fn extension(&self) -> Option<&str> {
        Some("jsonlz4")
    }

    fn select_file(&self, source_path: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        let path_str = source_path
            .to_str()
            .ok_or(anyhow!("Invalid path: source path contains invalid UTF-8"))?;

        // On Linux, the path contains the lowercase identifier; on macOS,
        // uppercase identifier is required.
        if path_str.contains("firefox") || path_str.contains("Firefox") {
            // The Firefox bookmarks directory contains multiple bookmark files.
            let bookmark_path = Firefox::find_most_recent_file(source_path)?;
            Ok(Some(bookmark_path))
        } else {
            Err(anyhow!(
                "Unexpected format for source file: {}",
                source_path.display()
            ))
        }
    }

    fn select_source(&self, source_file: &Path) -> Result<Option<SourceType>, anyhow::Error> {
        let mut bookmarks_file = utils::open_file(source_file)?;
        let mut compressed_data = Vec::new();
        bookmarks_file.read_to_end(&mut compressed_data)?;

        // Skip the first 8 bytes: "mozLz40\0" (non-standard header)
        let decompressed_data = block::decompress(&compressed_data[8..], None)?;
        let value: Value = serde_json::from_slice(&decompressed_data)?;

        match value {
            Value::Object(obj) => {
                if let Some(Value::String(type_value)) = obj.get("type") {
                    if type_value == "text/x-moz-place-container" {
                        let path_str = source_file
                            .to_str()
                            .ok_or(anyhow!("Invalid path: source path contains invalid UTF-8"))?;

                        // On Linux, the path contains the lowercase identifier; on macOS,
                        // uppercase identifier is required.
                        let source_type =
                            if path_str.contains("firefox") || path_str.contains("Firefox") {
                                SourceType::Firefox
                            } else {
                                SourceType::Others
                            };
                        Ok(Some(source_type))
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

    fn read(&self, reader: &mut dyn Read) -> Result<String, anyhow::Error> {
        debug!("Read bookmarks from {}", self.name());

        let mut compressed_data = Vec::new();
        reader.read_to_end(&mut compressed_data)?;

        // Skip the first 8 bytes: "mozLz40\0" (non-standard header)
        let decompressed_data = block::decompress(&compressed_data[8..], None)?;

        Ok(String::from_utf8(decompressed_data)?)
    }

    fn parse(
        &self,
        raw_bookmarks: &str,
        source: &Source,
        bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        debug!("Parse bookmarks from {}", self.name());
        let value: Value = serde_json::from_str(raw_bookmarks)?;
        Firefox::traverse_json(&value, bookmarks, source);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_utils, utils};
    use std::{collections::HashMap, io::Cursor};

    #[test]
    fn test_read() {
        let decompressed_bookmark_path = Path::new("test_data/bookmarks_firefox.json");
        let decompressed_bookmarks = utils::read_file(decompressed_bookmark_path).unwrap();
        let bookmark_reader = FirefoxBookmarkReader;
        let mut bookmark_file = Cursor::new(decompressed_bookmarks.clone());

        let bookmarks = bookmark_reader.read(&mut bookmark_file);
        assert!(bookmarks.is_ok(), "{}", bookmarks.unwrap_err());
        let bookmarks = bookmarks.unwrap();

        assert_eq!(
            bookmarks,
            String::from_utf8(decompressed_bookmarks).unwrap()
        );
    }

    #[test]
    fn test_read_compressed() {
        let decompressed_bookmark_path = Path::new("test_data/bookmarks_firefox.json");
        let decompressed_bookmarks = utils::read_file(decompressed_bookmark_path).unwrap();
        let compressed_bookmarks = test_utils::compress_bookmarks(&decompressed_bookmarks);
        let bookmark_reader = FirefoxCompressedBookmarkReader;
        let mut bookmark_file = Cursor::new(compressed_bookmarks);

        let bookmarks = bookmark_reader.read(&mut bookmark_file);
        assert!(bookmarks.is_ok(), "{}", bookmarks.unwrap_err());
        let bookmarks = bookmarks.unwrap();

        assert_eq!(
            bookmarks,
            String::from_utf8(decompressed_bookmarks).unwrap()
        );
    }

    #[test]
    fn test_parse_all() {
        let decompressed_bookmark_path = Path::new("test_data/bookmarks_firefox.json");
        let decompressed_bookmarks = utils::read_file(decompressed_bookmark_path).unwrap();
        let bookmark_reader = FirefoxBookmarkReader;
        let mut bookmark_file = Cursor::new(decompressed_bookmarks);

        let raw_bookmarks = bookmark_reader.read(&mut bookmark_file).unwrap();
        let mut source_bookmarks = SourceBookmarks::default();
        let source = Source::new(SourceType::Firefox, &PathBuf::from("dummy_path"), vec![]);

        let res = bookmark_reader.parse(&raw_bookmarks, &source, &mut source_bookmarks);
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
    fn test_parse_all_compressed() {
        let decompressed_bookmark_path = Path::new("test_data/bookmarks_firefox.json");
        let decompressed_bookmarks = utils::read_file(decompressed_bookmark_path).unwrap();
        let compressed_bookmarks = test_utils::compress_bookmarks(&decompressed_bookmarks);
        let bookmark_reader = FirefoxCompressedBookmarkReader;
        let mut bookmark_file = Cursor::new(compressed_bookmarks);

        let raw_bookmarks = bookmark_reader.read(&mut bookmark_file).unwrap();
        let mut source_bookmarks = SourceBookmarks::default();
        let source = Source::new(SourceType::Firefox, &PathBuf::from("dummy_path"), vec![]);

        let res = bookmark_reader.parse(&raw_bookmarks, &source, &mut source_bookmarks);
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
    fn test_parse_folder() {
        let decompressed_bookmark_path = Path::new("test_data/bookmarks_firefox.json");
        let decompressed_bookmarks = utils::read_file(decompressed_bookmark_path).unwrap();
        let bookmark_reader = FirefoxBookmarkReader;
        let mut bookmark_file = Cursor::new(decompressed_bookmarks);

        let raw_bookmarks = bookmark_reader.read(&mut bookmark_file).unwrap();
        let mut source_bookmarks = SourceBookmarks::default();
        let source = Source::new(
            SourceType::Firefox,
            &PathBuf::from("dummy_path"),
            vec![String::from("dev")],
        );

        let res = bookmark_reader.parse(&raw_bookmarks, &source, &mut source_bookmarks);
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

    #[test]
    fn test_parse_folder_compressed() {
        let decompressed_bookmark_path = Path::new("test_data/bookmarks_firefox.json");
        let decompressed_bookmarks = utils::read_file(decompressed_bookmark_path).unwrap();
        let compressed_bookmarks = test_utils::compress_bookmarks(&decompressed_bookmarks);
        let bookmark_reader = FirefoxCompressedBookmarkReader;
        let mut bookmark_file = Cursor::new(compressed_bookmarks);

        let raw_bookmarks = bookmark_reader.read(&mut bookmark_file).unwrap();
        let mut source_bookmarks = SourceBookmarks::default();
        let source = Source::new(
            SourceType::Firefox,
            &PathBuf::from("dummy_path"),
            vec![String::from("dev")],
        );

        let res = bookmark_reader.parse(&raw_bookmarks, &source, &mut source_bookmarks);
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
