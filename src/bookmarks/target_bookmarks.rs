use super::{Action, JsonBookmark, Underlying};
use crate::{cache::CacheMode, SourceBookmark, SourceBookmarks, SourceType, UnderlyingType};
use chrono::{DateTime, Utc};
use log::{debug, trace};
use std::collections::{
    hash_map::{Entry, IntoIter, IntoValues, Iter, IterMut, Keys, Values, ValuesMut},
    HashMap, HashSet,
};
use url::Url;
use uuid::Uuid;

/// A standardized bookmark for internal bookkeeping that is created from the
/// [`SourceBookmarks`].
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TargetBookmark {
    pub id: String,
    pub url: Url,
    pub underlying: Option<Underlying>,
    pub last_imported: i64,
    pub last_cached: Option<i64>,
    pub sources: HashSet<SourceType>,
    pub cache_modes: HashSet<CacheMode>,
    pub action: Action,
}

impl TargetBookmark {
    pub fn new(
        url: Url,
        underlying: Option<Underlying>,
        last_imported: DateTime<Utc>,
        last_cached: Option<DateTime<Utc>>,
        sources: HashSet<SourceType>,
        cache_modes: HashSet<CacheMode>,
        action: Action,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            url,
            underlying,
            last_imported: last_imported.timestamp_millis(),
            last_cached: last_cached.map(|timestamp| timestamp.timestamp_millis()),
            sources,
            cache_modes,
            action,
        }
    }

    pub fn set_action(&mut self, action: Action) {
        self.action = action;
    }

    pub fn set_source(&mut self, source: SourceType) {
        self.sources.insert(source);
    }
}

impl TryFrom<JsonBookmark> for TargetBookmark {
    type Error = anyhow::Error;

    fn try_from(value: JsonBookmark) -> Result<Self, anyhow::Error> {
        let underlying_source = value
            .sources
            .iter()
            .find(|source| matches!(source, SourceType::Underlying(_)));

        let underlying = match underlying_source {
            Some(SourceType::Underlying(url)) => {
                if url.domain() == Some("https://news.ycombinator.com/") {
                    Some(Underlying::new(url, UnderlyingType::HackerNews))
                } else {
                    None
                }
            }
            _ => None,
        };

        Ok(Self {
            id: value.id,
            url: Url::parse(&value.url)?,
            underlying,
            last_imported: value.last_imported,
            last_cached: value.last_cached,
            sources: value.sources,
            cache_modes: value.cache_modes,
            action: Action::None,
        })
    }
}

/// A wrapper for a collection of [`TargetBookmark`]s that is stored in the
/// `bookmarks.json` file.
#[derive(Debug, PartialEq, Eq, Default)]
pub struct TargetBookmarks(HashMap<Url, TargetBookmark>);

impl TargetBookmarks {
    pub fn new(bookmarks: HashMap<Url, TargetBookmark>) -> Self {
        Self(bookmarks)
    }

    pub fn inner(self) -> HashMap<Url, TargetBookmark> {
        self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, url: &Url) -> Option<&TargetBookmark> {
        self.0.get(url)
    }

    pub fn get_mut(&mut self, url: &Url) -> Option<&mut TargetBookmark> {
        self.0.get_mut(url)
    }

    pub fn keys(&self) -> Keys<Url, TargetBookmark> {
        self.0.keys()
    }

    pub fn values(&self) -> Values<Url, TargetBookmark> {
        self.0.values()
    }

    pub fn values_mut(&mut self) -> ValuesMut<Url, TargetBookmark> {
        self.0.values_mut()
    }

    pub fn into_values(self) -> IntoValues<Url, TargetBookmark> {
        self.0.into_values()
    }

    pub fn contains_key(&self, url: &Url) -> bool {
        self.0.contains_key(url)
    }

