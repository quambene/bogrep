use super::SourceFolder;
use crate::SourceType;
use log::debug;
use std::collections::{
    hash_map::{Entry, IntoIter, Iter, IterMut, Keys},
    HashMap, HashSet,
};

/// A bookmark from a specific source, like Firefox or Chrome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBookmark {
    url: String,
    sources: HashSet<SourceType>,
    folders: HashSet<SourceFolder>,
}

impl SourceBookmark {
    pub fn builder(url: &str) -> SourceBookmarkBuilder {
        SourceBookmarkBuilder {
            url: url.to_owned(),
            sources: HashSet::new(),
            folders: HashSet::new(),
        }
    }

    pub fn url(&self) -> &str {
        self.url.as_ref()
    }

    pub fn sources(&self) -> &HashSet<SourceType> {
        &self.sources
    }

    pub fn sources_owned(self) -> HashSet<SourceType> {
        self.sources
    }

    pub fn folders(&self) -> &HashSet<SourceFolder> {
        &self.folders
    }

    pub fn folders_owned(self) -> HashSet<SourceFolder> {
        self.folders
    }

    pub fn add_source(&mut self, source: SourceType) {
        self.sources.insert(source);
    }

    pub fn add_folder(&mut self, source: SourceFolder) {
        self.folders.insert(source);
    }
}

pub struct SourceBookmarkBuilder {
    url: String,
    sources: HashSet<SourceType>,
    folders: HashSet<SourceFolder>,
}

impl SourceBookmarkBuilder {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_owned(),
            sources: HashSet::new(),
            folders: HashSet::new(),
        }
    }

    pub fn add_source(mut self, source: SourceType) -> Self {
        self.sources.insert(source);
        self
    }

    pub fn add_folder(mut self, source: SourceType, name: impl Into<String>) -> Self {
        let folder = SourceFolder::new(source, name.into());
        self.folders.insert(folder);
        self
    }

    pub fn add_folder_opt(mut self, source: SourceType, name: Option<impl Into<String>>) -> Self {
        if let Some(name) = name {
            let folder = SourceFolder::new(source, name.into());
            self.folders.insert(folder);
        }

        self
    }

    pub fn build(self) -> SourceBookmark {
        SourceBookmark {
            url: self.url,
            sources: self.sources,
            folders: self.folders,
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
        let folders = bookmark.folders;
        let entry = self.0.entry(url.to_owned());

        match entry {
            Entry::Occupied(entry) => {
                let url = entry.key().clone();
                debug!("Overwrite duplicate source bookmark: {}", url);
                let source_bookmark = entry.into_mut();

                for source in sources {
                    source_bookmark.add_source(source);
                }

                for folder in folders {
                    source_bookmark.add_folder(folder);
                }
            }
            Entry::Vacant(entry) => {
                let source_bookmark = SourceBookmark {
                    url: url.to_owned(),
                    sources,
                    folders,
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
