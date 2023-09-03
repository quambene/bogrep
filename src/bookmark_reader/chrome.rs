use super::BookmarkReader;
use crate::{utils, SourceBookmarks, SourceFile};
use log::{debug, trace};
use serde_json::{Map, Value};
use std::path::Path;

pub struct ChromeBookmarkReader;

impl ChromeBookmarkReader {
    fn select_bookmark(obj: &Map<String, Value>, bookmarks: &mut SourceBookmarks) {
        trace!("json object: {obj:#?}");

        if let Some(Value::String(type_value)) = obj.get("type") {
            if type_value == "url" {
                if let Some(Value::String(url_value)) = obj.get("url") {
                    if url_value.contains("http") {
                        bookmarks.insert(url_value);
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
                        if type_value == "folder" {
                            if let Some(Value::String(name_value)) = obj.get("name") {
                                if source_file.folders.contains(name_value) {
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
}

impl BookmarkReader for ChromeBookmarkReader {
    const NAME: &'static str = "Google Chrome";

    fn read(&self, bookmark_path: &Path) -> Result<String, anyhow::Error> {
        debug!("Read bookmarks from {}", Self::NAME);
        let bookmarks = utils::read_file(bookmark_path)?;
        Ok(String::from_utf8(bookmarks)?)
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
    use std::collections::HashSet;

    #[test]
    fn test_parse_all() {
        let source_path = Path::new("test_data/source/bookmarks_google-chrome.json");
        assert!(source_path.exists());

        let bookmark_reader = ChromeBookmarkReader;
        let bookmarks = bookmark_reader.read(source_path).unwrap();
        let mut source_bookmarks = SourceBookmarks::new();
        let source_file = SourceFile::new(source_path, vec![]);

        let res = bookmark_reader.parse(&bookmarks, &source_file, &mut source_bookmarks);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        assert_eq!(source_bookmarks.bookmarks, HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]));
    }

    #[test]
    fn test_parse_folder() {
        let source_path = Path::new("test_data/source/bookmarks_google-chrome.json");
        assert!(source_path.exists());

        let bookmark_reader = ChromeBookmarkReader;
        let bookmarks = bookmark_reader.read(source_path).unwrap();
        let mut source_bookmarks = SourceBookmarks::new();
        let source_file = SourceFile::new(source_path, vec![String::from("dev")]);

        let res = bookmark_reader.parse(&bookmarks, &source_file, &mut source_bookmarks);
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
