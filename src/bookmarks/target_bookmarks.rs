use super::Action;
use crate::{cache::CacheMode, SourceBookmarks, SourceType};
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
    pub action: Option<Action>,
}

impl TargetBookmark {
    pub fn new(
        url: impl Into<String>,
        last_imported: DateTime<Utc>,
        last_cached: Option<DateTime<Utc>>,
        sources: HashSet<SourceType>,
        cache_modes: HashSet<CacheMode>,
        action: Option<Action>,
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

    pub fn filter_to_add<'a>(&self, source_bookmarks: &'a SourceBookmarks) -> Vec<&'a str> {
        source_bookmarks
            .keys()
            .filter(|url| !self.0.contains_key(*url))
            .map(|url| url.as_str())
            .collect()
    }

    pub fn filter_to_remove(&self, source_bookmarks: &SourceBookmarks) -> Vec<String> {
        self.0
            .keys()
            .filter(|url| !source_bookmarks.contains_key(url))
            .cloned()
            .collect()
    }

    pub fn filter_to_remove_mut<'a>(
        &'a mut self,
        source_bookmarks: &SourceBookmarks,
    ) -> Vec<&'a mut TargetBookmark> {
        self.0
            .iter_mut()
            .filter(|(url, _)| !source_bookmarks.contains_key(url))
            .map(|(_, bookmark)| bookmark)
            .collect()
    }

    /// Update  on target bookmarks.
    ///
    /// Determine the difference between source and target bookmarks and update
    /// the target bookmarks.
    pub fn update(&mut self, source_bookmarks: &SourceBookmarks) -> Result<(), anyhow::Error> {
        let now = Utc::now();

        let urls_to_remove = self.filter_to_remove(source_bookmarks);
        trace!("Mark as removed: {urls_to_remove:#?}");

        let urls_to_add = self.filter_to_add(source_bookmarks);
        trace!("Mark as added: {urls_to_add:#?}");

        for url in &urls_to_remove {
            self.remove(url);
        }

        for url in &urls_to_add {
            if let Some(sources) = source_bookmarks.get(&url) {
                let target_bookmark = TargetBookmark::new(
                    url.to_owned(),
                    now,
                    None,
                    sources.to_owned(),
                    HashSet::new(),
                    Some(Action::Add),
                );
                self.insert(target_bookmark);
            }
        }

        if !urls_to_add.is_empty() {
            println!("Added {} new bookmarks", urls_to_add.len());
        }

        if urls_to_remove.is_empty() {
            println!("Removed {} bookmarks", urls_to_remove.len());
        }

        if urls_to_add.is_empty() && urls_to_remove.is_empty() {
            println!("Bookmarks are already up to date");
        }

        Ok(())
    }

    /// Update `action` on target bookmarks.
    ///
    /// Determine the difference between source and target bookmarks and update
    /// the target bookmarks.
    pub fn update_actions(
        &mut self,
        source_bookmarks: &SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        let now = Utc::now();

        let bookmarks_to_remove = self.filter_to_remove_mut(source_bookmarks);
        let len_to_remove = bookmarks_to_remove.len();
        trace!("Mark as removed: {bookmarks_to_remove:#?}");

        for bookmark in bookmarks_to_remove {
            bookmark.action = Some(Action::Remove);
        }

        let urls_to_add = self.filter_to_add(source_bookmarks);
        trace!("Mark as added: {urls_to_add:#?}");

        for url in urls_to_add.iter() {
            if let Some(sources) = source_bookmarks.get(&url) {
                let target_bookmark = TargetBookmark::new(
                    url.to_owned(),
                    now,
                    None,
                    sources.to_owned(),
                    HashSet::new(),
                    Some(Action::Add),
                );
                self.insert(target_bookmark);
            }
        }

        if !urls_to_add.is_empty() {
            println!("Added {} new bookmarks", urls_to_add.len());
        }

        if len_to_remove != 0 {
            println!("Removed {len_to_remove} bookmarks");
        }

        if urls_to_add.is_empty() && len_to_remove == 0 {
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

impl FromIterator<TargetBookmark> for TargetBookmarks {
    fn from_iter<I: IntoIterator<Item = TargetBookmark>>(iter: I) -> Self {
        let mut bookmarks = HashMap::new();

        for bookmark in iter {
            bookmarks.insert(bookmark.url.clone(), bookmark);
        }

        TargetBookmarks(bookmarks)
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
                source_bookmark.1,
                HashSet::new(),
                None,
            );
            target_bookmarks.insert(target_bookmark)
        }

        target_bookmarks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmark_reader::{ReadTarget, WriteTarget};
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
            "cache_modes": [],
            "action": null
        },
        {
            "id": "511b1590-e6de-4989-bca4-96dc61730508",
            "url": "https://www.deepl.com/translator",
            "last_imported": 1694989714351,
            "last_cached": null,
            "sources": [],
            "cache_modes": [],
            "action": null
        }
    ]
}"#;

    const EXPECTED_BOOKMARKS_EMPTY: &str = r#"{
    "bookmarks": []
}"#;

    #[test]
    fn test_update() {
        let now = Utc::now();
        let source_bookmarks = SourceBookmarks::new(HashMap::from_iter([
            ("https://url1".to_owned(), HashSet::new()),
            ("https://url2".to_owned(), HashSet::new()),
            ("https://url3".to_owned(), HashSet::new()),
            ("https://url4".to_owned(), HashSet::new()),
        ]));
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                "https://url1".to_owned(),
                TargetBookmark::new(
                    "https://url1",
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    None,
                ),
            ),
            (
                "https://url3".to_owned(),
                TargetBookmark::new(
                    "https://url3",
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    None,
                ),
            ),
            (
                "https://url5".to_owned(),
                TargetBookmark::new(
                    "https://url5",
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    None,
                ),
            ),
        ]));
        let res = target_bookmarks.update(&source_bookmarks);
        assert!(res.is_ok());
        assert_eq!(
            target_bookmarks.keys().collect::<HashSet<_>>(),
            source_bookmarks.keys().collect(),
        );
    }

    #[test]
    fn test_update_actions() {
        let now = Utc::now();
        let source_bookmarks = SourceBookmarks::new(HashMap::from_iter([
            ("https://url1".to_owned(), HashSet::new()),
            ("https://url2".to_owned(), HashSet::new()),
            ("https://url3".to_owned(), HashSet::new()),
            ("https://url4".to_owned(), HashSet::new()),
        ]));
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                "https://url1".to_owned(),
                TargetBookmark::new(
                    "https://url1",
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    None,
                ),
            ),
            (
                "https://url3".to_owned(),
                TargetBookmark::new(
                    "https://url3",
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    None,
                ),
            ),
            (
                "https://url5".to_owned(),
                TargetBookmark::new(
                    "https://url5",
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    None,
                ),
            ),
        ]));
        let res = target_bookmarks.update_actions(&source_bookmarks);
        assert!(res.is_ok());
        assert_eq!(target_bookmarks.get("https://url1").unwrap().action, None);
        assert_eq!(
            target_bookmarks.get("https://url2").unwrap().action,
            Some(Action::Add)
        );
        assert_eq!(target_bookmarks.get("https://url3").unwrap().action, None);
        assert_eq!(
            target_bookmarks.get("https://url4").unwrap().action,
            Some(Action::Add)
        );
        assert_eq!(
            target_bookmarks.get("https://url5").unwrap().action,
            Some(Action::Remove)
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
                        action: None,
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
                        action: None,
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
                    action: None,
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
                    action: None,
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
