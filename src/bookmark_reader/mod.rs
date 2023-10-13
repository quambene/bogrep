mod chrome;
mod firefox;
mod simple;

use crate::{Source, SourceBookmarks};
use anyhow::anyhow;
pub use chrome::ChromeBookmarkReader;
pub use firefox::FirefoxBookmarkReader;
use log::debug;
pub use simple::SimpleBookmarkReader;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub enum SourceType {
    Firefox,
    GoogleChrome,
    TextFile,
}

pub trait ReadBookmark {
    fn name(&self) -> &str;

    fn validate_path(&self, path: &Path) -> Result<PathBuf, anyhow::Error>;

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

pub struct SourceReader {
    source: Source,
    bookmark_path: PathBuf,
    reader: Box<dyn Read>,
    bookmark_reader: Box<dyn ReadBookmark>,
}

impl SourceReader {
    pub fn new(source: &Source) -> Result<Self, anyhow::Error> {
        let source_type = Self::select_source(&source.path)?;
        let bookmark_reader = Self::select_reader(&source_type);
        let bookmark_path = bookmark_reader.validate_path(&source.path)?;
        let file = bookmark_reader.open(&bookmark_path)?;
        let reader = Box::new(file);

        Ok(Self {
            source: source.to_owned(),
            bookmark_path,
            reader,
            bookmark_reader,
        })
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    pub fn read_and_parse(&mut self, bookmarks: &mut SourceBookmarks) -> Result<(), anyhow::Error> {
        debug!(
            "Read bookmarks from file '{}'",
            self.bookmark_path.display()
        );

        self.bookmark_reader
            .read_and_parse(&mut self.reader, &self.source, bookmarks)?;
        Ok(())
    }

    fn select_reader(source_type: &SourceType) -> Box<dyn ReadBookmark> {
        match source_type {
            SourceType::Firefox => Box::new(FirefoxBookmarkReader),
            SourceType::GoogleChrome => Box::new(ChromeBookmarkReader),
            SourceType::TextFile => Box::new(SimpleBookmarkReader),
        }
    }

    fn select_source(source_path: &Path) -> Result<SourceType, anyhow::Error> {
        let path_str = source_path.to_str().unwrap();

        if path_str.contains("firefox") {
            Ok(SourceType::Firefox)
        } else if path_str.contains("google-chrome") {
            Ok(SourceType::GoogleChrome)
        } else if source_path.extension().map(|path| path.to_str()) == Some(Some("txt")) {
            Ok(SourceType::TextFile)
        } else {
            Err(anyhow!(
                "Format not supported for bookmark file '{}'",
                source_path.display()
            ))
        }
    }
}
