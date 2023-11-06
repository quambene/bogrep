mod chrome;
mod firefox;
mod simple;

use crate::{utils, Source, SourceBookmarks};
use anyhow::anyhow;
pub use chrome::{ChromeBookmarkReader, ChromeNoExtensionBookmarkReader};
pub use firefox::{FirefoxBookmarkReader, FirefoxCompressedBookmarkReader};
use log::debug;
pub use simple::SimpleBookmarkReader;
use std::{
    fs::File,
    io::{Read, Seek},
    path::{Path, PathBuf},
};

pub trait ReadBookmark {
    fn name(&self) -> &str;

    fn extension(&self) -> Option<&str>;

    fn select(&self, bookmarks: &str) -> Result<Option<Box<dyn ReadBookmark>>, anyhow::Error>;

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

pub struct SourceReader {
    source: Source,
    bookmarks_path: PathBuf,
    reader: Box<dyn Read>,
    bookmark_reader: Box<dyn ReadBookmark>,
}

impl SourceReader {
    pub fn new(
        source: &Source,
        bookmark_readers: &[Box<dyn ReadBookmark>],
    ) -> Result<Self, anyhow::Error> {
        let (bookmark_reader, reader, bookmarks_path) =
            Self::select_reader(&source.path, bookmark_readers)?;

        Ok(Self {
            source: source.to_owned(),
            bookmarks_path,
            reader: Box::new(reader),
            bookmark_reader,
        })
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    pub fn bookmark_reader(&self) -> &dyn ReadBookmark {
        self.bookmark_reader.as_ref()
    }

    pub fn read_and_parse(&mut self, bookmarks: &mut SourceBookmarks) -> Result<(), anyhow::Error> {
        debug!(
            "Read bookmarks from file '{}'",
            self.bookmarks_path.display()
        );

        self.bookmark_reader
            .read_and_parse(&mut self.reader, &self.source, bookmarks)?;
        Ok(())
    }

    pub fn select_reader(
        source_path: &Path,
        bookmark_readers: &[Box<dyn ReadBookmark>],
    ) -> Result<(Box<dyn ReadBookmark>, impl Read, PathBuf), anyhow::Error> {
        for bookmark_reader in bookmark_readers {
            if source_path.is_file()
                && bookmark_reader.extension()
                    == source_path.extension().and_then(|path| path.to_str())
            {
                {
                    let mut bookmark_file = utils::open_file(source_path)?;
                    let bookmarks = bookmark_reader.read(&mut bookmark_file)?;
                    bookmark_file.rewind()?;
                    let bookmark_reader = bookmark_reader.select(&bookmarks)?;

                    if let Some(bookmark_reader) = bookmark_reader {
                        return Ok((bookmark_reader, bookmark_file, source_path.to_owned()));
                    }
                }
            } else if source_path.is_dir() {
                if let Some(bookmarks_path) = bookmark_reader.select_path(source_path)? {
                    if bookmarks_path.is_file()
                        && bookmark_reader.extension()
                            == bookmarks_path.extension().and_then(|path| path.to_str())
                    {
                        let mut bookmark_file = utils::open_file(&bookmarks_path)?;
                        let bookmarks = bookmark_reader.read(&mut bookmark_file)?;
                        bookmark_file.rewind()?;
                        let bookmark_reader = bookmark_reader.select(&bookmarks)?;

                        if let Some(bookmark_reader) = bookmark_reader {
                            return Ok((bookmark_reader, bookmark_file, bookmarks_path));
                        }
                    }
                }
            }
        }

        Err(anyhow!(
            "Format not supported for bookmark file '{}'",
            source_path.display()
        ))
    }
}
