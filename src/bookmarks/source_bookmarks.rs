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
        let bookmark_path = Path::new("test_data/source/bookmarks_firefox.jsonlz4");
        test_utils::create_compressed_bookmarks(bookmark_path);
        let bookmark_readers = BookmarkReaders::new();
        let source = Source::new(bookmark_path, vec![]);
        let source_reader = SourceReader::new(&source, &bookmark_readers.0).unwrap();
        let res = SourceBookmarks::read(&mut [source_reader]);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let source_bookmarks = res.unwrap();
        assert_eq!(source_bookmarks.bookmarks, HashSet::from_iter([
            String::from("https://www.mozilla.org/en-US/firefox/central/"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html")
        ]));
    }

    #[test]
    fn test_read_chrome() {
        let bookmark_path = Path::new("test_data/source/bookmarks_google-chrome.json");
        let bookmark_readers = BookmarkReaders::new();
        let source = Source::new(bookmark_path, vec![]);
        let source_reader = SourceReader::new(&source, &bookmark_readers.0).unwrap();
        let res = SourceBookmarks::read(&mut [source_reader]);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let source_bookmarks = res.unwrap();
        assert_eq!(source_bookmarks.bookmarks, HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]));
    }

    #[test]
    fn test_read_simple() {
        let bookmark_path = Path::new("test_data/source/bookmarks_simple.txt");
        let bookmark_readers = BookmarkReaders::new();

        let source = Source::new(bookmark_path, vec![]);
        let source_reader = SourceReader::new(&source, &bookmark_readers.0).unwrap();
        let res = SourceBookmarks::read(&mut [source_reader]);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let source_bookmarks = res.unwrap();
        assert_eq!(source_bookmarks.bookmarks, HashSet::from_iter([
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://www.quantamagazine.org/how-galois-groups-used-polynomial-symmetries-to-reshape-math-20210803/"),
            String::from("https://www.quantamagazine.org/computing-expert-says-programmers-need-more-math-20220517/"),
        ]));
    }
}
