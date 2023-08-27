use crate::{config::Config, SourceBookmarks};
use anyhow::Context;
use chrono::{DateTime, Utc};
use log::{info, trace};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    slice,
};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
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
    pub fn read(config: &Config) -> Result<TargetBookmarks, anyhow::Error> {
        let mut target_bookmarks_file = File::open(&config.target_bookmark_file)?;
        let mut buffer = String::new();
        target_bookmarks_file
            .read_to_string(&mut buffer)
            .context("Can't read bookmark file")?;
        let bookmarks = serde_json::from_str(&buffer)?;
        Ok(bookmarks)
    }

    pub fn write(&self, config: &Config) -> Result<(), anyhow::Error> {
        let json_bookmarks = serde_json::to_string_pretty(self)?;
        let mut target_bookmarks_file = File::create(&config.target_bookmark_file)?;
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
                    .map(|bookmark_file| bookmark_file.source.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            target_bookmarks
        };

        Ok(bookmarks)
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

    pub fn diff(
        &mut self,
        source_bookmarks: &SourceBookmarks,
        config: &Config,
    ) -> Result<(), anyhow::Error> {
        let now = Utc::now();
        let bookmarks_to_add = self.filter_to_add(source_bookmarks);
        let bookmarks_to_remove = self.filter_to_remove(source_bookmarks);

        for url in &bookmarks_to_add {
            let bookmark = TargetBookmark::new(*url, now, None);
            self.add(&bookmark);
        }

        for bookmark in &bookmarks_to_remove {
            self.remove(bookmark);
        }

        if !bookmarks_to_add.is_empty() {
            self.write(config)?;
            info!("Added {} new bookmarks", bookmarks_to_add.len());
            trace!("Added new bookmarks: {bookmarks_to_add:#?}");
        }

        if !bookmarks_to_remove.is_empty() {
            self.write(config)?;
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
