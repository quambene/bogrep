use crate::bookmark_reader::SourceReader;
use log::debug;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct SourceBookmarks {
    pub bookmarks: HashSet<String>,
}

impl Default for SourceBookmarks {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceBookmarks {
    pub fn new() -> Self {
        Self {
            bookmarks: HashSet::new(),
        }
    }

    pub fn insert(&mut self, url: &str) {
        let is_new_bookmark = self.bookmarks.insert(url.to_owned());

        if !is_new_bookmark {
            debug!("Overwrite duplicate bookmark: {}", url);
        }
    }

    pub fn read(source_reader: &mut [SourceReader]) -> Result<SourceBookmarks, anyhow::Error> {
        let mut bookmarks = SourceBookmarks::new();

        for reader in source_reader {
            reader.read_and_parse(&mut bookmarks)?
        }

        Ok(bookmarks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bookmark_reader::BookmarkReaders, test_utils, Source};
    use std::path::Path;

    #[test]
    fn test_read_firefox() {
        let bookmark_path = Path::new("test_data/source/bookmarks_firefox.json");
        let bookmark_readers = BookmarkReaders::new();
        let source = Source::new(bookmark_path, vec![]);
        let source_reader = SourceReader::new(&source, &bookmark_readers.0).unwrap();
        assert_eq!(source_reader.bookmark_reader().name(), "Firefox");
    }

    #[test]
    fn test_read_firefox_compressed() {
        let bookmark_path = Path::new("test_data/source/bookmarks_firefox.jsonlz4");
        test_utils::create_compressed_bookmarks(bookmark_path);
        let bookmark_readers = BookmarkReaders::new();
        let source = Source::new(bookmark_path, vec![]);
        let source_reader = SourceReader::new(&source, &bookmark_readers.0).unwrap();
        assert_eq!(
            source_reader.bookmark_reader().name(),
            "Firefox (compressed)"
        );
    }

    #[test]
    fn test_read_chrome() {
        let bookmark_path = Path::new("test_data/source/bookmarks_chrome.json");
        let bookmark_readers = BookmarkReaders::new();
        let source = Source::new(bookmark_path, vec![]);
        let source_reader = SourceReader::new(&source, &bookmark_readers.0).unwrap();
        assert_eq!(source_reader.bookmark_reader().name(), "Chrome/Chromium");
    }

    #[test]
    fn test_read_chrome_no_extension() {
        let bookmark_path = Path::new("test_data/source/bookmarks_chrome_no_extension");
        let bookmark_readers = BookmarkReaders::new();
        let source = Source::new(bookmark_path, vec![]);
        let source_reader = SourceReader::new(&source, &bookmark_readers.0).unwrap();
        assert_eq!(
            source_reader.bookmark_reader().name(),
            "Chrome/Chromium (no extension)"
        );
    }

    #[test]
    fn test_read_simple() {
        let bookmark_path = Path::new("test_data/source/bookmarks_simple.txt");
        let bookmark_readers = BookmarkReaders::new();
        let source = Source::new(bookmark_path, vec![]);
        let source_reader = SourceReader::new(&source, &bookmark_readers.0).unwrap();
        assert_eq!(source_reader.bookmark_reader().name(), "simple");
    }
}
