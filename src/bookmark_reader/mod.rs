mod chrome;
mod firefox;
mod simple;
mod source_reader;
mod target_reader;
mod target_reader_writer;
mod target_writer;

use crate::{Source, SourceBookmarks};
pub use chrome::{ChromeBookmarkReader, ChromeNoExtensionBookmarkReader};
pub use firefox::{FirefoxBookmarkReader, FirefoxCompressedBookmarkReader};
use log::debug;
pub use simple::SimpleBookmarkReader;
pub use source_reader::SourceReader;
use std::{
    fmt,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
pub use target_reader::ReadTarget;
pub use target_reader_writer::TargetReaderWriter;
pub use target_writer::WriteTarget;

/// A trait to read bookmarks from multiple sources, like Firefox or Chrome.
pub trait ReadBookmark: fmt::Debug {
    fn name(&self) -> &str;

    fn extension(&self) -> Option<&str>;

    fn is_selected(&self, bookmarks: &str) -> Result<bool, anyhow::Error>;

    fn select_path(&self, _path: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        Ok(None)
    }

    fn open(&self, path: &Path) -> Result<File, anyhow::Error> {
        let file = File::open(path)?;
        Ok(file)
    }

    fn read(&self, reader: &mut dyn Read) -> Result<String, anyhow::Error> {
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
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
        reader: &mut dyn Read,
        source: &Source,
        bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        debug!("Read bookmarks from file '{}'", source.path.display());

        let raw_bookmarks = self.read(reader)?;
        self.parse(&raw_bookmarks, source, bookmarks)?;
        Ok(())
    }
}

pub struct BookmarkReaders(pub Vec<Box<dyn ReadBookmark>>);

impl BookmarkReaders {
    pub fn new() -> Self {
        BookmarkReaders(vec![
            Box::new(FirefoxBookmarkReader),
            Box::new(FirefoxCompressedBookmarkReader),
            Box::new(ChromeBookmarkReader),
            Box::new(ChromeNoExtensionBookmarkReader),
            Box::new(SimpleBookmarkReader),
        ])
    }
}

impl Default for BookmarkReaders {
    fn default() -> Self {
        Self::new()
    }
}
