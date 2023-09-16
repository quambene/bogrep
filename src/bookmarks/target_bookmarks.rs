use crate::{config::Config, utils, SourceBookmarks};
use anyhow::Context;
use chrono::{DateTime, Utc};
use log::{info, trace};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    slice,
};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub struct TargetBookmark {
    pub id: String,
    pub url: String,
    pub last_imported: i64,
    pub last_cached: Option<i64>,
}

impl TargetBookmark {
    pub fn new(
        url: impl Into<String>,
        last_imported: DateTime<Utc>,
        last_cached: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            url: url.into(),
            last_imported: last_imported.timestamp_millis(),
            last_cached: last_cached.map(|timestamp| timestamp.timestamp_millis()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TargetBookmarks {
    pub bookmarks: Vec<TargetBookmark>,
}

impl TargetBookmarks {
    pub fn init(config: &Config) -> Result<TargetBookmarks, anyhow::Error> {
        let bookmarks = if config.target_bookmark_file.exists() {
            info!("Bookmarks already imported");
            TargetBookmarks::read(config)?
        } else {
            let mut source_bookmarks = SourceBookmarks::new();
            source_bookmarks.read(config)?;
            let target_bookmarks = TargetBookmarks::from(source_bookmarks);
            target_bookmarks.write(config)?;

            info!(
                "Imported {} bookmarks from {} sources: {}",
                target_bookmarks.bookmarks.len(),
                config.settings.source_bookmark_files.len(),
                config
                    .settings
                    .source_bookmark_files
                    .iter()
                    .map(|source| source.path.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            target_bookmarks
        };

        Ok(bookmarks)
    }

    pub fn read(config: &Config) -> Result<TargetBookmarks, anyhow::Error> {
        let mut target_bookmarks_file = utils::open_file(&config.target_bookmark_file)?;
        let mut buffer = String::new();
        target_bookmarks_file
            .read_to_string(&mut buffer)
            .context("Can't read bookmark file")?;
        let bookmarks = serde_json::from_str(&buffer)?;
        Ok(bookmarks)
    }

    pub fn write(&self, config: &Config) -> Result<(), anyhow::Error> {
        let json_bookmarks = serde_json::to_string_pretty(self)?;
        let mut target_bookmarks_file = utils::create_file(&config.target_bookmark_file)?;
        target_bookmarks_file.write_all(json_bookmarks.as_bytes())?;
        Ok(())
    }

    pub fn add(&mut self, bookmark: &TargetBookmark) {
        self.bookmarks.push(bookmark.to_owned());
    }

    pub fn remove(&mut self, bookmark: &TargetBookmark) {
        let index = self
            .bookmarks
            .iter()
            .position(|target_bookmark| target_bookmark == bookmark);

        if let Some(index) = index {
            self.bookmarks.remove(index);
        }
    }

    pub fn find(&self, url: &str) -> Option<&TargetBookmark> {
        self.bookmarks.iter().find(|bookmark| bookmark.url == url)
    }

    pub fn filter_to_add<'a>(&self, source_bookmarks: &'a SourceBookmarks) -> Vec<&'a str> {
        source_bookmarks
            .bookmarks
            .iter()
            .filter(|bookmark_url| {
                !self
                    .bookmarks
                    .iter()
                    .any(|bookmark| &&bookmark.url == bookmark_url)
            })
            .map(|url| url.as_str())
            .collect()
    }

    pub fn filter_to_remove(&self, source_bookmarks: &SourceBookmarks) -> Vec<TargetBookmark> {
        self.bookmarks
            .iter()
            .filter(|bookmark| !source_bookmarks.bookmarks.contains(&bookmark.url))
            .cloned()
            .collect()
    }

    /// Update target bookmarks.
    ///
    /// Determine the difference between source and target bookmarks and update
    /// the target bookmarks.
    pub fn update(&mut self, source_bookmarks: SourceBookmarks) -> Result<(), anyhow::Error> {
        if self.bookmarks.is_empty() {
            self.bookmarks = Self::from(source_bookmarks.clone()).bookmarks;
            return Ok(());
        }

        let now = Utc::now();
        let bookmarks_to_add = self.filter_to_add(&source_bookmarks);
        let bookmarks_to_remove = self.filter_to_remove(&source_bookmarks);

        for url in &bookmarks_to_add {
            let bookmark = TargetBookmark::new(*url, now, None);
            self.add(&bookmark);
        }

        for bookmark in &bookmarks_to_remove {
            self.remove(bookmark);
        }

        if !bookmarks_to_add.is_empty() {
            info!("Added {} new bookmarks", bookmarks_to_add.len());
            trace!("Added new bookmarks: {bookmarks_to_add:#?}");
        }

        if !bookmarks_to_remove.is_empty() {
            info!("Removed {} bookmarks", bookmarks_to_remove.len());
            trace!("Removed bookmarks: {bookmarks_to_remove:#?}");
        }

        if bookmarks_to_add.is_empty() && bookmarks_to_remove.is_empty() {
            info!("Bookmarks are already up to date");
        }

        Ok(())
    }
}

impl<'a> IntoIterator for &'a TargetBookmarks {
    type Item = &'a TargetBookmark;
    type IntoIter = slice::Iter<'a, TargetBookmark>;

    fn into_iter(self) -> Self::IntoIter {
        self.bookmarks.iter()
    }
}

impl From<SourceBookmarks> for TargetBookmarks {
    fn from(source_bookmarks: SourceBookmarks) -> Self {
        let now = Utc::now();
        TargetBookmarks {
            bookmarks: source_bookmarks
                .bookmarks
                .into_iter()
                .map(|bookmark| TargetBookmark::new(bookmark, now, None))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Settings, Source};
    use std::{collections::HashSet, path::PathBuf};

    #[test]
    fn test_init_target_empty() {
        let settings = Settings {
            source_bookmark_files: vec![],
            ..Default::default()
        };
        let config = Config {
            target_bookmark_file: PathBuf::from("test_data/target/bookmarks_empty.json"),
            settings,
            ..Default::default()
        };
        let res = TargetBookmarks::init(&config);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let target_bookmarks = res.unwrap();
        assert_eq!(target_bookmarks.bookmarks, vec![]);
    }

    #[test]
    fn test_init_target_simple() {
        let source_path = PathBuf::from("test_data/source/bookmarks_simple.txt");
        let target_path = PathBuf::from("test_data/target/bookmarks_simple.json");

        // Prepare test
        if target_path.exists() {
            utils::remove_file(&target_path).unwrap();
        }

        let settings = Settings {
            source_bookmark_files: vec![Source {
                path: source_path,
                folders: vec![],
            }],
            ..Default::default()
        };
        let config = Config {
            target_bookmark_file: target_path,
            settings,
            ..Default::default()
        };
        let res = TargetBookmarks::init(&config);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let target_bookmarks = res.unwrap();

        assert!(target_bookmarks
            .bookmarks
            .iter()
            .all(|bookmark| bookmark.last_cached == None));
        assert_eq!(
            target_bookmarks
            .bookmarks
            .iter()
            .map(|bookmark| bookmark.url.clone())
            .collect::<HashSet<_>>(),
            HashSet::from_iter([
                String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
                String::from("https://www.quantamagazine.org/how-galois-groups-used-polynomial-symmetries-to-reshape-math-20210803/"),
                String::from("https://www.quantamagazine.org/computing-expert-says-programmers-need-more-math-20220517/")
            ])
        );
    }

    #[test]
    fn test_init_target_firefox() {
        let source_path = PathBuf::from("test_data/source/bookmarks_firefox.json");
        let target_path = PathBuf::from("test_data/target/bookmarks_firefox.json");

        // Prepare test
        if target_path.exists() {
            utils::remove_file(&target_path).unwrap();
        }

        let settings = Settings {
            source_bookmark_files: vec![Source {
                path: source_path,
                folders: vec![],
            }],
            ..Default::default()
        };
        let config = Config {
            target_bookmark_file: target_path,
            settings,
            ..Default::default()
        };
        let res = TargetBookmarks::init(&config);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let target_bookmarks = res.unwrap();

        assert!(target_bookmarks
            .bookmarks
            .iter()
            .all(|bookmark| bookmark.last_cached == None));
        assert_eq!(
            target_bookmarks
            .bookmarks
            .iter()
            .map(|bookmark| bookmark.url.clone())
            .collect::<HashSet<_>>(),
            HashSet::from_iter([
                String::from("https://www.mozilla.org/en-US/firefox/central/"),
                String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
                String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
                String::from("https://doc.rust-lang.org/book/title-page.html")
            ])
        );
    }

    #[test]
    fn test_init_target_chrome() {
        let source_path = PathBuf::from("test_data/source/bookmarks_google-chrome.json");
        let target_path = PathBuf::from("test_data/target/bookmarks_google-chrome.json");

        // Prepare test
        if target_path.exists() {
            utils::remove_file(&target_path).unwrap();
        }

        let settings = Settings {
            source_bookmark_files: vec![Source {
                path: source_path,
                folders: vec![],
            }],
            ..Default::default()
        };
        let config = Config {
            target_bookmark_file: target_path,
            settings,
            ..Default::default()
        };
        let res = TargetBookmarks::init(&config);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let target_bookmarks = res.unwrap();

        assert!(target_bookmarks
            .bookmarks
            .iter()
            .all(|bookmark| bookmark.last_cached == None));
        assert_eq!(
            target_bookmarks
            .bookmarks
            .iter()
            .map(|bookmark| bookmark.url.clone())
            .collect::<HashSet<_>>(),
            HashSet::from_iter([
                String::from("https://www.deepl.com/translator"),
                String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
                String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
                String::from("https://doc.rust-lang.org/book/title-page.html"),
            ])
        );
    }
}
