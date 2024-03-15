use crate::SourceType;
use log::debug;
use std::collections::{
    hash_map::{Entry, IntoIter, Iter, IterMut, Keys},
    HashMap, HashSet,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BookmarkSource {
    source_type: SourceType,
    source_folder: Option<String>,
}

impl BookmarkSource {
    pub fn new(source_type: SourceType, source_folder: Option<String>) -> Self {
        Self {
            source_type,
            source_folder,
        }
    }

    pub fn source_type(&self) -> &SourceType {
        &self.source_type
    }
}

/// A bookmark from a specific source, like Firefox or Chrome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBookmark {
    url: String,
    sources: HashSet<BookmarkSource>,
}

impl SourceBookmark {
    pub fn builder(url: &str) -> SourceBookmarkBuilder {
        SourceBookmarkBuilder {
            url: url.to_owned(),
            sources: HashSet::new(),
        }
    }

    pub fn url(&self) -> &str {
        self.url.as_ref()
    }

    pub fn sources(&self) -> &HashSet<BookmarkSource> {
        &self.sources
    }

    pub fn sources_owned(self) -> HashSet<BookmarkSource> {
        self.sources
    }

    pub fn add_source(&mut self, source: BookmarkSource) {
        self.sources.insert(source);
    }
}

pub struct SourceBookmarkBuilder {
    url: String,
    sources: HashSet<BookmarkSource>,
}

impl SourceBookmarkBuilder {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_owned(),
            sources: HashSet::new(),
        }
    }

    pub fn add_source(mut self, source: BookmarkSource) -> Self {
        self.sources.insert(source);
        self
    }

    pub fn add_source_type(mut self, source_type: &SourceType) -> Self {
        let source = BookmarkSource::new(source_type.to_owned(), None);
        self.sources.insert(source.to_owned());
        self
    }

    pub fn build(self) -> SourceBookmark {
        SourceBookmark {
            url: self.url,
            sources: self.sources,
        }
    }
}

/// Describes the bookmark url which originates from one or more sources.
#[derive(Debug, Clone, Default)]
pub struct SourceBookmarks(HashMap<String, SourceBookmark>);

impl SourceBookmarks {
    pub fn new(bookmarks: HashMap<String, SourceBookmark>) -> Self {
        Self(bookmarks)
    }

    pub fn inner(self) -> HashMap<String, SourceBookmark> {
        self.0
    }

    pub fn get(&self, url: &str) -> Option<&SourceBookmark> {
        self.0.get(url)
    }

    pub fn keys(&self) -> Keys<String, SourceBookmark> {
        self.0.keys()
    }

    pub fn contains_key(&self, url: &str) -> bool {
        self.0.contains_key(url)
    }

    pub fn iter(&self) -> Iter<String, SourceBookmark> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<String, SourceBookmark> {
        self.0.iter_mut()
    }

    pub fn insert(&mut self, bookmark: SourceBookmark) {
        let url = &bookmark.url;
        let sources = bookmark.sources;
        let entry = self.0.entry(url.to_owned());

        match entry {
            Entry::Occupied(entry) => {
                let url = entry.key().clone();
                debug!("Overwrite duplicate source bookmark: {}", url);
                let source_bookmark = entry.into_mut();

                for source in sources {
                    source_bookmark.add_source(source);
                }
            }
            Entry::Vacant(entry) => {
                let source_bookmark = SourceBookmark {
                    url: url.to_owned(),
                    sources,
                };
                entry.insert(source_bookmark);
            }
        }
    }
}

impl IntoIterator for SourceBookmarks {
    type Item = (String, SourceBookmark);
    type IntoIter = IntoIter<String, SourceBookmark>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl AsRef<HashMap<String, SourceBookmark>> for SourceBookmarks {
    fn as_ref(&self) -> &HashMap<String, SourceBookmark> {
        &self.0
    }
}
