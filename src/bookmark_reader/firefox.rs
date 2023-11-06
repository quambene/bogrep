use super::ReadBookmark;
use crate::{Source, SourceBookmarks};
use anyhow::anyhow;
use log::{debug, trace};
use lz4::block;
use serde_json::{Map, Value};
use std::{
    fs::{self, DirEntry, File},
    io::Read,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub struct Firefox;

impl Firefox {
    fn select_bookmark(obj: &Map<String, Value>, bookmarks: &mut SourceBookmarks) {
        trace!("json object: {obj:#?}");

        if let Some(Value::String(type_value)) = obj.get("type") {
            if type_value == "text/x-moz-place" {
                if let Some(Value::String(uri_value)) = obj.get("uri") {
                    if uri_value.contains("http") {
                        bookmarks.insert(uri_value);
                    }
                }
            }
        }
    }

    pub fn traverse_json(value: &Value, bookmarks: &mut SourceBookmarks, source: &Source) {
        match value {
            Value::Object(obj) => {
                if source.folders.is_empty() {
                    Self::select_bookmark(obj, bookmarks);

                    for (_, val) in obj {
                        Self::traverse_json(val, bookmarks, source);
                    }
                } else {
                    if let Some(Value::String(type_value)) = obj.get("type") {
                        if type_value == "text/x-moz-place-container" {
                            if let Some(Value::String(title_value)) = obj.get("title") {
                                if source.folders.contains(title_value) {
                                    for (_, val) in obj {
                                        Self::traverse_children(val, bookmarks);
                                    }
                                }
                            }
                        }
                    }

                    for (_, val) in obj {
                        Self::traverse_json(val, bookmarks, source);
                    }
                }
            }
            Value::Array(arr) => {
                for (_index, val) in arr.iter().enumerate() {
                    Self::traverse_json(val, bookmarks, source);
                }
            }
            Value::String(_) => (),
            Value::Number(_) => (),
            Value::Bool(_) => (),
            Value::Null => (),
        }
    }

    fn traverse_children(value: &Value, bookmarks: &mut SourceBookmarks) {
        match value {
            Value::Object(obj) => {
                Self::select_bookmark(obj, bookmarks);

                for (_, val) in obj {
                    Self::traverse_children(val, bookmarks);
                }
            }
            Value::Array(arr) => {
                for (_index, val) in arr.iter().enumerate() {
                    Self::traverse_children(val, bookmarks);
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

            if bookmark_path.is_file()
                && bookmark_path.extension().map(|path| path.to_str()) == Some(Some("jsonlz4"))
            {
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

#[derive(Copy, Clone)]
pub struct FirefoxBookmarkReader;

impl ReadBookmark for FirefoxBookmarkReader {
    fn name(&self) -> &str {
        "Firefox"
    }

    fn extension(&self) -> Option<&str> {
        Some("json")
    }

    fn select(&self, raw_bookmarks: &str) -> Result<Option<Box<dyn ReadBookmark>>, anyhow::Error> {
        let value: Value = serde_json::from_str(&raw_bookmarks)?;

        match value {
            Value::Object(obj) => {
                if let Some(Value::String(type_value)) = obj.get("type") {
                    if type_value == "text/x-moz-place-container" {
                        Ok(Some(Box::new(*self)))
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

    fn select_path(&self, source_path: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        let path_str = source_path
            .to_str()
            .ok_or(anyhow!("Invalid path: source path contains invalid UTF-8"))?;

        // On Linux, the path contains lowercase identifier; on macOS uppercase
        // identifier is required.
        if path_str.contains("firefox") || path_str.contains("Firefox") {
            // The Firefox bookmarks directory contains multiple bookmark file.
            // Check if a specific file or a directory of files is given.
            let bookmark_path = Firefox::find_most_recent_file(&source_path)?;
            Ok(Some(bookmark_path))
        } else {
            Err(anyhow!(
                "Unexpected format for source file: {}",
                source_path.display()
            ))
        }
    }

    fn open(&self, source_path: &Path) -> Result<File, anyhow::Error> {
        let bookmark_file = File::open(source_path)?;
        Ok(bookmark_file)
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

#[derive(Copy, Clone)]
pub struct FirefoxCompressedBookmarkReader;

impl ReadBookmark for FirefoxCompressedBookmarkReader {
    fn name(&self) -> &str {
        "Firefox (compressed)"
    }

    fn extension(&self) -> Option<&str> {
        Some("jsonlz4")
    }

    fn select(&self, raw_bookmarks: &str) -> Result<Option<Box<dyn ReadBookmark>>, anyhow::Error> {
        let value: Value = serde_json::from_str(&raw_bookmarks)?;

        match value {
            Value::Object(obj) => {
                if let Some(Value::String(type_value)) = obj.get("type") {
                    if type_value == "text/x-moz-place-container" {
                        Ok(Some(Box::new(*self)))
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

    fn select_path(&self, source_path: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        let path_str = source_path
            .to_str()
            .ok_or(anyhow!("Invalid path: source path contains invalid UTF-8"))?;

        // On Linux, the path contains lowercase identifier; on macOS uppercase
        // identifier is required.
        if path_str.contains("firefox") || path_str.contains("Firefox") {
            // The Firefox bookmarks directory contains multiple bookmark file.
            // Check if a specific file or a directory of files is given.
            let bookmark_path = Firefox::find_most_recent_file(&source_path)?;
            Ok(Some(bookmark_path))
        } else {
            Err(anyhow!(
                "Unexpected format for source file: {}",
                source_path.display()
            ))
        }
    }

    fn open(&self, source_path: &Path) -> Result<File, anyhow::Error> {
        let bookmark_file = File::open(source_path)?;
        Ok(bookmark_file)
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
    use std::{collections::HashSet, io::Cursor};

    #[test]
    fn test_read() {
        let decompressed_bookmark_path = Path::new("test_data/source/bookmarks_firefox.json");
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
        let decompressed_bookmark_path = Path::new("test_data/source/bookmarks_firefox.json");
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
        let decompressed_bookmark_path = Path::new("test_data/source/bookmarks_firefox.json");
        let decompressed_bookmarks = utils::read_file(decompressed_bookmark_path).unwrap();
        let bookmark_reader = FirefoxBookmarkReader;
        let mut bookmark_file = Cursor::new(decompressed_bookmarks);

        let raw_bookmarks = bookmark_reader.read(&mut bookmark_file).unwrap();
        let mut source_bookmarks = SourceBookmarks::new();
        let source = Source::new("dummy_path", vec![]);

        let res = bookmark_reader.parse(&raw_bookmarks, &source, &mut source_bookmarks);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        assert_eq!(source_bookmarks.bookmarks, HashSet::from_iter([
            String::from("https://www.mozilla.org/en-US/firefox/central/"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html")
        ]));
    }

    #[test]
    fn test_parse_all_compressed() {
        let decompressed_bookmark_path = Path::new("test_data/source/bookmarks_firefox.json");
        let decompressed_bookmarks = utils::read_file(decompressed_bookmark_path).unwrap();
        let compressed_bookmarks = test_utils::compress_bookmarks(&decompressed_bookmarks);
        let bookmark_reader = FirefoxCompressedBookmarkReader;
        let mut bookmark_file = Cursor::new(compressed_bookmarks);

        let raw_bookmarks = bookmark_reader.read(&mut bookmark_file).unwrap();
        let mut source_bookmarks = SourceBookmarks::new();
        let source = Source::new("dummy_path", vec![]);

        let res = bookmark_reader.parse(&raw_bookmarks, &source, &mut source_bookmarks);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        assert_eq!(source_bookmarks.bookmarks, HashSet::from_iter([
            String::from("https://www.mozilla.org/en-US/firefox/central/"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html")
        ]));
    }

    #[test]
    fn test_parse_folder() {
        let decompressed_bookmark_path = Path::new("test_data/source/bookmarks_firefox.json");
        let decompressed_bookmarks = utils::read_file(decompressed_bookmark_path).unwrap();
        let bookmark_reader = FirefoxBookmarkReader;
        let mut bookmark_file = Cursor::new(decompressed_bookmarks);

        let raw_bookmarks = bookmark_reader.read(&mut bookmark_file).unwrap();
        let mut source_bookmarks = SourceBookmarks::new();
        let source = Source::new("dummy_path", vec![String::from("dev")]);

        let res = bookmark_reader.parse(&raw_bookmarks, &source, &mut source_bookmarks);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        assert_eq!(
            source_bookmarks.bookmarks,
            HashSet::from_iter([
                String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
                String::from("https://doc.rust-lang.org/book/title-page.html"),
            ])
        );
    }

    #[test]
    fn test_parse_folder_compressed() {
        let decompressed_bookmark_path = Path::new("test_data/source/bookmarks_firefox.json");
        let decompressed_bookmarks = utils::read_file(decompressed_bookmark_path).unwrap();
        let compressed_bookmarks = test_utils::compress_bookmarks(&decompressed_bookmarks);
        let bookmark_reader = FirefoxCompressedBookmarkReader;
        let mut bookmark_file = Cursor::new(compressed_bookmarks);

        let raw_bookmarks = bookmark_reader.read(&mut bookmark_file).unwrap();
        let mut source_bookmarks = SourceBookmarks::new();
        let source = Source::new("dummy_path", vec![String::from("dev")]);

        let res = bookmark_reader.parse(&raw_bookmarks, &source, &mut source_bookmarks);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        assert_eq!(
            source_bookmarks.bookmarks,
            HashSet::from_iter([
                String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
                String::from("https://doc.rust-lang.org/book/title-page.html"),
            ])
        );
    }
}
