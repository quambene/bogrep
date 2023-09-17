use super::ReadBookmark;
use crate::{Source, SourceBookmarks};
use anyhow::anyhow;
use log::{debug, trace};
use serde_json::{Map, Value};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

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

    fn traverse_json(value: &Value, bookmarks: &mut SourceBookmarks, source: &Source) {
        match value {
            Value::Object(obj) => {
                if source.folders.is_empty() {
                    Self::select_bookmark(obj, bookmarks);

                    for (_, val) in obj {
                        Self::traverse_json(val, bookmarks, source);
                    }
                } else {
                    if let Some(Value::String(type_value)) = obj.get("type") {
                        if type_value == "folder" {
                            if let Some(Value::String(name_value)) = obj.get("name") {
                                if source.folders.contains(name_value) {
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
}

impl ReadBookmark for ChromeBookmarkReader {
    fn name(&self) -> &'static str {
        "Google Chrome"
    }

    fn validate_path(&self, path: &Path) -> Result<PathBuf, anyhow::Error> {
        if path.exists() && path.is_file() {
            Ok(path.to_owned())
        } else {
            Err(anyhow!(
                "Missing source file for {}: {}",
                self.name(),
                path.display()
            ))
        }
    }

    fn read(&self, reader: &mut dyn Read) -> Result<String, anyhow::Error> {
        debug!("Read bookmarks from {}", self.name());
        let mut bookmarks = Vec::new();
        reader.read_to_end(&mut bookmarks)?;
        Ok(String::from_utf8(bookmarks)?)
    }

    fn parse(
        &self,
        raw_bookmarks: &str,
        source: &Source,
        bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        debug!("Parse bookmarks from {}", self.name());
        let value: Value = serde_json::from_str(raw_bookmarks)?;
        Self::traverse_json(&value, bookmarks, source);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, path::Path};

    #[test]
    fn test_parse_all() {
        let source_path = Path::new("test_data/source/bookmarks_google-chrome.json");
        assert!(source_path.exists());

        let bookmark_reader = ChromeBookmarkReader;
        let mut bookmark_file = bookmark_reader.open(source_path).unwrap();

        let bookmarks = bookmark_reader.read(&mut bookmark_file).unwrap();
        let mut source_bookmarks = SourceBookmarks::new();
        let source = Source::new(source_path, vec![]);

        let res = bookmark_reader.parse(&bookmarks, &source, &mut source_bookmarks);
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
        let mut bookmark_file = bookmark_reader.open(source_path).unwrap();

        let bookmarks = bookmark_reader.read(&mut bookmark_file).unwrap();
        let mut source_bookmarks = SourceBookmarks::new();
        let source = Source::new(source_path, vec![String::from("dev")]);

        let res = bookmark_reader.parse(&bookmarks, &source, &mut source_bookmarks);
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
