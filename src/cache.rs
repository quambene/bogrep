use crate::{
    bookmarks::TargetBookmark,
    html,
    utils::{self},
    TargetBookmarks,
};
use async_trait::async_trait;
use clap::ValueEnum;
use log::debug;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tokio::{fs, io::AsyncWriteExt};

#[derive(Debug, ValueEnum, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CacheMode {
    Html,
    #[default]
    Text,
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
pub trait Caching {
    // Get the cache mode.
    fn mode(&self) -> &CacheMode;

    // Get the available cache modes.
    fn modes() -> [CacheMode; 2];

    /// Check if content of bookmark exists in cache.
    fn exists(&self, bookmark: &TargetBookmark) -> bool;

    /// Open the cached file for a bookmark.
    // TODO: return `Result<Option<impl Read>, anyhow::Error>` (see <https://github.com/rust-lang/rust/issues/91611>).
    fn open(&self, bookmark: &TargetBookmark) -> Result<Option<File>, anyhow::Error>;

    /// Get the content of a bookmark from cache.
    // TODO: make get async
    fn get(&self, bookmark: &TargetBookmark) -> Result<Option<String>, anyhow::Error>;

    /// Add the content of a bookmark to cache.
    async fn add(&self, html: String, bookmark: &TargetBookmark) -> Result<String, anyhow::Error>;

    /// Replace the content of a bookmark in cache.
    async fn replace(
        &self,
        html: String,
        bookmark: &TargetBookmark,
    ) -> Result<String, anyhow::Error>;

    /// Remove the content of a bookmark from cache.
    async fn remove(&self, bookmark: &TargetBookmark) -> Result<(), anyhow::Error>;

