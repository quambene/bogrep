use crate::{
    bookmark_reader::{ChromeBookmarkReader, FirefoxBookmarkReader},
    BookmarkReader, Config, SimpleBookmarkReader,
};
use anyhow::anyhow;
use log::{debug, info};
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

    pub fn read(&mut self, config: &Config) -> Result<(), anyhow::Error> {
        for bookmark_file in &config.settings.source_bookmark_files {
            debug!(
                "Read bookmarks from file '{}'",
                bookmark_file.source.display()
            );

            if config.verbosity >= 1 {
                info!(
                    "Read bookmarks from file '{}'",
                    bookmark_file.source.display()
                );
            }

            let path_str = bookmark_file.source.to_str().unwrap_or("");

            if path_str.contains("firefox") {
                let firefox_reader = FirefoxBookmarkReader;
                firefox_reader.read_and_parse(bookmark_file, self)?;
            } else if path_str.contains("google-chrome") {
                let chrome_reader = ChromeBookmarkReader;
                chrome_reader.read_and_parse(bookmark_file, self)?;
            } else if bookmark_file.source.extension().map(|path| path.to_str())
                == Some(Some("txt"))
            {
                let simple_reader = SimpleBookmarkReader;
                simple_reader.read_and_parse(bookmark_file, self)?;
            } else {
                return Err(anyhow!(
                    "Format not supported for bookmark file '{}'",
                    bookmark_file.source.display()
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Settings, SourceFile};
    use std::path::PathBuf;

    #[test]
    fn test_read_empty() {
        let mut source_bookmarks = SourceBookmarks::new();
        let settings = Settings {
            source_bookmark_files: vec![],
            ..Default::default()
        };
        let config = Config {
            settings,
            ..Default::default()
        };
        let res = source_bookmarks.read(&config);
        assert!(res.is_ok());
        assert_eq!(source_bookmarks.bookmarks, HashSet::new());
    }

    #[test]
    fn test_read_firefox() {
        let mut source_bookmarks = SourceBookmarks::new();
        let settings = Settings {
            source_bookmark_files: vec![SourceFile {
                source: PathBuf::from("test_data/source/bookmarks_firefox.json"),
                folders: vec![],
            }],
            ..Default::default()
        };
        let config = Config {
            settings,
            ..Default::default()
        };
        let res = source_bookmarks.read(&config);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        assert_eq!(source_bookmarks.bookmarks, HashSet::from_iter([
            String::from("https://www.mozilla.org/en-US/firefox/central/"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html")
        ]));
    }

    #[test]
    fn test_read_chrome() {
        let mut source_bookmarks = SourceBookmarks::new();
        let settings = Settings {
            source_bookmark_files: vec![SourceFile {
                source: PathBuf::from("test_data/source/bookmarks_google-chrome.json"),
                folders: vec![],
            }],
            ..Default::default()
        };
        let config = Config {
            settings,
            ..Default::default()
        };
        let res = source_bookmarks.read(&config);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        assert_eq!(source_bookmarks.bookmarks, HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]));
    }

    #[test]
    fn test_read_simple() {
        let mut source_bookmarks = SourceBookmarks::new();
        let settings = Settings {
            source_bookmark_files: vec![SourceFile {
                source: PathBuf::from("test_data/source/bookmarks_simple.txt"),
                folders: vec![],
            }],
            ..Default::default()
        };
        let config = Config {
            settings,
            ..Default::default()
        };
        let res = source_bookmarks.read(&config);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        assert_eq!(source_bookmarks.bookmarks, HashSet::from_iter([
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://www.quantamagazine.org/how-galois-groups-used-polynomial-symmetries-to-reshape-math-20210803/"),
            String::from("https://www.quantamagazine.org/computing-expert-says-programmers-need-more-math-20220517/"),
        ]));
    }
}
