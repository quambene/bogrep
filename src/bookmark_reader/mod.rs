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
pub use source_reader::{
    CompressedJsonReader, JsonReader, PlistReader, ReadSource, SeekRead, SourceReader, TextReader,
};
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
    FirefoxCompressed,
    Chromium,
    ChromiumNoExtension,
    Safari,
    Simple,
}

impl fmt::Display for ReaderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reader_name = match &self {
            ReaderName::Firefox => "Firefox",
            ReaderName::FirefoxCompressed => "Firefox (compressed)",
            ReaderName::Chromium => "Chromium",
            ReaderName::ChromiumNoExtension => "Chromium (no extension)",
            ReaderName::Safari => "Safari",
            ReaderName::Simple => "Simple",
        };
        write!(f, "{}", reader_name)
    }
}

/// A trait to read bookmarks from multiple sources, like Firefox or Chrome.
pub trait ReadBookmark: fmt::Debug {
    // TODO: remove life lifetime parameter for object safety
    type ParsedValue<'a>;

    fn name(&self) -> ReaderName;

    fn extension(&self) -> Option<&str>;

    /// Identify and select the source.
    ///
    /// A bookmark reader can read from multiple sources. For example, the json
    /// format for bookmarks from Chromium can be used for Chrome and Edge.
    fn select_source<'a>(
        &self,
        source_path: &Path,
        parsed_bookmarks: &Self::ParsedValue<'a>,
    ) -> Result<Option<SourceType>, anyhow::Error>;

    /// Select the bookmarks file if the source is given as a directory.
    fn select_file(&self, _source_dir: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        Ok(None)
    }

    fn import<'a>(
        &self,
        source: &Source,
        parsed_bookmarks: Self::ParsedValue<'a>,
        source_bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error>;
}
