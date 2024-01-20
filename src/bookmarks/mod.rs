mod source_bookmarks;
mod target_bookmarks;

use crate::CacheMode;
use serde::{Deserialize, Serialize};
pub use source_bookmarks::{SourceBookmark, SourceBookmarkBuilder, SourceBookmarks};
use std::{
    cmp::Ordering,
    collections::HashSet,
    path::{Path, PathBuf},
    slice::Iter,
};
pub use target_bookmarks::{TargetBookmark, TargetBookmarks};
use uuid::Uuid;

/// The action to be performed on the bookmark.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum Action {
    /// Fetch and cache the bookmark, even if it is cached already. The cached
    /// content will be updated with the most recent version of the website.
    FetchAndReplace,
    /// Fetch and cache bookmark if it is not cached yet.
    FetchAndAdd,
    /// Remove bookmark from cache.
    Remove,
    /// No actions to be performed.
    None,
}

/// The type used to identify a source.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceType {
    Firefox,
    Chromium,
    Chrome,
    Edge,
    Simple,
    Internal,
    External,
    #[default]
    Others,
}

/// The source of bookmarks.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct RawSource {
    /// The path to the source file or directory.
    #[serde(rename = "source")]
    pub path: PathBuf,
    /// The folders to be imported.
    ///
    /// If no folders are selected, all bookmarks in the source file will be
    /// imported.
    pub folders: Vec<String>,
}

impl RawSource {
    pub fn new(path: impl Into<PathBuf>, folders: Vec<String>) -> Self {
        Self {
            path: path.into(),
            folders,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Source {
    /// The name of the source.
    pub name: SourceType,
    /// The path of the source file used for displaying.
    pub path: String,
    /// The folders to be imported.
    ///
    /// If no folders are selected, all bookmarks in the source file will be
    /// imported.
    pub folders: Vec<String>,
}

impl Source {
    pub fn new(name: SourceType, path: &Path, folders: Vec<String>) -> Self {
        Self {
            name,
            path: path.to_string_lossy().to_string(),
            folders,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonBookmark {
    pub id: String,
    pub url: String,
    pub last_imported: i64,
    pub last_cached: Option<i64>,
    pub sources: HashSet<SourceType>,
    pub cache_modes: HashSet<CacheMode>,
}

impl JsonBookmark {
    pub fn new(
        url: String,
        last_imported: i64,
        last_cached: Option<i64>,
        sources: HashSet<SourceType>,
        cache_modes: HashSet<CacheMode>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            url,
            last_imported,
            last_cached,
            sources,
            cache_modes,
        }
    }
}

impl From<TargetBookmark> for JsonBookmark {
    fn from(value: TargetBookmark) -> Self {
        Self {
            id: value.id,
            url: value.url.to_string(),
            last_imported: value.last_imported,
            last_cached: value.last_cached,
            sources: value.sources,
            cache_modes: value.cache_modes,
        }
    }
}

impl From<&TargetBookmark> for JsonBookmark {
    fn from(value: &TargetBookmark) -> Self {
        Self {
            id: value.id.clone(),
            url: value.url.to_string(),
            last_imported: value.last_imported,
            last_cached: value.last_cached,
            sources: value.sources.clone(),
            cache_modes: value.cache_modes.clone(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct JsonBookmarks {
    pub bookmarks: Vec<JsonBookmark>,
}

impl JsonBookmarks {
    pub fn new(bookmarks: Vec<TargetBookmark>) -> Self {
        let mut bookmarks = bookmarks
            .into_iter()
            .map(JsonBookmark::from)
            .collect::<Vec<_>>();
        bookmarks.sort_by(Self::compare);

        Self { bookmarks }
    }

    pub fn iter(&self) -> Iter<JsonBookmark> {
        self.bookmarks.iter()
    }

    pub fn get(&self, index: usize) -> Option<&JsonBookmark> {
        self.bookmarks.get(index)
    }

    pub fn is_empty(&self) -> bool {
        self.bookmarks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.bookmarks.len()
    }

    // Sort by `last_cached` and then by `url`.
    fn compare(a: &JsonBookmark, b: &JsonBookmark) -> Ordering {
        match (a.last_cached, b.last_cached) {
            (Some(a_cached), Some(b_cached)) => {
                a_cached.cmp(&b_cached).then_with(|| a.url.cmp(&b.url))
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => a.url.cmp(&b.url),
        }
    }
}

impl From<&TargetBookmarks> for JsonBookmarks {
    fn from(target_bookmarks: &TargetBookmarks) -> Self {
        let mut bookmarks = target_bookmarks
            .values()
            .map(JsonBookmark::from)
            .collect::<Vec<_>>();
        bookmarks.sort_by(Self::compare);
        JsonBookmarks { bookmarks }
    }
}

impl IntoIterator for JsonBookmarks {
    type Item = JsonBookmark;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.bookmarks.into_iter()
    }
}

pub struct JsonBookmarksIterator<'a> {
    bookmarks_iter: Iter<'a, JsonBookmark>,
}

impl<'a> IntoIterator for &'a JsonBookmarks {
    type Item = &'a JsonBookmark;
    type IntoIter = JsonBookmarksIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        JsonBookmarksIterator {
            bookmarks_iter: self.bookmarks.iter(),
        }
    }
}

impl<'a> Iterator for JsonBookmarksIterator<'a> {
    type Item = &'a JsonBookmark;

    fn next(&mut self) -> Option<Self::Item> {
        self.bookmarks_iter.next()
    }
}
