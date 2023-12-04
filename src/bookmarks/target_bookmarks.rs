use super::Action;
use crate::{cache::CacheMode, SourceBookmark, SourceBookmarks, SourceType};
use chrono::{DateTime, Utc};
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use std::collections::{
    hash_map::{Entry, IntoIter, IntoValues, Iter, IterMut, Keys, Values, ValuesMut},
    HashMap, HashSet,
};
use uuid::Uuid;

/// A standardized bookmark for internal bookkeeping that is created from the
/// [`SourceBookmarks`].
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct TargetBookmark {
    pub id: String,
    pub url: String,
    pub last_imported: i64,
    pub last_cached: Option<i64>,
    pub sources: HashSet<SourceType>,
    pub cache_modes: HashSet<CacheMode>,
    pub action: Action,
}

impl TargetBookmark {
    pub fn new(
        url: impl Into<String>,
        last_imported: DateTime<Utc>,
        last_cached: Option<DateTime<Utc>>,
        sources: HashSet<SourceType>,
        cache_modes: HashSet<CacheMode>,
        action: Action,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            url: url.into(),
            last_imported: last_imported.timestamp_millis(),
            last_cached: last_cached.map(|timestamp| timestamp.timestamp_millis()),
            sources,
            cache_modes,
            action,
        }
    }
}

/// A wrapper for a collection of [`TargetBookmark`]s that is stored in the
/// `bookmarks.json` file.
#[derive(Debug, PartialEq, Eq, Default)]
pub struct TargetBookmarks(HashMap<String, TargetBookmark>);

impl TargetBookmarks {
    pub fn new(bookmarks: HashMap<String, TargetBookmark>) -> Self {
        Self(bookmarks)
    }

    pub fn inner(self) -> HashMap<String, TargetBookmark> {
        self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, url: &str) -> Option<&TargetBookmark> {
        self.0.get(url)
    }

    pub fn get_mut(&mut self, url: &str) -> Option<&mut TargetBookmark> {
        self.0.get_mut(url)
    }

    pub fn keys(&self) -> Keys<String, TargetBookmark> {
        self.0.keys()
    }

    pub fn values(&self) -> Values<String, TargetBookmark> {
        self.0.values()
    }

    pub fn values_mut(&mut self) -> ValuesMut<String, TargetBookmark> {
        self.0.values_mut()
    }

    pub fn into_values(self) -> IntoValues<String, TargetBookmark> {
        self.0.into_values()
    }

    pub fn contains_key(&self, url: &str) -> bool {
        self.0.contains_key(url)
    }

    pub fn iter(&self) -> Iter<String, TargetBookmark> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<String, TargetBookmark> {
        self.0.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// If the cache was removed, reset the cache values in the target
    /// bookmarks.
    pub fn reset_cache_status(&mut self) {
        debug!("Reset cache status");
        for bookmark in self.values_mut() {
            bookmark.last_cached = None;
            bookmark.cache_modes.clear();
        }
    }

    pub fn set_action(&mut self, action: &Action) {
        debug!("Set action to {action:#?}");

        for bookmark in self.values_mut() {
            bookmark.action = action.clone()
        }
    }

    pub fn insert(&mut self, bookmark: TargetBookmark) {
        let url = &bookmark.url;
        let entry = self.0.entry(url.clone());

        match entry {
            Entry::Occupied(entry) => {
                let url = entry.key().clone();
                let target_bookmark = entry.into_mut();
                debug!("Overwrite duplicate target bookmark: {}", url);
                *target_bookmark = bookmark;
            }
            Entry::Vacant(entry) => {
                entry.insert(bookmark);
            }
        }
    }

    pub fn remove(&mut self, url: &str) -> Option<TargetBookmark> {
        self.0.remove(url)
    }

    pub fn filter_to_add<'a>(
        &self,
        source_bookmarks: &'a SourceBookmarks,
    ) -> Vec<&'a SourceBookmark> {
        source_bookmarks
            .iter()
            .filter(|(url, _)| !self.0.contains_key(*url))
            .map(|(_, bookmark)| bookmark)
            .collect()
    }

    pub fn filter_to_remove<'a>(
        &'a mut self,
        source_bookmarks: &SourceBookmarks,
    ) -> Vec<&'a mut TargetBookmark> {
        self.0
            .iter_mut()
            .filter(|(url, _)| !source_bookmarks.contains_key(url))
            .map(|(_, target_bookmark)| target_bookmark)
            .collect()
    }

    /// Update target bookmarks.
    ///
    /// Determine the difference between source and target bookmarks and update
    /// the target bookmarks.
    pub fn update(&mut self, source_bookmarks: &SourceBookmarks) -> Result<(), anyhow::Error> {
        let now = Utc::now();

        let bookmarks_to_add = self.filter_to_add(source_bookmarks);
        let urls_to_add = bookmarks_to_add
            .iter()
            .map(|bookmark| bookmark.url.to_owned())
            .collect::<Vec<_>>();

        for bookmark in bookmarks_to_add {
            let target_bookmark = TargetBookmark::new(
                bookmark.url.to_owned(),
                now,
                None,
                bookmark.sources.to_owned(),
                HashSet::new(),
                Action::Add,
            );
            self.insert(target_bookmark);
        }

        let bookmarks_to_remove = self.filter_to_remove(source_bookmarks);
        let urls_to_remove = bookmarks_to_remove
            .iter()
            .map(|bookmark| bookmark.url.to_owned())
            .collect::<Vec<_>>();

        for bookmark in bookmarks_to_remove {
            bookmark.action = Action::Remove;
        }

        if !urls_to_add.is_empty() {
            println!("Added {} new bookmarks", urls_to_add.len());
            trace!("Added new bookmarks: {urls_to_add:#?}",);
        }

        if !urls_to_remove.is_empty() {
            println!("Removed {} bookmarks", urls_to_remove.len());
            trace!("Removed bookmarks: {urls_to_remove:#?}",);
        }

        if urls_to_add.is_empty() && urls_to_remove.is_empty() {
            println!("Bookmarks are already up to date");
        }

        Ok(())
    }
}

