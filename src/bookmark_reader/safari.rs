use crate::{ReadBookmark, Source, SourceBookmarks, SourceType};
use log::debug;
use plist::Value;
use std::path::Path;

pub type PlistBookmarkReader<'a> = Box<dyn ReadBookmark<'a, ParsedValue = plist::Value>>;

/// A bookmark reader to read bookmarks in plist format from Safari.
#[derive(Debug)]
pub struct SafariReader;

impl SafariReader {
    pub fn new() -> Box<Self> {
        Box::new(Self)
    }

    fn select_bookmark(source: &Source, obj: &Value, source_bookmarks: &mut SourceBookmarks) {
        todo!()
    }

    fn traverse_plist(source: &Source, value: &Value, source_bookmarks: &mut SourceBookmarks) {
        todo!()
    }

    fn traverse_children(source: &Source, value: &Value, source_bookmarks: &mut SourceBookmarks) {
        todo!()
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
        debug!("Import bookmarks from {:#?}", self.name());
        Self::traverse_plist(source, &parsed_bookmarks, source_bookmarks);
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
