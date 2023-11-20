use crate::SourceType;
use log::debug;
use std::collections::{
    hash_map::{Entry, IntoIter, Iter, Keys},
    HashMap, HashSet,
};

/// A bookmark from a specific source, like Firefox or Chrome.
#[derive(Debug, Clone)]
pub struct SourceBookmark {
    pub url: String,
    pub source_type: SourceType,
}

impl SourceBookmark {
    pub fn new(url: String, source_type: SourceType) -> Self {
        Self { url, source_type }
    }
}

/// Describes the bookmark url which originates from one or more sources.
#[derive(Debug, Clone, Default)]
pub struct SourceBookmarks(HashMap<String, HashSet<SourceType>>);

impl SourceBookmarks {
    pub fn new(bookmarks: HashMap<String, HashSet<SourceType>>) -> Self {
        Self(bookmarks)
    }

    pub fn inner(self) -> HashMap<String, HashSet<SourceType>> {
        self.0
    }

    pub fn get(&self, url: &str) -> Option<&HashSet<SourceType>> {
        self.0.get(url)
    }

    pub fn keys(&self) -> Keys<String, HashSet<SourceType>> {
        self.0.keys()
    }

    pub fn contains_key(&self, url: &str) -> bool {
        self.0.contains_key(url)
    }

    pub fn iter(&self) -> Iter<String, HashSet<SourceType>> {
        self.0.iter()
    }

    pub fn insert(&mut self, bookmark: SourceBookmark) {
        let url = bookmark.url;
        let source_type = bookmark.source_type;
        let entry = self.0.entry(url);

        match entry {
            Entry::Occupied(entry) => {
                let url = entry.key().clone();
                let source_types = entry.into_mut();
                debug!("Overwrite duplicate source bookmark: {}", url);
                source_types.insert(source_type);
            }
            Entry::Vacant(entry) => {
                let mut source_types = HashSet::new();
                source_types.insert(source_type);
                entry.insert(source_types);
            }
        }
    }
}

impl IntoIterator for SourceBookmarks {
    type Item = (String, HashSet<SourceType>);
    type IntoIter = IntoIter<String, HashSet<SourceType>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl AsRef<HashMap<String, HashSet<SourceType>>> for SourceBookmarks {
    fn as_ref(&self) -> &HashMap<String, HashSet<SourceType>> {
        &self.0
    }
}
