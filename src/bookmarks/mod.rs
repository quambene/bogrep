mod source_bookmarks;
mod target_bookmarks;

use serde::{Deserialize, Serialize};
pub use source_bookmarks::{SourceBookmark, SourceBookmarks};
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    slice::Iter,
};
pub use target_bookmarks::{TargetBookmark, TargetBookmarks};

/// The action to be performed on the bookmark.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum Action {
    Add,
    Remove,
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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BookmarksJson {
    pub bookmarks: Vec<TargetBookmark>,
}

impl BookmarksJson {
    pub fn new(mut bookmarks: Vec<TargetBookmark>) -> Self {
        bookmarks.sort_by(Self::compare);

        Self { bookmarks }
    }

    pub fn iter(&self) -> Iter<TargetBookmark> {
        self.bookmarks.iter()
    }

    pub fn get(&self, index: usize) -> Option<&TargetBookmark> {
        self.bookmarks.get(index)
    }

    pub fn is_empty(&self) -> bool {
        self.bookmarks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.bookmarks.len()
    }

    // Sort by `last_cached` and then by `url`.
    fn compare(a: &TargetBookmark, b: &TargetBookmark) -> Ordering {
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

impl From<&TargetBookmarks> for BookmarksJson {
    fn from(target_bookmarks: &TargetBookmarks) -> Self {
        let mut bookmarks = target_bookmarks.values().cloned().collect::<Vec<_>>();
        bookmarks.sort_by(Self::compare);
        BookmarksJson { bookmarks }
    }
}

impl IntoIterator for BookmarksJson {
    type Item = TargetBookmark;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.bookmarks.into_iter()
    }
}

pub struct BookmarksJsonIterator<'a> {
    bookmarks_iter: Iter<'a, TargetBookmark>,
}

impl<'a> IntoIterator for &'a BookmarksJson {
    type Item = &'a TargetBookmark;
    type IntoIter = BookmarksJsonIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        BookmarksJsonIterator {
            bookmarks_iter: self.bookmarks.iter(),
        }
    }
}

impl<'a> Iterator for BookmarksJsonIterator<'a> {
    type Item = &'a TargetBookmark;

    fn next(&mut self) -> Option<Self::Item> {
        self.bookmarks_iter.next()
    }
}
