mod chrome;
mod firefox;
mod simple;

use crate::{SourceBookmarks, SourceFile};
pub use chrome::ChromeBookmarkReader;
pub use firefox::FirefoxBookmarkReader;
pub use simple::SimpleBookmarkReader;
use std::path::Path;

pub trait BookmarkReader {
    const NAME: &'static str;

    fn read(&self, _source_file: &Path) -> Result<String, anyhow::Error> {
        Ok(String::default())
    }

    fn parse(
        &self,
        _raw_bookmarks: &str,
        _source_file: &SourceFile,
        _bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn read_and_parse(
        &self,
        source_file: &SourceFile,
        bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        let raw_bookmarks = self.read(&source_file.source)?;
        self.parse(&raw_bookmarks, source_file, bookmarks)?;
        Ok(())
    }
}
