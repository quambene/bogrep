mod chrome;
mod firefox;
mod simple;

use crate::{Source, SourceBookmarks};
pub use chrome::ChromeBookmarkReader;
pub use firefox::FirefoxBookmarkReader;
pub use simple::SimpleBookmarkReader;
use std::{fs::File, io::Read, path::PathBuf};

pub trait BookmarkReader {
    const NAME: &'static str;

    fn path(&self) -> Result<PathBuf, anyhow::Error>;

    fn open(&self) -> Result<File, anyhow::Error> {
        let path = self.path()?;
        let file = File::open(path)?;
        Ok(file)
    }

    fn read(&self, source_reader: &mut impl Read) -> Result<String, anyhow::Error> {
        let mut buf = String::new();
        source_reader.read_to_string(&mut buf)?;
        Ok(buf)
    }

    fn parse(
        &self,
        _raw_bookmarks: &str,
        _source: &Source,
        _bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn read_and_parse(
        &self,
        source: &Source,
        bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        let mut file = self.open()?;
        let raw_bookmarks = self.read(&mut file)?;
        self.parse(&raw_bookmarks, source, bookmarks)?;
        Ok(())
    }
}
