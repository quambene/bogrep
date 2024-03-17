mod chrome;
mod chromium;
mod edge;
mod firefox;
mod json_reader;
mod plist_reader;
mod safari;
mod simple;
mod source_reader;
mod target_reader;
mod target_reader_writer;
mod target_writer;
mod text_reader;

use crate::{Source, SourceBookmarks, SourceType};
pub use chromium::ChromiumReader;
pub use firefox::FirefoxReader;
pub use json_reader::{CompressedJsonReader, JsonReader, JsonReaderNoExtension};
pub use plist_reader::PlistReader;
pub use safari::SafariReader;
pub use simple::SimpleReader;
pub use source_reader::SourceReader;
use std::{
    fmt,
    io::{BufReader, Lines, Read, Seek},
    path::{Path, PathBuf},
};
pub use target_reader::ReadTarget;
pub use target_reader_writer::TargetReaderWriter;
pub use target_writer::WriteTarget;
pub use text_reader::TextReader;

pub type SourceSelector = Box<dyn SelectSource>;
pub type BookmarkReader<'a, P> = Box<dyn ReadBookmark<'a, ParsedValue = P>>;

/// The supported operating systems.
#[non_exhaustive]
#[derive(Debug)]
pub enum SourceOs {
    Linux,
    Macos,
    Windows,
}

/// The parsed bookmarks from a bookmarks file.
#[derive(Debug)]
pub enum ParsedBookmarks<'a> {
    Json(serde_json::Value),
    #[allow(dead_code)]
    Html(scraper::Html),
    Plist(plist::Value),
    Text(Lines<BufReader<&'a mut dyn SeekRead>>),
}

/// A trait to find the bookmarks directory in the system's directories, and/or
/// the bookmarks file within a given directory.
pub trait SelectSource {
    fn name(&self) -> SourceType;

    /// The extension of the bookmarks file.
    fn extension(&self) -> Option<&str>;

    /// Find the source files for a given browser and file format.
    ///
    /// `home_dir` is provided as e.g.
    ///     /home/alice on Linux
    ///     /Users/Alice on macOS
    ///     C:\Users\Alice on Windows
    fn find_sources(
        &self,
        _home_dir: &Path,
        _source_os: &SourceOs,
    ) -> Result<Vec<PathBuf>, anyhow::Error>;

    /// Select the bookmarks file if the source is given as a directory.
    ///
    /// `source_dir` is the directory which contains the bookmarks file.
    fn find_source_file(&self, _source_dir: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        Ok(None)
    }
}

pub trait SeekRead: Seek + Read + fmt::Debug {}
impl<T> SeekRead for T where T: Seek + Read + fmt::Debug {}

/// A trait to read and parse the content for different file extensions.
pub trait ReadSource: fmt::Debug {
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
