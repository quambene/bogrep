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
#[cfg(test)]
pub use source_reader::{ReadSource, TextReader};
pub use source_reader::{SeekRead, SourceReader};
use std::{
    fmt,
    path::{Path, PathBuf},
};
pub use target_reader::ReadTarget;
pub use target_reader_writer::TargetReaderWriter;
pub use target_writer::WriteTarget;

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
