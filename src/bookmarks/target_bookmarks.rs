use super::{Action, JsonBookmark, Status};
use crate::{cache::CacheMode, SourceBookmarks, SourceType, UnderlyingType};
use chrono::{DateTime, Utc};
use log::debug;
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
    /// Unique id of the bookmark used for caching.
    pub id: String,
    /// The url of the bookmark.
    pub url: Url,
    /// The url of the underlying for supported `UnderlyingType`s.
    pub underlying_url: Option<Url>,
    /// The type of the underlying.
    pub underlying_type: UnderlyingType,
    /// The timestamp in milliseconds when the bookmark was imported from the source.
    pub last_imported: i64,
    /// The timestamp in milliseconds when the bookmark was added to the cache.
    pub last_cached: Option<i64>,
    /// The source or sources this bookmark was imported from.
    pub sources: HashSet<SourceType>,
    /// The file format for the cached bookmark.
    pub cache_modes: HashSet<CacheMode>,
    /// The status of an imported bookmark.
    pub status: Status,
    /// The action performed when processing [`TargetBookmark`] in
    /// `BookmarkProcessor`.
    pub action: Action,
}

impl TargetBookmark {
    pub fn builder(url: Url, last_imported: DateTime<Utc>) -> TargetBookmarkBuilder {
        TargetBookmarkBuilder::new(url, last_imported)
    }

    pub fn builder_with_id(
        id: String,
        url: Url,
        last_imported: DateTime<Utc>,
    ) -> TargetBookmarkBuilder {
        TargetBookmarkBuilder::new_with_id(id, url, last_imported)
    }

    pub fn new(url: Url, last_imported: DateTime<Utc>) -> Self {
        let underlying_type = UnderlyingType::from(&url);

        Self {
            id: Uuid::new_v4().to_string(),
            url,
            underlying_url: None,
            underlying_type,
            last_imported: last_imported.timestamp_millis(),
            last_cached: None,
            sources: HashSet::new(),
            cache_modes: HashSet::new(),
            status: Status::None,
            action: Action::None,
        }
    }

