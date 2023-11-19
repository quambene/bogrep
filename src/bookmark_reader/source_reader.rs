use super::BookmarkReaders;
use crate::{bookmarks::Source, utils, ReadBookmark, SourceBookmarks};
use anyhow::anyhow;
use log::debug;
use std::{
    io::{Read, Seek},
    path::PathBuf,
};

/// A reader to read from a source, like Firefox or Chrome.
pub struct SourceReader {
    source: Source,
    bookmarks_path: PathBuf,
    reader: Box<dyn Read>,
    bookmark_reader: Box<dyn ReadBookmark>,
}

impl SourceReader {
    pub fn new(
        source: Source,
        bookmarks_path: PathBuf,
        reader: Box<dyn Read>,
        bookmark_reader: Box<dyn ReadBookmark>,
    ) -> Self {
        Self {
            source,
            bookmarks_path,
            reader,
            bookmark_reader,
        }
    }

    pub fn init(source: &Source) -> Result<Self, anyhow::Error> {
        let bookmark_readers = BookmarkReaders::new();

        for bookmark_reader in bookmark_readers.0 {
            if source.path.is_file()
                && bookmark_reader.extension()
                    == source.path.extension().and_then(|path| path.to_str())
            {
                {
                    let mut bookmark_file = utils::open_file(&source.path)?;
                    let bookmarks = bookmark_reader.read(&mut bookmark_file)?;
                    bookmark_file.rewind()?;

                    if bookmark_reader.is_selected(&bookmarks)? {
                        return Ok(SourceReader::new(
                            source.to_owned(),
                            source.path.to_owned(),
                            Box::new(bookmark_file),
                            bookmark_reader,
                        ));
                    }
                }
            } else if source.path.is_dir() {
                if let Some(bookmarks_path) = bookmark_reader.select_path(&source.path)? {
                    if bookmarks_path.is_file()
                        && bookmark_reader.extension()
                            == bookmarks_path.extension().and_then(|path| path.to_str())
                    {
                        let mut bookmark_file = utils::open_file(&bookmarks_path)?;
                        let bookmarks = bookmark_reader.read(&mut bookmark_file)?;
                        bookmark_file.rewind()?;

                        if bookmark_reader.is_selected(&bookmarks)? {
                            return Ok(SourceReader::new(
                                source.to_owned(),
                                bookmarks_path,
                                Box::new(bookmark_file),
                                bookmark_reader,
                            ));
                        }
                    }
                }
            }
        }

        Err(anyhow!(
            "Format not supported for bookmark file '{}'",
            source.path.display()
        ))
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    #[cfg(test)]
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;
    use std::path::Path;

    #[test]
    fn test_init_firefox() {
        let source_path = Path::new("test_data/bookmarks_firefox.json");
        let source_folders = vec![];
        let source = Source::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert_eq!(source_reader.bookmark_reader().name(), "Firefox");
    }

    #[test]
    fn test_init_firefox_compressed() {
        let source_path = Path::new("test_data/bookmarks_firefox.jsonlz4");
        test_utils::create_compressed_bookmarks(source_path);
        let source_folders = vec![];
        let source = Source::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert_eq!(
            source_reader.bookmark_reader().name(),
            "Firefox (compressed)"
        );
    }

    #[test]
    fn test_init_chrome() {
        let source_path = Path::new("test_data/bookmarks_chromium.json");
        let source_folders = vec![];
        let source = Source::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert_eq!(source_reader.bookmark_reader().name(), "Chromium");
    }

    #[test]
    fn test_init_chrome_no_extension() {
        let source_path = Path::new("test_data/bookmarks_chromium_no_extension");
        let source_folders = vec![];
        let source = Source::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert_eq!(
            source_reader.bookmark_reader().name(),
            "Chromium (no extension)"
        );
    }

    #[test]
    fn test_init_simple() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        let source_folders = vec![];
        let source = Source::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert_eq!(source_reader.bookmark_reader().name(), "simple");
    }
}
