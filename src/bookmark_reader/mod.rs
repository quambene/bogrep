mod chromium;
mod firefox;
mod safari;
mod simple;
mod source_reader;
mod target_reader;
mod target_reader_writer;
mod target_writer;

use crate::{Source, SourceBookmarks, SourceType};
pub use chromium::ChromiumBookmarkReader;
pub use firefox::FirefoxBookmarkReader;
pub use safari::SafariBookmarkReader;
pub use simple::SimpleBookmarkReader;
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

#[derive(Debug)]
pub enum ParsedBookmarks<'a> {
    Json(serde_json::Value),
    Html(scraper::Html),
    Plist(plist::Value),
    Text(Lines<BufReader<&'a mut dyn SeekRead>>),
}

#[derive(Debug, PartialEq)]
pub enum ReaderName {
    Firefox,
    Chromium,
    Safari,
    Simple,
}

impl fmt::Display for ReaderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reader_name = match &self {
            ReaderName::Firefox => "Firefox",
            ReaderName::Chromium => "Chromium",
            ReaderName::Safari => "Safari",
            ReaderName::Simple => "Simple",
        };
        write!(f, "{}", reader_name)
    }
}

pub trait SeekRead: Seek + Read + fmt::Debug {}
impl<T> SeekRead for T where T: Seek + Read + fmt::Debug {}

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

    fn name(&self) -> ReaderName;

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
