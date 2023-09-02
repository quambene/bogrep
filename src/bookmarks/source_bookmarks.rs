use crate::{
    bookmark_reader::{ChromeBookmarkReader, FirefoxBookmarkReader},
    BookmarkReader, Config, SimpleBookmarkReader,
};
use anyhow::anyhow;
use log::{debug, info};
use std::collections::HashSet;

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
