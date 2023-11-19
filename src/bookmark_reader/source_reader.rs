use super::BookmarkReaders;
use crate::{
    bookmarks::{RawSource, Source},
    utils, ReadBookmark, SourceBookmarks,
};
use anyhow::anyhow;
use log::debug;
use std::io::Read;

/// A reader to read from a source, like Firefox or Chrome.
pub struct SourceReader {
    source: Source,
    reader: Box<dyn Read>,
    bookmark_reader: Box<dyn ReadBookmark>,
}

impl SourceReader {
    pub fn new(
        source: Source,
        reader: Box<dyn Read>,
        bookmark_reader: Box<dyn ReadBookmark>,
    ) -> Self {
        Self {
            source,
            reader,
            bookmark_reader,
        }
    }

    pub fn init(raw_source: &RawSource) -> Result<Self, anyhow::Error> {
        let bookmark_readers = BookmarkReaders::new();

        for bookmark_reader in bookmark_readers.0 {
            if raw_source.path.is_file()
                && bookmark_reader.extension()
                    == raw_source.path.extension().and_then(|path| path.to_str())
            {
                {
                    if let Some(source_type) = bookmark_reader.select_source(&raw_source.path)? {
                        let source =
                            Source::new(source_type, &raw_source.path, raw_source.folders.clone());
                        let bookmark_file = utils::open_file(&raw_source.path)?;

                        return Ok(SourceReader::new(
                            source,
                            Box::new(bookmark_file),
                            bookmark_reader,
                        ));
                    }
                }
            } else if raw_source.path.is_dir() {
                if let Some(bookmarks_path) = bookmark_reader.select_file(&raw_source.path)? {
                    if bookmarks_path.is_file()
                        && bookmark_reader.extension()
                            == bookmarks_path.extension().and_then(|path| path.to_str())
                    {
                        if let Some(source_type) = bookmark_reader.select_source(&bookmarks_path)? {
                            let source = Source::new(
                                source_type,
                                &bookmarks_path,
                                raw_source.folders.clone(),
                            );
                            let bookmark_file = utils::open_file(&bookmarks_path)?;

                            return Ok(SourceReader::new(
                                source,
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
            raw_source.path.display()
        ))
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    #[cfg(test)]
    pub fn bookmark_reader(&self) -> &dyn ReadBookmark {
        self.bookmark_reader.as_ref()
    }

    pub fn read_and_parse(
        &mut self,
        source_bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        debug!("Read bookmarks from file '{}'", self.source.path);

        self.bookmark_reader
            .read_and_parse(&mut self.reader, &self.source, source_bookmarks)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bookmark_reader::ReaderName, test_utils};
    use std::path::Path;

    #[test]
    fn test_init_firefox() {
        let source_path = Path::new("test_data/bookmarks_firefox.json");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert_eq!(source_reader.bookmark_reader().name(), ReaderName::Firefox);
    }

    #[test]
    fn test_init_firefox_compressed() {
        let source_path = Path::new("test_data/bookmarks_firefox.jsonlz4");
        test_utils::create_compressed_bookmarks(source_path);
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert_eq!(
            source_reader.bookmark_reader().name(),
            ReaderName::FirefoxCompressed
        );
    }

    #[test]
    fn test_init_chrome() {
        let source_path = Path::new("test_data/bookmarks_chromium.json");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert_eq!(source_reader.bookmark_reader().name(), ReaderName::Chromium);
    }

    #[test]
    fn test_init_chrome_no_extension() {
        let source_path = Path::new("test_data/bookmarks_chromium_no_extension");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert_eq!(
            source_reader.bookmark_reader().name(),
            ReaderName::ChromiumNoExtension
        );
    }

    #[test]
    fn test_init_simple() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert_eq!(source_reader.bookmark_reader().name(), ReaderName::Simple);
    }
}
