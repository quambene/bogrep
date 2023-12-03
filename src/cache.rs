use crate::{
    bookmarks::TargetBookmark,
    errors::BogrepError,
    html,
    utils::{self},
    TargetBookmarks,
};
use async_trait::async_trait;
use chrono::Utc;
use clap::ValueEnum;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt,
    fs::File,
    io::Read,
    marker::{Send, Sync},
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Debug, ValueEnum, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CacheMode {
    Html,
    #[default]
    Text,
}

impl fmt::Display for CacheMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cache_mode = match &self {
            CacheMode::Html => "html",
            CacheMode::Text => "text",
        };
        write!(f, "{}", cache_mode)
    }
}

impl CacheMode {
    /// Use cache mode if it was provided in the CLI command. If cache mode
    /// is not provided, the cache mode configured in the settings is used.
    pub fn new(cache_mode: &Option<CacheMode>, configured: &CacheMode) -> CacheMode {
        cache_mode.as_ref().cloned().unwrap_or(configured.clone())
    }

    pub fn extension(&self) -> &str {
        match self {
            Self::Html => "html",
            Self::Text => "txt",
        }
    }

    pub fn suffix(&self) -> &str {
        match self {
            Self::Html => ".html",
            Self::Text => ".txt",
        }
    }
}

/// A trait to manage the cache in a file system or a mock cache used in
/// testing.
#[async_trait]
pub trait Caching: Send + Sync + 'static {
    // Get the cache mode.
    fn mode(&self) -> &CacheMode;

    // Get the available cache modes.
    fn modes() -> [CacheMode; 2];

    // Check if the cache directory exists or is empty
    fn is_empty(&self) -> bool;

    /// Check if content of bookmark exists in cache.
    fn exists(&self, bookmark: &TargetBookmark) -> bool;

    /// Open the cached file for a bookmark.
    // TODO: return `Result<Option<impl Read>, anyhow::Error>` (see <https://github.com/rust-lang/rust/issues/91611>).
    fn open(&self, bookmark: &TargetBookmark) -> Result<Option<File>, BogrepError>;

    /// Get the content of a bookmark from cache.
    // TODO: make get async
    fn get(&self, bookmark: &TargetBookmark) -> Result<Option<String>, BogrepError>;

    /// Add the content of a bookmark to cache.
    async fn add(&self, html: String, bookmark: &mut TargetBookmark)
        -> Result<String, BogrepError>;

    /// Replace the content of a bookmark in cache.
    async fn replace(
        &self,
        html: String,
        bookmark: &mut TargetBookmark,
    ) -> Result<String, BogrepError>;

    /// Remove the content of a bookmark from cache.
    async fn remove(&self, bookmark: &mut TargetBookmark) -> Result<(), BogrepError>;

    /// Remove the content of multiple bookmarks from cache.
    async fn remove_all(&self, bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError>;

    /// Clear the cache, i.e. remove all files in the cache directory.
    fn clear(&self, bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError>;
}

/// A cache to store the fetched bookmarks.
pub struct Cache {
    /// The path to the cache directory.
    path: PathBuf,
    /// The file extension of the cached files.
    mode: CacheMode,
}

impl Cache {
    /// Create new cache.
    pub fn new(cache_path: &Path, cache_mode: CacheMode) -> Self {
        Self {
            path: cache_path.to_owned(),
            mode: cache_mode,
        }
    }

    fn bookmark_path(&self, bookmark_id: &str) -> PathBuf {
        self.path
            .join(bookmark_id)
            .with_extension(self.mode.extension())
    }

    fn bookmark_path_by_cache_mode(&self, bookmark_id: &str, cache_mode: &CacheMode) -> PathBuf {
        self.path
            .join(bookmark_id)
            .with_extension(cache_mode.extension())
    }
}

#[async_trait]
impl Caching for Cache {
    fn mode(&self) -> &CacheMode {
        &self.mode
    }

    fn modes() -> [CacheMode; 2] {
        [CacheMode::Text, CacheMode::Html]
    }

    fn is_empty(&self) -> bool {
        self.path.exists()
            && std::fs::read_dir(&self.path).is_ok_and(|mut file| file.next().is_none())
    }

    fn exists(&self, bookmark: &TargetBookmark) -> bool {
        bookmark.cache_modes.contains(self.mode())
    }

    fn open(&self, bookmark: &TargetBookmark) -> Result<Option<File>, BogrepError> {
        let cache_path = self.bookmark_path(&bookmark.id);
        debug!("Open website: {}", cache_path.display());

        if cache_path.exists() {
            let cache_file = utils::open_file(&cache_path)?;
            Ok(Some(cache_file))
        } else {
            Ok(None)
        }
    }

    fn get(&self, bookmark: &TargetBookmark) -> Result<Option<String>, BogrepError> {
        if let Some(mut cache_file) = self.open(bookmark)? {
            debug!(
                "Get website from cache: {}",
                self.bookmark_path(&bookmark.id).display()
            );
            let mut buf = String::new();
            cache_file
                .read_to_string(&mut buf)
                .map_err(BogrepError::ReadFile)?;
            Ok(Some(buf))
        } else {
            Ok(None)
        }
    }

    async fn add(
        &self,
        html: String,
        bookmark: &mut TargetBookmark,
    ) -> Result<String, BogrepError> {
        let cache_path = self.bookmark_path(&bookmark.id);

        let content = match self.mode {
            CacheMode::Html => html,
            CacheMode::Text => html::convert_to_text(&html, &bookmark.url)?,
        };

        if !cache_path.exists() {
            debug!("Add website to cache: {}", cache_path.display());
            utils::write_file_async(&cache_path, content.as_bytes()).await?;

            bookmark.last_cached = Some(Utc::now().timestamp_millis());
            bookmark.cache_modes.insert(self.mode.clone());
        }

        Ok(content)
    }

    async fn replace(
        &self,
        html: String,
        bookmark: &mut TargetBookmark,
    ) -> Result<String, BogrepError> {
        let cache_path = self.bookmark_path(&bookmark.id);
        debug!("Replace website in cache: {}", cache_path.display());

        let content = match self.mode {
            CacheMode::Html => html,
            CacheMode::Text => html::convert_to_text(&html, &bookmark.url)?,
        };

        utils::write_file_async(&cache_path, content.as_bytes()).await?;

        bookmark.last_cached = Some(Utc::now().timestamp_millis());
        bookmark.cache_modes.insert(self.mode.clone());

        Ok(content)
    }

    async fn remove(&self, bookmark: &mut TargetBookmark) -> Result<(), BogrepError> {
        let cache_path = self.bookmark_path(&bookmark.id);

        if cache_path.exists() {
            debug!("Remove website from cache: {}", cache_path.display());
            utils::remove_file_async(&cache_path).await?;
            bookmark.last_cached = None;
            bookmark.cache_modes.remove(&self.mode);
        }

        Ok(())
    }

    async fn remove_all(&self, bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError> {
        debug!("Remove all cached websites");
        for bookmark in bookmarks.values_mut() {
            let cache_path = self.bookmark_path(&bookmark.id);

            if cache_path.exists() {
                debug!("Remove website from cache: {}", cache_path.display());
                utils::remove_file_async(&cache_path).await?;
                bookmark.last_cached = None;
                bookmark.cache_modes.remove(&self.mode);
            }
        }

        Ok(())
    }

    /// Clears the cache.
    ///
    /// Note: For safety reasons, `clear` iterates over the given `bookmarks`
    /// instead of using [`std::fs::remove_dir_all`] for the cache directory.
    fn clear(&self, bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError> {
        debug!("Clear cache");
        let cache_modes = Cache::modes();

        for bookmark in bookmarks.values_mut() {
            for cache_mode in &cache_modes {
                let cache_path = self.bookmark_path_by_cache_mode(&bookmark.id, cache_mode);

                if cache_path.exists() {
                    debug!("Remove website from cache: {}", cache_path.display());
                    utils::remove_file(&cache_path)?;
                    bookmark.last_cached = None;
                    bookmark.cache_modes.clear();
                }
            }
        }

        Ok(())
    }
}

/// A mock cache to store fetched bookmarks used in testing.
#[derive(Debug, Default)]
pub struct MockCache {
    /// Mock the file system.
    cache_map: Mutex<HashMap<String, String>>,
    /// The file extension of the cached files.
    mode: CacheMode,
}

impl MockCache {
    pub fn new(cache_mode: CacheMode) -> Self {
        let cache_map = Mutex::new(HashMap::new());
        Self {
            cache_map,
            mode: cache_mode,
        }
    }

    pub fn cache_map(&self) -> HashMap<String, String> {
        let cache_map = self.cache_map.lock().unwrap();
        trace!("Get mock cash: {:?}", cache_map);
        cache_map.clone()
    }
}

#[async_trait]
impl Caching for MockCache {
    fn mode(&self) -> &CacheMode {
        &self.mode
    }

    fn modes() -> [CacheMode; 2] {
        [CacheMode::Text, CacheMode::Html]
    }

    fn is_empty(&self) -> bool {
        let cache_map = self.cache_map.lock().unwrap();
        cache_map.is_empty()
    }

    fn exists(&self, bookmark: &TargetBookmark) -> bool {
        self.get(bookmark).unwrap().is_some()
    }

    fn open(&self, _bookmark: &TargetBookmark) -> Result<Option<File>, BogrepError> {
        Ok(None)
    }

    fn get(&self, bookmark: &TargetBookmark) -> Result<Option<String>, BogrepError> {
        let cache_map = self.cache_map.lock().unwrap();
        let content = cache_map
            .get(&bookmark.id)
            .map(|content| content.to_owned());
        Ok(content)
    }

    async fn add(
        &self,
        html: String,
        bookmark: &mut TargetBookmark,
    ) -> Result<String, BogrepError> {
        debug!("Add bookmark ({}) to cache", bookmark.url);
        let mut cache_map = self.cache_map.lock().unwrap();
        let content = match self.mode {
            CacheMode::Html => html,
            CacheMode::Text => html::convert_to_text(&html, &bookmark.url)?,
        };
        cache_map.insert(bookmark.id.clone(), content.clone());

        bookmark.last_cached = Some(Utc::now().timestamp_millis());
        bookmark.cache_modes.insert(self.mode.clone());

        Ok(content)
    }

    async fn replace(
        &self,
        html: String,
        bookmark: &mut TargetBookmark,
    ) -> Result<String, BogrepError> {
        debug!("Replace bookmark ({}) in cache", bookmark.url);
        let mut cache_map = self.cache_map.lock().unwrap();
        let content = match self.mode {
            CacheMode::Html => html,
            CacheMode::Text => html::convert_to_text(&html, &bookmark.url)?,
        };
        cache_map.insert(bookmark.id.clone(), content.clone());

        bookmark.last_cached = Some(Utc::now().timestamp_millis());
        bookmark.cache_modes.insert(self.mode.clone());

        Ok(content)
    }

    async fn remove(&self, bookmark: &mut TargetBookmark) -> Result<(), BogrepError> {
        debug!("Remove bookmark ({}) from cache", bookmark.url);
        let mut cache_map = self.cache_map.lock().unwrap();
        cache_map.remove(&bookmark.id);

        bookmark.last_cached = None;
        bookmark.cache_modes.remove(&self.mode);

        Ok(())
    }

    async fn remove_all(&self, bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError> {
        let mut cache_map = self.cache_map.lock().unwrap();

        for bookmark in bookmarks.values_mut() {
            cache_map.remove(&bookmark.id);

            bookmark.last_cached = None;
            bookmark.cache_modes.remove(&self.mode);
        }

        Ok(())
    }

    fn clear(&self, bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError> {
        let mut cache_map = self.cache_map.lock().unwrap();
        cache_map.clear();

        for bookmark in bookmarks.values_mut() {
            bookmark.last_cached = None;
            bookmark.cache_modes.clear();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashSet;

    #[tokio::test]
    async fn test_add_mode_html() {
        let cache = MockCache::new(CacheMode::Html);
        let now = Utc::now();
        let mut bookmark = TargetBookmark::new(
            "https://url.com",
            now,
            None,
            HashSet::new(),
            HashSet::new(),
            None,
        );
        let content = "<html><head></head><body><p>Test content</p></body></html>";
        let cached_content = cache.add(content.to_owned(), &mut bookmark).await.unwrap();
        assert_eq!(
            cached_content,
            "<html><head></head><body><p>Test content</p></body></html>"
        );
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 1);
        assert!(bookmark.last_cached.is_some());
        assert!(bookmark.cache_modes.contains(&CacheMode::Html));
    }

    #[tokio::test]
    async fn test_add_mode_text() {
        let cache = MockCache::new(CacheMode::Text);
        let now = Utc::now();
        let mut bookmark = TargetBookmark::new(
            "https://url.com",
            now,
            None,
            HashSet::new(),
            HashSet::new(),
            None,
        );
        let content = "<html><head></head><body><p>Test content</p></body></html>";
        let cached_content = cache.add(content.to_owned(), &mut bookmark).await.unwrap();
        assert_eq!(cached_content, "Test content");
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 1);
        assert!(bookmark.last_cached.is_some());
        assert!(bookmark.cache_modes.contains(&CacheMode::Text));
    }

    #[tokio::test]
    async fn test_replace_mode_html() {
        let cache = MockCache::new(CacheMode::Html);
        let now = Utc::now();
        let mut bookmark = TargetBookmark::new(
            "https://url.com",
            now,
            None,
            HashSet::new(),
            HashSet::new(),
            None,
        );
        let content1 = "<html><head></head><body><p>Test content 1</p></body></html>";
        cache.add(content1.to_owned(), &mut bookmark).await.unwrap();
        let content2 = "<html><head></head><body><p>Test content 2</p></body></html>";
        let replaced_content = cache
            .replace(content2.to_owned(), &mut bookmark)
            .await
            .unwrap();
        assert_eq!(
            replaced_content,
            "<html><head></head><body><p>Test content 2</p></body></html>"
        );
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 1);
    }

    #[tokio::test]
    async fn test_replace_mode_text() {
        let cache = MockCache::new(CacheMode::Text);
        let now = Utc::now();
        let mut bookmark = TargetBookmark::new(
            "https://test_url.com",
            now,
            None,
            HashSet::new(),
            HashSet::new(),
            None,
        );
        let content1 = "<html><head></head><body><p>Test content 1</p></body></html>";
        cache.add(content1.to_owned(), &mut bookmark).await.unwrap();
        let content2 = "<html><head></head><body><p>Test content 2</p></body></html>";
        let replaced_content = cache
            .replace(content2.to_owned(), &mut bookmark)
            .await
            .unwrap();
        assert_eq!(replaced_content, "Test content 2");
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 1);
    }

    #[tokio::test]
    async fn test_remove_mode_html() {
        let cache = MockCache::new(CacheMode::Html);
        let now = Utc::now();
        let mut bookmark = TargetBookmark::new(
            "https://url.com",
            now,
            None,
            HashSet::new(),
            HashSet::new(),
            None,
        );
        let content = "<html><head></head><body><p>Test content</p></body></html>";

        cache.add(content.to_owned(), &mut bookmark).await.unwrap();
        assert!(bookmark.last_cached.is_some());
        assert!(bookmark.cache_modes.contains(&CacheMode::Html));

        cache.remove(&mut bookmark).await.unwrap();
        assert!(bookmark.last_cached.is_none());
        assert!(bookmark.cache_modes.is_empty());
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 0);
    }

    #[tokio::test]
    async fn test_remove_all_mode_html() {
        let cache = MockCache::new(CacheMode::Html);
        let now = Utc::now();
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                "https://url1.com".to_owned(),
                TargetBookmark::new(
                    "https://url1.com",
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    None,
                ),
            ),
            (
                "https://url2.com".to_owned(),
                TargetBookmark::new(
                    "https://url2.com",
                    now,
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    None,
                ),
            ),
        ]));

        for bookmark in target_bookmarks.values_mut() {
            cache
                .add(
                    "<html><head></head><body><p>Test content</p></body></html>".to_owned(),
                    bookmark,
                )
                .await
                .unwrap();
        }

        cache.remove_all(&mut target_bookmarks).await.unwrap();
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 0);
    }

    #[tokio::test]
    async fn test_clear_mode_html() {
        let cache = MockCache::new(CacheMode::Html);
        let now = Utc::now();
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([(
            "https://url.com".to_owned(),
            TargetBookmark::new(
                "https://url.com",
                now,
                None,
                HashSet::new(),
                HashSet::new(),
                None,
            ),
        )]));

        for bookmark in target_bookmarks.values_mut() {
            cache
                .add(
                    "<html><head></head><body><p>Test content</p></body></html>".to_owned(),
                    bookmark,
                )
                .await
                .unwrap();
        }

        cache.clear(&mut target_bookmarks).unwrap();
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 0);
    }
}
