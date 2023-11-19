use crate::SourceType;
use log::debug;
use std::collections::{hash_map::Entry, HashMap, HashSet};

/// A bookmark from a specific source, like Firefox or Chrome.
#[derive(Debug, Clone)]
pub struct SourceBookmark {
    url: String,
    source_type: SourceType,
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

    pub fn insert(&mut self, source_bookmark: SourceBookmark) {
        let url = source_bookmark.url;
        let source_type = source_bookmark.source_type;
        let entry = self.0.entry(url);

        match entry {
            Entry::Occupied(entry) => {
                let url = entry.key().clone();
                let source_types = entry.into_mut();
                debug!("Overwrite duplicate bookmark: {}", url);
                source_types.insert(source_type);
            }
            Entry::Vacant(entry) => {
                let mut source_types = HashSet::new();
                source_types.insert(source_type);
                entry.insert(source_types);
            }
        }
    }

    pub fn contains(&self, url: &str) -> bool {
        self.0.contains_key(url)
    }
}

impl AsRef<HashMap<String, HashSet<SourceType>>> for SourceBookmarks {
    fn as_ref(&self) -> &HashMap<String, HashSet<SourceType>> {
        &self.0
    }
}
