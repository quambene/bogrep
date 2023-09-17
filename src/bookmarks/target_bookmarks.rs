use crate::{json, SourceBookmarks};
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct TargetBookmarks {
    pub bookmarks: Vec<TargetBookmark>,
}

impl TargetBookmarks {
    pub fn read(
        target_reader_writer: &mut (impl Read + Write),
    ) -> Result<TargetBookmarks, anyhow::Error> {
        let mut buf = Vec::new();
        target_reader_writer
            .read_to_end(&mut buf)
            .context("Can't read from `bookmarks.json` file:")?;
        let target_bookmarks = json::deserialize::<TargetBookmarks>(&buf)?;
        Ok(target_bookmarks)
    }

    pub fn write(
        &self,
        target_reader_writer: &mut (impl Read + Write),
    ) -> Result<(), anyhow::Error> {
        let bookmarks_json = json::serialize(&self)?;
        target_reader_writer
            .write_all(&bookmarks_json)
            .context("Can't write to `bookmarks.json` file")?;
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
    use crate::utils;
    use std::{io::Cursor, path::Path};

    #[test]
    fn test_read_target_bookmarks() {
        let bookmark_path = Path::new("test_data/target/bookmarks.json");
        let mut bookmark_file = utils::open_file(bookmark_path).unwrap();
        let res = TargetBookmarks::read(&mut bookmark_file);
        assert!(res.is_ok());

        let bookmarks = res.unwrap();
        assert_eq!(
            bookmarks.bookmarks,
            vec![
                TargetBookmark {
                    id: String::from("a87f7024-a7f5-4f9c-8a71-f64880b2f275"),
                    url: String::from("https://doc.rust-lang.org/book/title-page.html"),
                    last_imported: 1694989714351,
                    last_cached: None,
                },
                TargetBookmark {
                    id: String::from("511b1590-e6de-4989-bca4-96dc61730508"),
                    url: String::from("https://www.deepl.com/translator"),
                    last_imported: 1694989714351,
                    last_cached: None,
                }
            ]
        );
    }

    #[test]
    fn test_read_target_bookmarks_empty() {
        let bookmark_path = Path::new("test_data/target/bookmarks_empty.json");
        let mut bookmark_file = utils::open_file(bookmark_path).unwrap();
        let res = TargetBookmarks::read(&mut bookmark_file);
        assert!(res.is_ok());

        let bookmarks = res.unwrap();
        assert!(bookmarks.bookmarks.is_empty());
    }

    #[test]
    fn test_write_target_bookmarks() {
        let bookmarks = TargetBookmarks {
            bookmarks: vec![
                TargetBookmark {
                    id: String::from("a87f7024-a7f5-4f9c-8a71-f64880b2f275"),
                    url: String::from("https://doc.rust-lang.org/book/title-page.html"),
                    last_imported: 1694989714351,
                    last_cached: None,
                },
                TargetBookmark {
                    id: String::from("511b1590-e6de-4989-bca4-96dc61730508"),
                    url: String::from("https://www.deepl.com/translator"),
                    last_imported: 1694989714351,
                    last_cached: None,
                },
            ],
        };
        let mut cursor = Cursor::new(Vec::new());
        let res = TargetBookmarks::write(&bookmarks, &mut cursor);
        assert!(res.is_ok());

        let actual = cursor.into_inner();
        let expected_path = Path::new("test_data/target/bookmarks.json");
        let expected = utils::read_file(expected_path).unwrap();
        assert_eq!(
            String::from_utf8(actual).unwrap(),
            String::from_utf8(expected).unwrap()
        );
    }

    #[test]
    fn test_write_target_bookmarks_empty() {
        let bookmarks = TargetBookmarks::default();
        let mut cursor = Cursor::new(Vec::new());
        let res = TargetBookmarks::write(&bookmarks, &mut cursor);
        assert!(res.is_ok());

        let actual = cursor.into_inner();
        let expected_path = Path::new("test_data/target/bookmarks_empty.json");
        let expected = utils::read_file(expected_path).unwrap();
        assert_eq!(
            String::from_utf8(actual).unwrap(),
            String::from_utf8(expected).unwrap()
        );
    }
}