    pub fn id(&self) -> &str {
        self.id.as_ref()
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn underlying_url(&self) -> Option<&Url> {
        self.underlying_url.as_ref()
    }

    pub fn underlying_type(&self) -> &UnderlyingType {
        &self.underlying_type
    }

    pub fn last_imported(&self) -> i64 {
        self.last_imported
    }

    pub fn last_cached(&self) -> Option<i64> {
        self.last_cached
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn action(&self) -> &Action {
        &self.action
    }

    pub fn sources(&self) -> &HashSet<SourceType> {
        &self.sources
    }

    pub fn cache_modes(&self) -> &HashSet<CacheMode> {
        &self.cache_modes
    }

    pub fn set_url(&mut self, url: Url) {
        self.url = url;
    }

    pub fn set_underlying_url(&mut self, underlying_url: Url) {
        self.underlying_url = Some(underlying_url);
    }

    pub fn set_last_imported(&mut self, last_imported: DateTime<Utc>) {
        self.last_imported = last_imported.timestamp_millis();
    }

    pub fn set_last_cached(&mut self, last_cached: DateTime<Utc>) {
        self.last_cached = Some(last_cached.timestamp_millis());
    }

    pub fn unset_last_cached(&mut self) {
        self.last_cached = None;
    }

    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    pub fn set_action(&mut self, action: Action) {
        self.action = action;
    }

    pub fn add_source(&mut self, source: SourceType) {
        self.sources.insert(source);
    }

    pub fn add_cache_mode(&mut self, cache_mode: CacheMode) {
        self.cache_modes.insert(cache_mode);
    }

    pub fn remove_cache_mode(&mut self, cache_mode: &CacheMode) {
        self.cache_modes.remove(cache_mode);
    }

    pub fn clear_cache_mode(&mut self) {
        self.cache_modes.clear();
    }
}

pub struct TargetBookmarkBuilder {
    id: String,
    url: Url,
    underlying_url: Option<Url>,
    last_imported: DateTime<Utc>,
    last_cached: Option<DateTime<Utc>>,
    sources: HashSet<SourceType>,
    cache_modes: HashSet<CacheMode>,
    status: Status,
    action: Action,
}

impl TargetBookmarkBuilder {
    pub fn new(url: Url, last_imported: DateTime<Utc>) -> TargetBookmarkBuilder {
        TargetBookmarkBuilder {
            id: Uuid::new_v4().to_string(),
            url,
            underlying_url: None,
            last_imported,
            last_cached: None,
            sources: HashSet::new(),
            cache_modes: HashSet::new(),
            status: Status::None,
            action: Action::None,
        }
    }

    pub fn new_with_id(
        id: String,
        url: Url,
        last_imported: DateTime<Utc>,
    ) -> TargetBookmarkBuilder {
        TargetBookmarkBuilder {
            id,
            url,
            underlying_url: None,
            last_imported,
            last_cached: None,
            sources: HashSet::new(),
            cache_modes: HashSet::new(),
            status: Status::None,
            action: Action::None,
        }
    }

    pub fn with_status(mut self, status: Status) -> TargetBookmarkBuilder {
        self.status = status;
        self
    }

    pub fn with_action(mut self, action: Action) -> TargetBookmarkBuilder {
        self.action = action;
        self
    }

    pub fn with_sources(mut self, sources: HashSet<SourceType>) -> TargetBookmarkBuilder {
        self.sources = sources;
        self
    }

    pub fn add_source(mut self, source: SourceType) -> TargetBookmarkBuilder {
        self.sources.insert(source);
        self
    }

    pub fn add_cache_mode(mut self, cache_mode: CacheMode) -> TargetBookmarkBuilder {
        self.cache_modes.insert(cache_mode);
        self
    }

    pub fn build(self) -> TargetBookmark {
        let underlying_type = UnderlyingType::from(&self.url);

        TargetBookmark {
            id: self.id,
            url: self.url,
            underlying_url: self.underlying_url,
            underlying_type,
            last_imported: self.last_imported.timestamp_millis(),
            last_cached: self
                .last_cached
                .map(|timestamp| timestamp.timestamp_millis()),
            sources: self.sources,
            cache_modes: self.cache_modes,
            status: self.status,
            action: self.action,
        }
    }
}

impl TryFrom<JsonBookmark> for TargetBookmark {
    type Error = anyhow::Error;

    fn try_from(value: JsonBookmark) -> Result<Self, anyhow::Error> {
        let url = Url::parse(&value.url)?;
        let underlying_type = UnderlyingType::from(&url);

        Ok(Self {
            id: value.id,
            url,
            underlying_url: None,
            underlying_type,
            last_imported: value.last_imported,
            last_cached: value.last_cached,
            sources: value.sources,
            cache_modes: value.cache_modes,
            status: Status::None,
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

    pub fn insert(&mut self, bookmark: TargetBookmark) -> Option<TargetBookmark> {
        self.0.insert(bookmark.url.clone(), bookmark)
    }

    pub fn upsert(&mut self, bookmark: TargetBookmark) {
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

                for source in bookmark.sources {
                    target_bookmark.sources.insert(source);
                }

                target_bookmark.cache_modes = bookmark.cache_modes;
                target_bookmark.action = bookmark.action;
            }
            Entry::Vacant(entry) => {
                let inserted_bookmark = entry.insert(bookmark);
                inserted_bookmark.status = Status::Added;
            }
        }
    }

    pub fn remove(&mut self, url: &Url) -> Option<TargetBookmark> {
        self.0.remove(url)
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
            let bookmark_sources = source_bookmark.1.sources();
            let mut sources = HashSet::new();

            for bookmark_source in bookmark_sources {
                sources.insert(bookmark_source.source_type().to_owned());
            }

            let target_bookmark = TargetBookmarkBuilder::new(url.to_owned(), now)
                .with_sources(sources)
                .build();
            target_bookmarks.insert(target_bookmark);
        }

        Ok(target_bookmarks)
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
                        underlying_url: None,
                        underlying_type: UnderlyingType::None,
                        last_imported: 1694989714351,
                        last_cached: None,
                        sources: HashSet::new(),
                        cache_modes: HashSet::new(),
                        status: Status::None,
                        action: Action::None,
                    }
                ),
                (
                    Url::parse("https://url2.com").unwrap(),
                    TargetBookmark {
                        id: String::from("511b1590-e6de-4989-bca4-96dc61730508"),
                        url: Url::parse("https://url2.com").unwrap(),
                        underlying_url: None,
                        underlying_type: UnderlyingType::None,
                        last_imported: 1694989714351,
                        last_cached: None,
                        sources: HashSet::new(),
                        cache_modes: HashSet::new(),
                        status: Status::None,
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
                    underlying_url: None,
                    underlying_type: UnderlyingType::None,
                    last_imported: 1694989714351,
                    last_cached: None,
                    sources: HashSet::new(),
                    cache_modes: HashSet::new(),
                    status: Status::None,
                    action: Action::None,
                },
            ),
            (
                Url::parse("https://url2.com").unwrap(),
                TargetBookmark {
                    id: String::from("511b1590-e6de-4989-bca4-96dc61730508"),
                    url: Url::parse("https://url2.com").unwrap(),
                    underlying_url: None,
                    underlying_type: UnderlyingType::None,
                    last_imported: 1694989714351,
                    last_cached: None,
                    sources: HashSet::new(),
                    cache_modes: HashSet::new(),
                    status: Status::None,
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