impl IntoIterator for TargetBookmarks {
    type Item = (String, TargetBookmark);
    type IntoIter = IntoIter<String, TargetBookmark>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<SourceBookmarks> for TargetBookmarks {
    fn from(source_bookmarks: SourceBookmarks) -> Self {
        let now = Utc::now();
        let mut target_bookmarks = TargetBookmarks::default();

        for source_bookmark in source_bookmarks.into_iter() {
            let target_bookmark = TargetBookmark::new(
                source_bookmark.0,
                now,
                None,
                source_bookmark.1.sources,
                HashSet::new(),
                Action::None,
            );
            target_bookmarks.insert(target_bookmark)
        }

        target_bookmarks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bookmark_reader::{ReadTarget, WriteTarget},
        bookmarks::SourceBookmarkBuilder,
    };
    use std::{
        collections::{HashMap, HashSet},
        io::Cursor,
    };

    const EXPECTED_BOOKMARKS: &str = r#"{
    "bookmarks": [
        {
            "id": "a87f7024-a7f5-4f9c-8a71-f64880b2f275",
            "url": "https://doc.rust-lang.org/book/title-page.html",
            "last_imported": 1694989714351,
            "last_cached": null,
            "sources": [],
            "cache_modes": []
        },
        {
            "id": "511b1590-e6de-4989-bca4-96dc61730508",
            "url": "https://www.deepl.com/translator",
            "last_imported": 1694989714351,
            "last_cached": null,
            "sources": [],
            "cache_modes": []
        }
    ]
}"#;

    const EXPECTED_BOOKMARKS_EMPTY: &str = r#"{
    "bookmarks": []
}"#;

    #[test]
    fn test_update() {
        let now = Utc::now();

        let url1 = "https://www.deepl.com/translator";
        let url2 =
            "https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/";
        let url3 = "https://en.wikipedia.org/wiki/Design_Patterns";
        let url4 = "https://doc.rust-lang.org/book/title-page.html";

        let expected_bookmarks = HashMap::from_iter([
            (url1.to_owned(), SourceBookmarkBuilder::new(url1).build()),
            (url2.to_owned(), SourceBookmarkBuilder::new(url2).build()),
            (url3.to_owned(), SourceBookmarkBuilder::new(url3).build()),
            (url4.to_owned(), SourceBookmarkBuilder::new(url4).build()),
        ]);
        let source_bookmarks = SourceBookmarks::new(expected_bookmarks.clone());
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                url1.to_owned(),
                TargetBookmark::new(
                    url1,
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    Action::None,
                ),
            ),
            (
                url2.to_owned(),
                TargetBookmark::new(
                    url2,
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    Action::None,
                ),
            ),
        ]));
        let res = target_bookmarks.update(&source_bookmarks);
        assert!(res.is_ok());
        assert_eq!(
            target_bookmarks.keys().collect::<HashSet<_>>(),
            expected_bookmarks.keys().collect(),
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
            target_bookmarks,
            TargetBookmarks::new(HashMap::from_iter([
                (
                    String::from("https://doc.rust-lang.org/book/title-page.html"),
                    TargetBookmark {
                        id: String::from("a87f7024-a7f5-4f9c-8a71-f64880b2f275"),
                        url: String::from("https://doc.rust-lang.org/book/title-page.html"),
                        last_imported: 1694989714351,
                        last_cached: None,
                        sources: HashSet::new(),
                        cache_modes: HashSet::new(),
                        action: Action::None,
                    }
                ),
                (
                    String::from("https://www.deepl.com/translator"),
                    TargetBookmark {
                        id: String::from("511b1590-e6de-4989-bca4-96dc61730508"),
                        url: String::from("https://www.deepl.com/translator"),
                        last_imported: 1694989714351,
                        last_cached: None,
                        sources: HashSet::new(),
                        cache_modes: HashSet::new(),
                        action: Action::None,
                    }
                )
            ]))
        );
    }

    #[test]
    fn test_read_target_bookmarks_empty() {
        let expected_bookmarks = EXPECTED_BOOKMARKS_EMPTY.as_bytes().to_vec();
        let mut target_bookmarks = TargetBookmarks::default();
        let mut target_reader = Cursor::new(expected_bookmarks);

        let res = target_reader.read(&mut target_bookmarks);
        assert!(res.is_ok());
        assert!(target_bookmarks.is_empty());
    }

    #[test]
    fn test_write_target_bookmarks() {
        let target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                String::from("https://doc.rust-lang.org/book/title-page.html"),
                TargetBookmark {
                    id: String::from("a87f7024-a7f5-4f9c-8a71-f64880b2f275"),
                    url: String::from("https://doc.rust-lang.org/book/title-page.html"),
                    last_imported: 1694989714351,
                    last_cached: None,
                    sources: HashSet::new(),
                    cache_modes: HashSet::new(),
                    action: Action::None,
                },
            ),
            (
                String::from("https://www.deepl.com/translator"),
                TargetBookmark {
                    id: String::from("511b1590-e6de-4989-bca4-96dc61730508"),
                    url: String::from("https://www.deepl.com/translator"),
                    last_imported: 1694989714351,
                    last_cached: None,
                    sources: HashSet::new(),
                    cache_modes: HashSet::new(),
                    action: Action::None,
                },
            ),
        ]));
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
