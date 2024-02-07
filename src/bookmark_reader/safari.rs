use super::ReaderName;
use crate::{ReadBookmark, Source, SourceBookmarks, SourceType};
use log::debug;
use plist::Value;
use std::path::Path;

pub struct Safari;

impl Safari {
    fn select_bookmark(source: &Source, obj: &Value, source_bookmarks: &mut SourceBookmarks) {
        todo!()
    }

    fn traverse_json(source: &Source, value: &Value, source_bookmarks: &mut SourceBookmarks) {
        todo!()
    }

    fn traverse_children(source: &Source, value: &Value, source_bookmarks: &mut SourceBookmarks) {
        todo!()
    }
}

/// A bookmark reader to read bookmarks in plist format from Safari.
#[derive(Debug)]
pub struct SafariBookmarkReader;

impl<'a> ReadBookmark<'a> for SafariBookmarkReader {
    type ParsedValue = plist::Value;

    fn name(&self) -> ReaderName {
        ReaderName::Safari
    }

    fn extension(&self) -> Option<&str> {
        Some("plist")
    }

    fn select_source(
        &self,
        source_path: &Path,
        parsed_bookmarks: &Value,
    ) -> Result<Option<SourceType>, anyhow::Error> {
        todo!()
    }

    fn import(
        &self,
        source: &Source,
        parsed_bookmarks: Value,
        source_bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        debug!("Import bookmarks from {}", self.name());
        Safari::traverse_json(source, &parsed_bookmarks, source_bookmarks);
        Ok(())
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