    /// Remove the content of multiple bookmarks from cache.
    async fn remove_all(&self, bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error>;

    /// Clear the cache, i.e. remove all files in the cache directory.
    fn clear(&self, bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error>;
}

/// The cache to store fetched bookmarks.
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

    fn path(&self, bookmark_id: &str) -> PathBuf {
        self.path
            .join(bookmark_id)
            .with_extension(self.mode.extension())
    }

    fn path_by_cache_mode(&self, bookmark_id: &str, cache_mode: &CacheMode) -> PathBuf {
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

    fn exists(&self, bookmark: &TargetBookmark) -> bool {
        let cache_path = self.path(&bookmark.id);
        cache_path.exists()
    }

    fn open(&self, bookmark: &TargetBookmark) -> Result<Option<File>, anyhow::Error> {
        let cache_path = self.path(&bookmark.id);
        debug!("Open website: {}", cache_path.display());

        if cache_path.exists() {
            let cache_file = utils::open_file(&cache_path)?;
            Ok(Some(cache_file))
        } else {
            Ok(None)
        }
    }

    fn get(&self, bookmark: &TargetBookmark) -> Result<Option<String>, anyhow::Error> {
        if let Some(mut cache_file) = self.open(bookmark)? {
            debug!(
                "Get website from cache: {}",
                self.path(&bookmark.id).display()
            );
            let mut buf = String::new();
            cache_file.read_to_string(&mut buf)?;
            Ok(Some(buf))
        } else {
            Ok(None)
        }
    }

    async fn add(&self, html: String, bookmark: &TargetBookmark) -> Result<String, anyhow::Error> {
        let cache_path = self.path(&bookmark.id);

        let content = match self.mode {
            CacheMode::Html => html,
            CacheMode::Text => html::convert_to_text(&html, &bookmark.url)?,
        };

        if !cache_path.exists() {
            debug!("Add website to cache: {}", cache_path.display());
            let mut cache_file = utils::create_file_async(&cache_path).await?;
            cache_file.write_all(content.as_bytes()).await?;
            cache_file.flush().await?;
        }

        Ok(content)
    }

    async fn replace(
        &self,
        html: String,
        bookmark: &TargetBookmark,
    ) -> Result<String, anyhow::Error> {
        let cache_path = self.path(&bookmark.id);
        debug!("Replace website in cache: {}", cache_path.display());

        let content = match self.mode {
            CacheMode::Html => html,
            CacheMode::Text => html::convert_to_text(&html, &bookmark.url)?,
        };

        let mut cache_file = utils::create_file_async(&cache_path).await?;
        cache_file.write_all(content.as_bytes()).await?;
        cache_file.flush().await?;
        Ok(content)
    }

    async fn remove(&self, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        let cache_file = self.path(&bookmark.id);

        if cache_file.exists() {
            debug!("Remove website from cache: {}", cache_file.display());
            fs::remove_file(cache_file).await?;
        }

        Ok(())
    }

    async fn remove_all(&self, bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error> {
        debug!("Remove all cached websites");
        for bookmark in &bookmarks.bookmarks {
            let cache_file = self.path(&bookmark.id);

            if cache_file.exists() {
                debug!("Remove website from cache: {}", cache_file.display());
                fs::remove_file(cache_file).await?;
            }
        }

        Ok(())
    }

    /// Clears the cache.
    ///
    /// Note: For safety reasons, `clear` iterates over the given `bookmarks`
    /// instead of using [`std::fs::remove_dir_all`] for the cache directory.
    fn clear(&self, bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error> {
        debug!("Clear cache");
        let cache_modes = Cache::modes();

        for bookmark in &bookmarks.bookmarks {
            for cache_mode in &cache_modes {
                let cache_file = self.path_by_cache_mode(&bookmark.id, cache_mode);

                if cache_file.exists() {
                    debug!("Remove website from cache: {}", cache_file.display());
                    std::fs::remove_file(cache_file)?;
                }
            }
        }

        Ok(())
    }
}

/// The cache to store fetched bookmarks.
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

    fn exists(&self, bookmark: &TargetBookmark) -> bool {
        self.get(bookmark).unwrap().is_some()
    }

    fn open(&self, _bookmark: &TargetBookmark) -> Result<Option<File>, anyhow::Error> {
        Ok(None)
    }

    fn get(&self, bookmark: &TargetBookmark) -> Result<Option<String>, anyhow::Error> {
        let cache_map = self.cache_map.lock().unwrap();
        let content = cache_map
            .get(&bookmark.id)
            .map(|content| content.to_owned());
        Ok(content)
    }

    async fn add(&self, html: String, bookmark: &TargetBookmark) -> Result<String, anyhow::Error> {
        let mut cache_map = self.cache_map.lock().unwrap();
        let content = match self.mode {
            CacheMode::Html => html,
            CacheMode::Text => html::convert_to_text(&html, &bookmark.url)?,
        };
        cache_map.insert(bookmark.id.clone(), content.clone());
        Ok(content)
    }

    async fn replace(
        &self,
        html: String,
        bookmark: &TargetBookmark,
    ) -> Result<String, anyhow::Error> {
        let mut cache_map = self.cache_map.lock().unwrap();
        let content = match self.mode {
            CacheMode::Html => html,
            CacheMode::Text => html::convert_to_text(&html, &bookmark.url)?,
        };
        cache_map.insert(bookmark.id.clone(), content.clone());
        Ok(content)
    }

    async fn remove(&self, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        let mut cache_map = self.cache_map.lock().unwrap();
        cache_map.remove(&bookmark.id);
        Ok(())
    }

    async fn remove_all(&self, bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error> {
        let mut cache_map = self.cache_map.lock().unwrap();

        for bookmark in &bookmarks.bookmarks {
            cache_map.remove(&bookmark.id);
        }

        Ok(())
    }

    fn clear(&self, _bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error> {
        let mut cache_map = self.cache_map.lock().unwrap();
        cache_map.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_add_mode_html() {
        let cache = MockCache::new(CacheMode::Html);
        let now = Utc::now();
        let bookmark = TargetBookmark::new("https://test_url.com", now, None);
        let content = "<html><head></head><body><p>Test content</p></body></html>";
        let cached_content = cache.add(content.to_owned(), &bookmark).await.unwrap();
        assert_eq!(
            cached_content,
            "<html><head></head><body><p>Test content</p></body></html>"
        );
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 1);
    }

    #[tokio::test]
    async fn test_add_mode_text() {
        let cache = MockCache::new(CacheMode::Text);
        let now = Utc::now();
        let bookmark = TargetBookmark::new("https://test_url.com", now, None);
        let content = "<html><head></head><body><p>Test content</p></body></html>";
        let cached_content = cache.add(content.to_owned(), &bookmark).await.unwrap();
        assert_eq!(cached_content, "Test content");
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 1);
    }

    #[tokio::test]
    async fn test_replace_mode_html() {
        let cache = MockCache::new(CacheMode::Html);
        let now = Utc::now();
        let bookmark = TargetBookmark::new("https://test_url.com", now, None);
        let content1 = "<html><head></head><body><p>Test content 1</p></body></html>";
        cache.add(content1.to_owned(), &bookmark).await.unwrap();
        let content2 = "<html><head></head><body><p>Test content 2</p></body></html>";
        let replaced_content = cache.replace(content2.to_owned(), &bookmark).await.unwrap();
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
        let bookmark = TargetBookmark::new("https://test_url.com", now, None);
        let content1 = "<html><head></head><body><p>Test content 1</p></body></html>";
        cache.add(content1.to_owned(), &bookmark).await.unwrap();
        let content2 = "<html><head></head><body><p>Test content 2</p></body></html>";
        let replaced_content = cache.replace(content2.to_owned(), &bookmark).await.unwrap();
        assert_eq!(replaced_content, "Test content 2");
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 1);
    }

    #[tokio::test]
    async fn test_remove_mode_html() {
        let cache = MockCache::new(CacheMode::Html);
        let now = Utc::now();
        let bookmark = TargetBookmark::new("https://test_url.com", now, None);
        let content = "<html><head></head><body><p>Test content</p></body></html>";
        cache.add(content.to_owned(), &bookmark).await.unwrap();
        cache.remove(&bookmark).await.unwrap();
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 0);
    }

    #[tokio::test]
    async fn test_remove_all_mode_html() {
        let cache = MockCache::new(CacheMode::Html);
        let now = Utc::now();
        let bookmarks = vec![
            TargetBookmark::new("https://test_url1.com", now, None),
            TargetBookmark::new("https://test_url2.com", now, None),
        ];

        for bookmark in &bookmarks {
            cache
                .add(
                    "<html><head></head><body><p>Test content</p></body></html>".to_owned(),
                    bookmark,
                )
                .await
                .unwrap();
        }

        let target_bookmarks = TargetBookmarks { bookmarks };

        cache.remove_all(&target_bookmarks).await.unwrap();
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 0);
    }

    #[tokio::test]
    async fn test_clear_mode_html() {
        let cache = MockCache::new(CacheMode::Html);
        let now = Utc::now();
        let bookmarks = vec![TargetBookmark::new("https://test_url.com", now, None)];

        for bookmark in &bookmarks {
            cache
                .add(
                    "<html><head></head><body><p>Test content</p></body></html>".to_owned(),
                    bookmark,
                )
                .await
                .unwrap();
        }

        let target_bookmarks = TargetBookmarks { bookmarks };

        cache.clear(&target_bookmarks).unwrap();
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 0);
    }
}