    pub fn iter(&self) -> Iter<Url, TargetBookmark> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<Url, TargetBookmark> {
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

                // We are keeping the existing id and url, but overwriting all other fields.
                target_bookmark.last_imported = bookmark.last_imported;
                target_bookmark.last_cached = bookmark.last_cached;
                target_bookmark.sources = bookmark.sources;
                target_bookmark.cache_modes = bookmark.cache_modes;
                target_bookmark.action = bookmark.action;
            }
            Entry::Vacant(entry) => {
                entry.insert(bookmark);
            }
        }
    }

    pub fn remove(&mut self, url: &Url) -> Option<TargetBookmark> {
        self.0.remove(url)
    }

    /// Clean up bookmarks which are marked by [`Action::Remove`].
    pub fn clean_up(&mut self) {
        let urls_to_remove = self
            .values()
            .filter_map(|bookmark| {
                if bookmark.action == Action::Remove {
                    Some(bookmark.url.to_owned())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for url in urls_to_remove {
            self.remove(&url);
        }
    }

    pub fn ignore_urls(&mut self, ignored_urls: &[Url]) {
        for url in ignored_urls {
            if let Some(target_bookmark) = self.get_mut(url) {
                target_bookmark.set_action(Action::Remove)
            }
        }
    }

    pub fn filter_to_add<'a>(
        &self,
        source_bookmarks: &'a SourceBookmarks,
    ) -> Vec<&'a SourceBookmark> {
        // TODO: refactor unwrap
        source_bookmarks
            .iter()
            .filter(|(url, _)| !self.0.contains_key(&Url::parse(url).unwrap()))
            .map(|(_, bookmark)| bookmark)
            .collect()
    }

    pub fn filter_to_remove<'a>(
        &'a mut self,
        source_bookmarks: &SourceBookmarks,
    ) -> Vec<&'a mut TargetBookmark> {
        self.0
            .iter_mut()
            .filter(|(url, _)| !source_bookmarks.contains_key(&url.to_string()))
            .map(|(_, target_bookmark)| target_bookmark)
            .collect()
    }

    /// Update target bookmarks.
    ///
    /// Determine the difference between source and target bookmarks and update
    /// the `action` of the target bookmarks.
    pub fn update(&mut self, source_bookmarks: &SourceBookmarks) -> Result<(), anyhow::Error> {
        let now = Utc::now();

        let bookmarks_to_add = self.filter_to_add(source_bookmarks);
        let urls_to_add = bookmarks_to_add
            .iter()
            .map(|bookmark| bookmark.url.to_owned())
            .collect::<Vec<_>>();

        for bookmark in bookmarks_to_add {
            let url = Url::parse(&bookmark.url)?;
            let target_bookmark = TargetBookmark::new(
                url,
                None,
                now,
                None,
                bookmark.sources.to_owned(),
                HashSet::new(),
                Action::FetchAndAdd,
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
    type Item = (Url, TargetBookmark);
    type IntoIter = IntoIter<Url, TargetBookmark>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl TryFrom<SourceBookmarks> for TargetBookmarks {
    type Error = anyhow::Error;

    fn try_from(source_bookmarks: SourceBookmarks) -> Result<Self, Self::Error> {
        let now = Utc::now();
        let mut target_bookmarks = TargetBookmarks::default();

        for source_bookmark in source_bookmarks.into_iter() {
            let url = Url::parse(&source_bookmark.0)?;
            let target_bookmark = TargetBookmark::new(
                url,
                None,
                now,
                None,
                source_bookmark.1.sources,
                HashSet::new(),
                Action::None,
            );
            target_bookmarks.insert(target_bookmark)
        }

        Ok(target_bookmarks)
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
            "url": "https://url1.com/",
            "last_imported": 1694989714351,
            "last_cached": null,
            "sources": [],
            "cache_modes": []
        },
        {
            "id": "511b1590-e6de-4989-bca4-96dc61730508",
            "url": "https://url2.com/",
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

        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let url3 = Url::parse("https://url3.com").unwrap();
        let url4 = Url::parse("https://url4.com").unwrap();
        let url5 = Url::parse("https://url5.com").unwrap();

        let source_bookmarks = SourceBookmarks::new(HashMap::from_iter([
            (
                url1.to_string(),
                SourceBookmarkBuilder::new(&url1.to_string()).build(),
            ),
            (
                url2.to_string(),
                SourceBookmarkBuilder::new(&url2.to_string()).build(),
            ),
            (
                url3.to_string(),
                SourceBookmarkBuilder::new(&url3.to_string()).build(),
            ),
            (
                url4.to_string(),
                SourceBookmarkBuilder::new(&url4.to_string()).build(),
            ),
        ]));
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                url1.clone(),
                TargetBookmark::new(
                    url1.clone(),
                    None,
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    Action::None,
                ),
            ),
            (
                url3.clone(),
                TargetBookmark::new(
                    url3.clone(),
                    None,
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    Action::None,
                ),
            ),
            (
                url5.clone(),
                TargetBookmark::new(
                    url5.clone(),
                    None,
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
        assert_eq!(target_bookmarks.get(&url1).unwrap().action, Action::None);
        assert_eq!(
            target_bookmarks.get(&url2).unwrap().action,
            Action::FetchAndAdd
        );
        assert_eq!(target_bookmarks.get(&url3).unwrap().action, Action::None);
        assert_eq!(
            target_bookmarks.get(&url4).unwrap().action,
            Action::FetchAndAdd
        );
        assert_eq!(target_bookmarks.get(&url5).unwrap().action, Action::Remove);
    }

    #[test]
    fn test_ignore_urls() {
        let now = Utc::now();
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let url3 = Url::parse("https://url3.com").unwrap();
        let ignored_urls = vec![url1.clone(), url3.clone()];
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                url1.clone(),
                TargetBookmark::new(
                    url1.clone(),
                    None,
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    Action::None,
                ),
            ),
            (
                url2.clone(),
                TargetBookmark::new(
                    url2.clone(),
                    None,
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    Action::None,
                ),
            ),
            (
                url3.clone(),
                TargetBookmark::new(
                    url3.clone(),
                    None,
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    Action::None,
                ),
            ),
        ]));

        target_bookmarks.ignore_urls(&ignored_urls);
        assert!(target_bookmarks.get(&url1).unwrap().action == Action::Remove);
        assert!(target_bookmarks.get(&url2).unwrap().action == Action::None);
        assert!(target_bookmarks.get(&url3).unwrap().action == Action::Remove);
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
                    Url::parse("https://url1.com").unwrap(),
                    TargetBookmark {
                        id: String::from("a87f7024-a7f5-4f9c-8a71-f64880b2f275"),
                        url: Url::parse("https://url1.com").unwrap(),
                        underlying: None,
                        last_imported: 1694989714351,
                        last_cached: None,
                        sources: HashSet::new(),
                        cache_modes: HashSet::new(),
                        action: Action::None,
                    }
                ),
                (
                    Url::parse("https://url2.com").unwrap(),
                    TargetBookmark {
                        id: String::from("511b1590-e6de-4989-bca4-96dc61730508"),
                        url: Url::parse("https://url2.com").unwrap(),
                        underlying: None,
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
                Url::parse("https://url1.com").unwrap(),
                TargetBookmark {
                    id: String::from("a87f7024-a7f5-4f9c-8a71-f64880b2f275"),
                    url: Url::parse("https://url1.com").unwrap(),
                    underlying: None,
                    last_imported: 1694989714351,
                    last_cached: None,
                    sources: HashSet::new(),
                    cache_modes: HashSet::new(),
                    action: Action::None,
                },
            ),
            (
                Url::parse("https://url2.com").unwrap(),
                TargetBookmark {
                    id: String::from("511b1590-e6de-4989-bca4-96dc61730508"),
                    url: Url::parse("https://url2.com").unwrap(),
                    underlying: None,
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
