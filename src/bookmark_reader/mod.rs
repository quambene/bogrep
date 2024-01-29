mod chromium;
mod firefox;
mod simple;
mod source_reader;
mod target_reader;
mod target_reader_writer;
mod target_writer;

use crate::{Source, SourceBookmarks, SourceType};
pub use chromium::{ChromiumBookmarkReader, ChromiumNoExtensionBookmarkReader};
pub use firefox::{FirefoxBookmarkReader, FirefoxCompressedBookmarkReader};
use log::debug;
pub use simple::SimpleBookmarkReader;
pub use source_reader::SourceReader;
use std::{
    fmt,
    io::Read,
    path::{Path, PathBuf},
};
pub use target_reader::ReadTarget;
pub use target_reader_writer::TargetReaderWriter;
pub use target_writer::WriteTarget;

#[derive(Debug, PartialEq)]
pub enum ReaderName {
    Firefox,
    FirefoxCompressed,
    Chromium,
    ChromiumNoExtension,
    Simple,
}

impl fmt::Display for ReaderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reader_name = match &self {
            ReaderName::Firefox => "Firefox",
            ReaderName::FirefoxCompressed => "Firefox (compressed)",
            ReaderName::Chromium => "Chromium",
            ReaderName::ChromiumNoExtension => "Chromium (no extension)",
            ReaderName::Simple => "Simple",
        };
        write!(f, "{}", reader_name)
    }
}

/// A trait to read bookmarks from multiple sources, like Firefox or Chrome.
pub trait ReadBookmark: fmt::Debug {
    fn name(&self) -> ReaderName;

    fn extension(&self) -> Option<&str>;

    /// Identify and select the source.
    ///
    /// A bookmark reader can read from multiple sources. For example, the json
    /// format for bookmarks from Chromium can be used for Chrome and Edge.
    fn select_source(&self, source_path: &Path) -> Result<Option<SourceType>, anyhow::Error>;

    /// Select the bookmarks file if the source is given as a directory.
    fn select_file(&self, _source_dir: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        Ok(None)
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
        debug!("Read bookmarks from file '{}'", source.path);

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
            Box::new(ChromiumBookmarkReader),
            Box::new(ChromiumNoExtensionBookmarkReader),
            Box::new(SimpleBookmarkReader),
        ])
    }
}

impl Default for BookmarkReaders {
    fn default() -> Self {
        Self::new()
    }
}
