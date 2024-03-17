use super::{SelectSource, SourceOs};
use crate::{
    bookmark_reader::plist_reader::traverse_plist, bookmarks::SourceBookmarkBuilder, ReadBookmark,
    Source, SourceBookmarks, SourceType,
};
use log::{debug, trace};
use plist::{Dictionary, Value};
use std::path::{Path, PathBuf};

pub type PlistBookmarkReader<'a> = Box<dyn ReadBookmark<'a, ParsedValue = plist::Value>>;

pub struct SafariSelector;

impl SafariSelector {
    pub fn new() -> Box<Self> {
        Box::new(SafariSelector)
    }
}

impl SelectSource for SafariSelector {
    fn name(&self) -> SourceType {
        SourceType::Safari
    }

    fn extension(&self) -> Option<&str> {
        Some("plist")
    }

    fn find_sources(
        &self,
        home_dir: &Path,
        source_os: &SourceOs,
    ) -> Result<Vec<PathBuf>, anyhow::Error> {
        debug!("Find sources for {}", self.name());

        let browser_dirs = match source_os {
            SourceOs::Linux => vec![],
            SourceOs::Windows => vec![],
            SourceOs::Macos => vec![home_dir.join("Library/Safari")],
        };
        let bookmark_files = browser_dirs
            .into_iter()
            .filter_map(|bookmark_dir| {
                let bookmark_file = bookmark_dir.join("Bookmarks.plist");

                if bookmark_file.is_file() {
                    Some(bookmark_file)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(bookmark_files.to_vec())
    }
}

/// A bookmark reader to read bookmarks in plist format from Safari.
#[derive(Debug)]
pub struct SafariReader;

impl SafariReader {
    pub fn new() -> Box<Self> {
        Box::new(Self)
    }

    fn select_bookmark(
        obj: &Dictionary,
        source: &Source,
        source_bookmarks: &mut SourceBookmarks,
        folder: &mut Option<String>,
    ) {
        trace!("json object: {obj:#?}");

        if let Some(Value::String(type_value)) = obj.get("WebBookmarkType") {
            if type_value == "WebBookmarkTypeLeaf" {
                if let Some(Value::String(url_value)) = obj.get("URLString") {
                    if url_value.contains("http") {
                        let source_bookmark = SourceBookmarkBuilder::new(url_value)
                            .add_source(source.source_type.to_owned())
                            .add_folder_opt(source.source_type.to_owned(), folder.to_owned())
                            .build();
                        source_bookmarks.insert(source_bookmark);
                    }
                }
            }
        }
    }

    fn select_folder(obj: &Dictionary) -> Option<&String> {
        if let Some(Value::String(type_value)) = obj.get("WebBookmarkType") {
            if type_value == "WebBookmarkTypeList" {
                if let Some(Value::String(title)) = obj.get("Title") {
                    return Some(title);
                }
            }
        }

        None
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
        traverse_plist(
            &parsed_bookmarks,
            source,
            source_bookmarks,
            Self::select_bookmark,
            Self::select_folder,
        );
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        bookmark_reader::{ParsedBookmarks, PlistReader, ReadSource, SourceReader},
        test_utils::{self, tests},
        utils,
    };
    use assert_matches::assert_matches;
    use std::{collections::HashMap, path::PathBuf};
    use tempfile::tempdir;

    #[test]
    fn test_selector_name() {
        let selector = SafariSelector;
        assert_eq!(selector.name(), SourceType::Safari);
    }

    #[test]
    fn test_find_sources_empty() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        let selector = SafariSelector;

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
    fn test_find_sources_macos() {
        let source_os = SourceOs::Macos;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        tests::create_test_files(temp_path, &source_os);

        let selector = SafariSelector;
        let res = selector.find_sources(temp_path, &source_os);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_dirs = res.unwrap();
        assert_eq!(bookmark_dirs.len(), 1);
        assert!(bookmark_dirs.contains(&temp_path.join("Library/Safari/Bookmarks.plist")));
    }

    #[cfg(not(any(target_os = "windows")))]
    #[test]
    fn test_find_sources_linux() {
        let source_os = SourceOs::Linux;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        tests::create_test_files(temp_path, &source_os);

        let selector = SafariSelector;
        let res = selector.find_sources(temp_path, &source_os);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_dirs = res.unwrap();
        assert!(bookmark_dirs.is_empty());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_find_sources_windows() {
        let source_os = SourceOs::Windows;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        tests::create_test_files(temp_path, &source_os);

        let selector = SafariSelector;
        let res = selector.find_sources(temp_path, &source_os);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_dirs = res.unwrap();
        assert!(bookmark_dirs.is_empty());
    }

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
                        .add_source(SourceType::Safari)
                        .build()
                ),
                (
                    url2.to_owned(),
                    SourceBookmarkBuilder::new(url2)
                        .add_source(SourceType::Safari)
                        .build()
                ),
                (
                    url3.to_owned(),
                    SourceBookmarkBuilder::new(url3)
                        .add_source(SourceType::Safari)
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
                    .add_source(SourceType::Safari)
                    .add_folder(SourceType::Safari, "Others")
                    .build()
            )])
        );
    }
}
