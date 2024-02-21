mod chromium;
mod firefox;
mod safari;
mod simple;
mod source_reader;
mod target_reader;
mod target_reader_writer;
mod target_writer;

use crate::{Source, SourceBookmarks, SourceType};
pub use chromium::ChromiumReader;
pub use firefox::FirefoxReader;
pub use safari::SafariReader;
pub use simple::SimpleReader;
pub use source_reader::SourceReader;
#[cfg(test)]
pub use source_reader::TextReader;
use std::{
    fmt,
    io::{BufReader, Lines, Read, Seek},
    path::{Path, PathBuf},
};
pub use target_reader::ReadTarget;
pub use target_reader_writer::TargetReaderWriter;
pub use target_writer::WriteTarget;

pub type SourceSelector = Box<dyn SelectSource>;
pub type BookmarkReader<'a, P> = Box<dyn ReadBookmark<'a, ParsedValue = P>>;

/// The parsed bookmarks from a bookmarks file.
#[derive(Debug)]
pub enum ParsedBookmarks<'a> {
    Json(serde_json::Value),
    Html(scraper::Html),
    Plist(plist::Value),
    Text(Lines<BufReader<&'a mut dyn SeekRead>>),
}

/// A trait to find the bookmarks directory in the system's directories, and/or the
/// bookmarks file within a given directory.
pub trait SelectSource {
    fn name(&self) -> SourceType;

    fn extension(&self) -> Option<&str>;

    /// Find the bookmarks directory in the system's directories.
    fn find_dir(&self, _home_dir: &Path) -> Result<Vec<PathBuf>, anyhow::Error> {
        Ok(Vec::new())
    }

    /// Select the bookmarks file if the source is given as a directory.
    fn find_file(&self, _source_dir: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        Ok(None)
    }
}

pub trait SeekRead: Seek + Read + fmt::Debug {}
impl<T> SeekRead for T where T: Seek + Read + fmt::Debug {}

/// A trait to read and parse the content for different file extensions.
pub trait ReadSource {
    fn extension(&self) -> Option<&str>;

    fn read_and_parse<'a>(
        &self,
        reader: &'a mut dyn SeekRead,
    ) -> Result<ParsedBookmarks<'a>, anyhow::Error>;
}

/// A trait to read bookmarks from multiple sources, like Firefox or Chrome.
///
/// We use a trait generic over the lifetime instead of a generic associative
/// type `ParsedValue<'a>` to achieve object safety.
pub trait ReadBookmark<'a>: fmt::Debug {
    type ParsedValue;

    fn name(&self) -> SourceType;

    fn extension(&self) -> Option<&str>;

    /// Identify and select the source.
    ///
    /// A bookmark reader can read from multiple sources. For example, the json
    /// format for bookmarks from Chromium can be used for Chrome and Edge.
    fn select_source(
        &self,
        source_path: &Path,
        parsed_bookmarks: &Self::ParsedValue,
    ) -> Result<Option<SourceType>, anyhow::Error>;

    /// Select the bookmarks file if the source is given as a directory.
    fn select_file(&self, _source_dir: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        Ok(None)
    }

    fn import(
        &self,
        source: &Source,
        parsed_bookmarks: Self::ParsedValue,
        source_bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error>;
}
