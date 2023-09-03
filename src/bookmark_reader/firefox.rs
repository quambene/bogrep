use super::BookmarkReader;
use crate::{utils, SourceBookmarks, SourceFile};
use anyhow::anyhow;
use log::{debug, trace};
use lz4::block;
use serde_json::{Map, Value};
use std::{
    borrow::Cow,
    fs::{self, DirEntry},
    path::{Path, PathBuf},
    time::SystemTime,
};

pub struct FirefoxBookmarkReader;

impl FirefoxBookmarkReader {
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

    fn traverse_json(value: &Value, bookmarks: &mut SourceBookmarks, source_file: &SourceFile) {
        match value {
            Value::Object(obj) => {
                if source_file.folders.is_empty() {
                    Self::select_bookmark(obj, bookmarks);

                    for (_, val) in obj {
                        Self::traverse_json(val, bookmarks, source_file);
                    }
                } else {
                    if let Some(Value::String(type_value)) = obj.get("type") {
                        if type_value == "text/x-moz-place-container" {
                            if let Some(Value::String(title_value)) = obj.get("title") {
                                if source_file.folders.contains(title_value) {
                                    for (_, val) in obj {
                                        Self::traverse_children(val, bookmarks, source_file);
                                    }
                                }
                            }
                        }
                    }

                    for (_, val) in obj {
                        Self::traverse_json(val, bookmarks, source_file);
                    }
                }
            }
            Value::Array(arr) => {
                for (_index, val) in arr.iter().enumerate() {
                    Self::traverse_json(val, bookmarks, source_file);
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
        bookmarks: &mut SourceBookmarks,
        _source_file: &SourceFile,
    ) {
        match value {
            Value::Object(obj) => {
                Self::select_bookmark(obj, bookmarks);

                for (_, val) in obj {
                    Self::traverse_children(val, bookmarks, _source_file);
                }
            }
            Value::Array(arr) => {
                for (_index, val) in arr.iter().enumerate() {
                    Self::traverse_children(val, bookmarks, _source_file);
                }
            }
            Value::String(_) => (),
            Value::Number(_) => (),
            Value::Bool(_) => (),
            Value::Null => (),
        }
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

impl BookmarkReader for FirefoxBookmarkReader {
    const NAME: &'static str = "Firefox";

    fn read(&self, bookmark_path: &Path) -> Result<String, anyhow::Error> {
        debug!("Read bookmarks from {}", Self::NAME);

        // The Firefox bookmarks directory contains multiple bookmark file.
        // Check if a specific file or a directory of files is given.
        let bookmark_path = if bookmark_path.is_file() {
            Cow::Borrowed(bookmark_path)
        } else if bookmark_path.is_dir() {
            let bookmark_path = Self::find_most_recent_file(bookmark_path)?;
            Cow::Owned(bookmark_path)
        } else {
            return Err(anyhow!(
                "Unexpected format for bookmark file: {}",
                bookmark_path.display()
            ));
        };

        // Import compressed bookmarks
        if bookmark_path.extension().map(|path| path.to_str()) == Some(Some("jsonlz4")) {
            let compressed_data = utils::read_file(&bookmark_path)?;

            // Skip the first 8 bytes: "mozLz40\0" (non-standard header)
            let decompressed_data = block::decompress(&compressed_data[8..], None)?;

            Ok(String::from_utf8(decompressed_data)?)
        // Import uncompressed bookmarks
        } else if bookmark_path.extension().map(|path| path.to_str()) == Some(Some("json")) {
            let bookmark_data = utils::read_file(&bookmark_path)?;
            Ok(String::from_utf8(bookmark_data)?)
        } else {
            Err(anyhow!(
                "Unexpected format for bookmark file: {}",
                bookmark_path.display()
            ))
        }
    }

    fn parse(
        &self,
        raw_bookmarks: &str,
        source_file: &SourceFile,
        bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        debug!("Parse bookmarks from {}", Self::NAME);
        let value: Value = serde_json::from_str(raw_bookmarks)?;
        Self::traverse_json(&value, bookmarks, source_file);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lz4::block;
    use std::{collections::HashSet, io::Write};

    fn compress_bookmarks(decompressed_bookmarks: &[u8], compressed_bookmark_path: &Path) {
        let compressed_data = block::compress(decompressed_bookmarks, None, true).unwrap();

        // Add non-standard header to data
        let prefix: &[u8] = b"mozLz40\0";
        let mut compressed_data_with_header =
            Vec::with_capacity(prefix.len() + compressed_data.len());
        compressed_data_with_header.extend_from_slice(prefix);
        compressed_data_with_header.extend_from_slice(&compressed_data);

        let mut file = utils::create_file(compressed_bookmark_path).unwrap();
        file.write_all(&compressed_data_with_header).unwrap();
        file.flush().unwrap();
    }

    #[test]
    fn test_read() {
        let decompressed_bookmark_path = Path::new("test_data/source/bookmarks_firefox.json");
        assert!(decompressed_bookmark_path.exists());
        let decompressed_bookmarks = utils::read_file(decompressed_bookmark_path).unwrap();

        let compressed_bookmark_path = Path::new("test_data/source/bookmarks_firefox.jsonlz4");
        compress_bookmarks(&decompressed_bookmarks, compressed_bookmark_path);
        assert!(compressed_bookmark_path.exists());

        let bookmark_reader = FirefoxBookmarkReader;
        let bookmarks = bookmark_reader.read(compressed_bookmark_path);
        assert!(bookmarks.is_ok(), "{}", bookmarks.unwrap_err());
        let bookmarks = bookmarks.unwrap();

        assert_eq!(
            bookmarks,
            String::from_utf8(decompressed_bookmarks).unwrap()
        );
    }

    #[test]
    fn test_parse_all() {
        let source_path = Path::new("test_data/source/bookmarks_firefox.json");
        assert!(source_path.exists());

        let bookmark_reader = FirefoxBookmarkReader;
        let raw_bookmarks = bookmark_reader.read(source_path).unwrap();
        let mut source_bookmarks = SourceBookmarks::new();
        let source_file = SourceFile::new(source_path, vec![]);

        let res = bookmark_reader.parse(&raw_bookmarks, &source_file, &mut source_bookmarks);
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
        let source_path = Path::new("test_data/source/bookmarks_firefox.json");
        assert!(source_path.exists());

        let bookmark_reader = FirefoxBookmarkReader;
        let raw_bookmarks = bookmark_reader.read(source_path).unwrap();
        let mut source_bookmarks = SourceBookmarks::new();
        let source_file = SourceFile::new(source_path, vec![String::from("dev")]);

        let res = bookmark_reader.parse(&raw_bookmarks, &source_file, &mut source_bookmarks);
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
