use crate::SourceBookmarks;
use chrono::{DateTime, Utc};
use log::{info, trace};
use serde::{Deserialize, Serialize};
use std::slice;
use uuid::Uuid;

/// A standardized bookmark for internal bookkeeping that is created from the
/// [`SourceBookmarks`].
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

/// A wrapper for a collection of [`TargetBookmark`]s that is stored in the
/// `bookmarks.json` file.
#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct TargetBookmarks {
    pub bookmarks: Vec<TargetBookmark>,
}

impl TargetBookmarks {
    pub fn new(bookmarks: Vec<TargetBookmark>) -> Self {
        Self { bookmarks }
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

    pub fn is_empty(&self) -> bool {
        self.bookmarks.is_empty()
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
    pub fn update(
        &mut self,
        source_bookmarks: &SourceBookmarks,
    ) -> Result<(Vec<TargetBookmark>, Vec<TargetBookmark>), anyhow::Error> {
        if self.bookmarks.is_empty() {
            self.bookmarks = Self::from(source_bookmarks.clone()).bookmarks;
            return Ok((vec![], vec![]));
        }

        let now = Utc::now();
        let urls_to_add = self.filter_to_add(source_bookmarks);
        let bookmarks_to_remove = self.filter_to_remove(source_bookmarks);
        let mut bookmarks_to_add = vec![];

        for url in urls_to_add {
            let bookmark = TargetBookmark::new(url, now, None);
            self.add(&bookmark);
            bookmarks_to_add.push(bookmark);
        }

        for bookmark in &bookmarks_to_remove {
            self.remove(bookmark);
        }

        if !bookmarks_to_add.is_empty() {
            info!("Added {} new bookmarks", bookmarks_to_add.len());
            trace!(
                "Added new bookmarks: {:#?}",
                bookmarks_to_add.iter().map(|bookmark| &bookmark.url)
            );
        }

        if !bookmarks_to_remove.is_empty() {
            info!("Removed {} bookmarks", bookmarks_to_remove.len());
            trace!("Removed bookmarks: {bookmarks_to_remove:#?}");
        }

        if bookmarks_to_add.is_empty() && bookmarks_to_remove.is_empty() {
            info!("Bookmarks are already up to date");
        }

        Ok((bookmarks_to_add, bookmarks_to_remove))
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
    use crate::bookmark_reader::{ReadTarget, WriteTarget};
    use std::{collections::HashSet, io::Cursor};

    const EXPECTED_BOOKMARKS: &str = r#"{
    "bookmarks": [
        {
            "id": "a87f7024-a7f5-4f9c-8a71-f64880b2f275",
            "url": "https://doc.rust-lang.org/book/title-page.html",
            "last_imported": 1694989714351,
            "last_cached": null
        },
        {
            "id": "511b1590-e6de-4989-bca4-96dc61730508",
            "url": "https://www.deepl.com/translator",
            "last_imported": 1694989714351,
            "last_cached": null
        }
    ]
}"#;

    const EXPECTED_BOOKMARKS_EMPTY: &str = r#"{
    "bookmarks": []
}"#;

    #[test]
    fn test_update() {
        let now = Utc::now();
        let expected_bookmarks = HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]);
        let source_bookmarks = SourceBookmarks {
            bookmarks: expected_bookmarks.clone(),
        };
        let mut target_bookmarks = TargetBookmarks {
            bookmarks: vec![TargetBookmark::new(
                "https://www.deepl.com/translator",
                now,
                None,
            ), TargetBookmark::new(
                "https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/",
                now,
                None,
            )],
        };
        let res = target_bookmarks.update(&source_bookmarks);
        assert!(res.is_ok());
        assert_eq!(
            target_bookmarks
                .bookmarks
                .iter()
                .map(|bookmark| bookmark.url.clone())
                .collect::<HashSet<_>>(),
            expected_bookmarks,
        );
    }

    #[test]
    fn test_read_target_bookmarks() {
        let expected_bookmarks = EXPECTED_BOOKMARKS.as_bytes().to_vec();
        let mut target_bookmarks = TargetBookmarks::default();
        let mut target_reader = Cursor::new(expected_bookmarks);

        let res = target_reader.read(&mut target_bookmarks);
        assert!(res.is_ok());
        assert_eq!(
            target_bookmarks.bookmarks,
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
        let expected_bookmarks = EXPECTED_BOOKMARKS_EMPTY.as_bytes().to_vec();
        let mut target_bookmarks = TargetBookmarks::default();
        let mut target_reader = Cursor::new(expected_bookmarks);

        let res = target_reader.read(&mut target_bookmarks);
        assert!(res.is_ok());
        assert!(target_bookmarks.bookmarks.is_empty());
    }

    #[test]
    fn test_write_target_bookmarks() {
        let target_bookmarks = TargetBookmarks::new(vec![
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
        ]);
        let mut target_reader = Cursor::new(Vec::new());
        let res = target_reader.write(&target_bookmarks);
        assert!(res.is_ok());

        let actual = target_reader.into_inner();
        assert_eq!(String::from_utf8(actual).unwrap(), EXPECTED_BOOKMARKS);
    }

    #[test]
    fn test_write_target_bookmarks_empty() {
        let bookmarks = TargetBookmarks::default();
        let mut target_writer = Cursor::new(Vec::new());
        let res = target_writer.write(&bookmarks);
        assert!(res.is_ok());

        let actual = target_writer.into_inner();
        assert_eq!(String::from_utf8(actual).unwrap(), EXPECTED_BOOKMARKS_EMPTY);
    }
}
