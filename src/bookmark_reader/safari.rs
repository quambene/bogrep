use super::ReaderName;
use crate::{ReadBookmark, Source, SourceBookmarks, SourceType};
use plist::Value;
use std::path::Path;

pub struct Safari;

impl Safari {
    fn select_bookmark(obj: &Value, source_bookmarks: &mut SourceBookmarks, source: &Source) {
        todo!()
    }

    fn traverse_json(value: &Value, source_bookmarks: &mut SourceBookmarks, source: &Source) {
        todo!()
    }

    fn traverse_children(value: &Value, source_bookmarks: &mut SourceBookmarks, source: &Source) {
        todo!()
    }
}

/// A bookmark reader to read bookmarks in plist format from Safari.
#[derive(Debug)]
pub struct SafariBookmarkReader;

impl ReadBookmark for SafariBookmarkReader {
    type ParsedValue<'a> = plist::Value;

    fn name(&self) -> ReaderName {
        ReaderName::Safari
    }

    fn extension(&self) -> Option<&str> {
        Some("plist")
    }

    fn select_source(
        &self,
        source_path: &Path,
        value: &Value,
    ) -> Result<Option<SourceType>, anyhow::Error> {
        todo!()
    }

    fn import(
        &self,
        source: &Source,
        parsed_bookmarks: Value,
        source_bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_import_all() {
        todo!()
    }

    #[test]
    fn test_import_folders() {
        todo!()
    }
}
