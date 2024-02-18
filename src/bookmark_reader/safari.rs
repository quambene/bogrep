use crate::{bookmarks::SourceBookmarkBuilder, ReadBookmark, Source, SourceBookmarks, SourceType};
use log::{debug, trace};
use plist::{Dictionary, Value};
use std::path::Path;

pub type PlistBookmarkReader<'a> = Box<dyn ReadBookmark<'a, ParsedValue = plist::Value>>;

/// A bookmark reader to read bookmarks in plist format from Safari.
#[derive(Debug)]
pub struct SafariReader;

impl SafariReader {
    pub fn new() -> Box<Self> {
        Box::new(Self)
    }

    fn select_bookmark(obj: &Dictionary, source: &Source, source_bookmarks: &mut SourceBookmarks) {
        trace!("json object: {obj:#?}");

        if let Some(Value::String(type_value)) = obj.get("WebBookmarkType") {
            if type_value == "WebBookmarkTypeLeaf" {
                if let Some(Value::String(url_value)) = obj.get("URLString") {
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

    fn traverse_plist(value: &Value, source: &Source, source_bookmarks: &mut SourceBookmarks) {
        match value {
            Value::Dictionary(obj) => {
                if source.folders.is_empty() {
                    Self::select_bookmark(obj, source, source_bookmarks);

                    for (_, val) in obj {
                        Self::traverse_plist(val, source, source_bookmarks);
                    }
                } else {
                    if let Some(Value::String(type_value)) = obj.get("WebBookmarkType") {
                        if type_value == "WebBookmarkTypeList" {
                            if let Some(Value::String(name_value)) = obj.get("Title") {
                                if source.folders.contains(name_value) {
                                    for (_, val) in obj {
                                        Self::traverse_children(val, source, source_bookmarks);
                                    }
                                }
                            }
                        }
                    }

                    for (_, val) in obj {
                        Self::traverse_plist(val, source, source_bookmarks);
                    }
                }
            }
            Value::Array(arr) => {
                for val in arr {
                    Self::traverse_plist(val, source, source_bookmarks);
                }
            }
            Value::Boolean(_) => (),
            Value::Data(_) => (),
            Value::Date(_) => (),
            Value::Real(_) => (),
            Value::Integer(_) => (),
            Value::String(_) => (),
            Value::Uid(_) => (),
            _ => (),
        }
    }

    fn traverse_children(value: &Value, source: &Source, source_bookmarks: &mut SourceBookmarks) {
        match value {
            Value::Dictionary(obj) => {
                Self::select_bookmark(obj, source, source_bookmarks);

                for (_, val) in obj {
                    Self::traverse_children(val, source, source_bookmarks);
                }
            }
            Value::Array(arr) => {
                for val in arr {
                    Self::traverse_children(val, source, source_bookmarks);
                }
            }
            Value::Boolean(_) => (),
            Value::Data(_) => (),
            Value::Date(_) => (),
            Value::Real(_) => (),
            Value::Integer(_) => (),
            Value::String(_) => (),
            Value::Uid(_) => (),
            _ => (),
        }
    }
}

impl<'a> ReadBookmark<'a> for SafariReader {
    type ParsedValue = plist::Value;

    fn name(&self) -> SourceType {
        SourceType::Safari
    }

    fn extension(&self) -> Option<&str> {
        Some("plist")
    }

    fn select_source(
        &self,
        _source_path: &Path,
        _parsed_bookmarks: &Value,
    ) -> Result<Option<SourceType>, anyhow::Error> {
        // Plist files are only supported by Safari
        Ok(Some(SourceType::Safari))
    }

    fn import(
        &self,
        source: &Source,
        parsed_bookmarks: Value,
        source_bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        debug!("Import bookmarks from {:#?}", self.name());
        Self::traverse_plist(&parsed_bookmarks, source, source_bookmarks);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        bookmark_reader::{source_reader::PlistReader, ParsedBookmarks, ReadSource, SourceReader},
        test_utils, utils,
    };
    use assert_matches::assert_matches;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_read_and_parse_xml() {
        let source_path = Path::new("test_data/bookmarks_safari_xml.plist");
        let mut reader = utils::open_file(source_path).unwrap();
        let source_reader = PlistReader;

        let res = source_reader.read_and_parse(&mut reader);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let parsed_bookmarks = res.unwrap();
        assert_matches!(parsed_bookmarks, ParsedBookmarks::Plist(_));
    }

    #[test]
    fn test_read_and_parse_binary() {
        let source_path = Path::new("test_data/bookmarks_safari_binary.plist");
        test_utils::create_binary_plist_file(source_path).unwrap();
        let mut reader = utils::open_file(source_path).unwrap();
        let source_reader = PlistReader;

        let res = source_reader.read_and_parse(&mut reader);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let parsed_bookmarks = res.unwrap();
        assert_matches!(parsed_bookmarks, ParsedBookmarks::Plist(_));
    }

    #[test]
    fn test_import_all() {
        let source_path = Path::new("test_data/bookmarks_safari_xml.plist");
        assert!(source_path.exists());

        let mut source_bookmarks = SourceBookmarks::default();
        let source = Source::new(SourceType::Unknown, &PathBuf::from("dummy_path"), vec![]);
        let bookmark_file = utils::open_file(source_path).unwrap();
        let source_reader = Box::new(PlistReader);
        let mut source_reader = SourceReader::new(source, Box::new(bookmark_file), source_reader);

        let res = source_reader.import(&mut source_bookmarks);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let url1 = "https://www.deepl.com/translator";
        let url2 = "https://en.wikipedia.org/wiki/Design_Patterns";
        let url3 = "https://doc.rust-lang.org/book/title-page.html";

        assert_eq!(
            source_bookmarks.inner(),
            HashMap::from_iter([
                (
                    url1.to_owned(),
                    SourceBookmarkBuilder::new(url1)
                        .add_source(&SourceType::Safari)
                        .build()
                ),
                (
                    url2.to_owned(),
                    SourceBookmarkBuilder::new(url2)
                        .add_source(&SourceType::Safari)
                        .build()
                ),
                (
                    url3.to_owned(),
                    SourceBookmarkBuilder::new(url3)
                        .add_source(&SourceType::Safari)
                        .build()
                )
            ])
        );
    }

    #[test]
    fn test_import_folder() {
        let source_path = Path::new("test_data/bookmarks_safari_xml.plist");
        assert!(source_path.exists());

        let mut source_bookmarks = SourceBookmarks::default();
        let source = Source::new(
            SourceType::Unknown,
            &PathBuf::from("dummy_path"),
            vec!["Others".to_owned()],
        );
        let bookmark_file = utils::open_file(source_path).unwrap();
        let source_reader = Box::new(PlistReader);
        let mut source_reader = SourceReader::new(source, Box::new(bookmark_file), source_reader);

        let res = source_reader.import(&mut source_bookmarks);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let url1 = "https://www.deepl.com/translator";

        assert_eq!(
            source_bookmarks.inner(),
            HashMap::from_iter([(
                url1.to_owned(),
                SourceBookmarkBuilder::new(url1)
                    .add_source(&SourceType::Safari)
                    .build()
            )])
        );
    }
}
