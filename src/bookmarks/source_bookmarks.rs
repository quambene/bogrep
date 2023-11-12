use log::debug;
use std::collections::HashSet;

/// Describes the bookmark url from a specific source, like Firefox or Chrome.
#[derive(Debug, Clone)]
pub struct SourceBookmarks {
    pub bookmarks: HashSet<String>,
}

impl Default for SourceBookmarks {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceBookmarks {
    pub fn new() -> Self {
        Self {
            bookmarks: HashSet::new(),
        }
    }

    pub fn insert(&mut self, url: &str) {
        let is_new_bookmark = self.bookmarks.insert(url.to_owned());

        if !is_new_bookmark {
            debug!("Overwrite duplicate bookmark: {}", url);
        }
    }
}
