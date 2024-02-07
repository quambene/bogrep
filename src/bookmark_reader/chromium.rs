use super::{ReadBookmark, ReaderName};
use crate::{
    bookmarks::{Source, SourceBookmarkBuilder},
    SourceBookmarks, SourceType,
};
use anyhow::anyhow;
use log::trace;
use serde_json::{Map, Value};
use std::path::Path;

pub struct Chromium;

impl Chromium {
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
                            .add_source(&source.name)
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

/// A bookmark reader to read bookmarks in JSON format from Chromium or Chrome.
#[derive(Debug)]
pub struct ChromiumBookmarkReader;

impl ReadBookmark for ChromiumBookmarkReader {
    type ParsedValue<'a> = serde_json::Value;

    fn name(&self) -> ReaderName {
        ReaderName::Chromium
    }

    fn extension(&self) -> Option<&str> {
        Some("json")
    }

    fn select_source(
        &self,
        source_path: &Path,
        value: &Value,
    ) -> Result<Option<SourceType>, anyhow::Error> {
        match value {
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
                            SourceType::ChromiumFamily
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
        value: Value,
        bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        Chromium::traverse_json(source, &value, bookmarks);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bookmark_reader::source_reader::{JsonReader, ReadSource},
        utils,
    };
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
    };

    #[test]
    fn test_import_all() {
        let source_path = Path::new("test_data/bookmarks_chromium.json");
        assert!(source_path.exists());

        let mut bookmark_file = utils::open_file(source_path).unwrap();
        let json_reader = JsonReader;
        let parsed_bookmarks = json_reader.read_and_parse(&mut bookmark_file).unwrap();

        let bookmark_reader = ChromiumBookmarkReader;
        let mut source_bookmarks = SourceBookmarks::default();
        let source = Source::new(SourceType::Chromium, &PathBuf::from("dummy_path"), vec![]);

        let res = bookmark_reader.import(&source, parsed_bookmarks, &mut source_bookmarks);
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
                        .add_source(&SourceType::Chromium)
                        .build()
                ),
                (
                    url2.to_owned(),
                    SourceBookmarkBuilder::new(url2)
                        .add_source(&SourceType::Chromium)
                        .build()
                ),
                (
                    url3.to_owned(),
                    SourceBookmarkBuilder::new(url3)
                        .add_source(&SourceType::Chromium)
                        .build()
                ),
                (
                    url4.to_owned(),
                    SourceBookmarkBuilder::new(url4)
                        .add_source(&SourceType::Chromium)
                        .build()
                )
            ])
        );
    }

    #[test]
    fn test_parse_folder() {
        let source_path = Path::new("test_data/bookmarks_chromium.json");
        assert!(source_path.exists());

        let mut bookmark_file = utils::open_file(source_path).unwrap();
        let json_reader = JsonReader;
        let parsed_bookmarks = json_reader.read_and_parse(&mut bookmark_file).unwrap();

        let bookmark_reader = ChromiumBookmarkReader;
        let mut source_bookmarks = SourceBookmarks::default();
        let source = Source::new(
            SourceType::Chromium,
            &PathBuf::from("dummy_path"),
            vec!["dev".to_owned()],
        );

        let res = bookmark_reader.import(&source, parsed_bookmarks, &mut source_bookmarks);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let url1 = "https://en.wikipedia.org/wiki/Design_Patterns";
        let url2 = "https://doc.rust-lang.org/book/title-page.html";

        assert_eq!(
            source_bookmarks.inner(),
            HashMap::from_iter([
                (
                    url1.to_owned(),
                    SourceBookmarkBuilder::new(url1)
                        .add_source(&SourceType::Chromium)
                        .build()
                ),
                (
                    url2.to_owned(),
                    SourceBookmarkBuilder::new(url2)
                        .add_source(&SourceType::Chromium)
                        .build()
                ),
            ])
        );
    }
}
