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

#[async_trait]
pub trait Caching {
    /// Check if content of bookmark exists in cache.
    fn exists(&self, bookmark: &TargetBookmark) -> bool;

    /// Open the cached file for a bookmark.
    // TODO: return `Result<Option<impl Read>, anyhow::Error>` (see <https://github.com/rust-lang/rust/issues/91611>).
    fn open(&self, bookmark: &TargetBookmark) -> Result<Option<File>, anyhow::Error>;

    /// Get the content of a bookmark from cache.
    // TODO: make get async
    fn get(&self, bookmark: &TargetBookmark) -> Result<Option<String>, anyhow::Error>;

    /// Add the content of a bookmark to cache.
    async fn add(&self, html: String, bookmark: &TargetBookmark) -> Result<(), anyhow::Error>;

    /// Replace the content of a bookmark in cache.
    async fn replace(&self, html: String, bookmark: &TargetBookmark) -> Result<(), anyhow::Error>;

    /// Remove the content of a bookmark from cache.
    async fn remove(&self, bookmark: &TargetBookmark) -> Result<(), anyhow::Error>;

    /// Remove the content of multiple bookmarks from cache.
    async fn remove_all(&self, bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error>;

    /// Clear the cache, i.e. remove all files in the cache directory.
    fn clear(&self) -> Result<(), anyhow::Error>;
}

/// The cache to store fetched bookmarks.
pub struct Cache {
    /// The path to the cache directory.
    pub path: PathBuf,
    /// The file extension of the cached files.
    pub mode: CacheMode,
}

impl Cache {
    /// Create new cache.
    pub fn new(cache_path: &Path, cache_mode: CacheMode) -> Self {
        Self {
            path: cache_path.to_owned(),
            mode: cache_mode,
        }
    }

    fn get_path(&self, bookmark_id: &str) -> PathBuf {
        self.path
            .join(bookmark_id)
            .with_extension(self.mode.extension())
    }
}

#[async_trait]
impl Caching for Cache {
    fn exists(&self, bookmark: &TargetBookmark) -> bool {
        let cache_path = self.get_path(&bookmark.id);
        cache_path.exists()
    }

    fn open(&self, bookmark: &TargetBookmark) -> Result<Option<File>, anyhow::Error> {
        let cache_path = self.get_path(&bookmark.id);
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
                self.get_path(&bookmark.id).display()
            );
            let mut buf = String::new();
            cache_file.read_to_string(&mut buf)?;
            Ok(Some(buf))
        } else {
            Ok(None)
        }
    }

    async fn add(&self, html: String, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        let cache_path = self.get_path(&bookmark.id);

        let website = match self.mode {
            CacheMode::Html => html,
            CacheMode::Text => html::convert_to_text(&html, &bookmark.url)?,
        };

        if !cache_path.exists() {
            debug!("Add website to cache: {}", cache_path.display());
            let mut cache_file = utils::create_file_async(&cache_path).await?;
            cache_file.write_all(website.as_bytes()).await?;
            cache_file.flush().await?;
        }

        Ok(())
    }

    async fn replace(&self, html: String, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        let cache_path = self.get_path(&bookmark.id);
        debug!("Replace website in cache: {}", cache_path.display());

        let website = match self.mode {
            CacheMode::Html => html,
            CacheMode::Text => html::convert_to_text(&html, &bookmark.url)?,
        };

        let mut cache_file = utils::create_file_async(&cache_path).await?;
        cache_file.write_all(website.as_bytes()).await?;
        cache_file.flush().await?;
        Ok(())
    }

    async fn remove(&self, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        let cache_file = self.get_path(&bookmark.id);

        if cache_file.exists() {
            debug!("Remove website from cache: {}", cache_file.display());
            fs::remove_file(cache_file).await?;
        }

        Ok(())
    }

    async fn remove_all(&self, bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error> {
        println!("Remove all");
        for bookmark in &bookmarks.bookmarks {
            let cache_file = self.get_path(&bookmark.id);

            if cache_file.exists() {
                debug!("Remove website from cache: {}", cache_file.display());
                fs::remove_file(cache_file).await?;
            }
        }

        Ok(())
    }

    fn clear(&self) -> Result<(), anyhow::Error> {
        let cache_path = &self.path;
        let entries = std::fs::read_dir(cache_path)?;

        for entry in entries {
            let entry = entry?;
            let file_path = entry.path();

            if let Some(extension) = file_path.extension() {
                if extension == "txt" || extension == "html" {
                    std::fs::remove_file(&file_path)?;
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
}

impl MockCache {
    pub fn new() -> Self {
        let cache_map = Mutex::new(HashMap::new());
        Self { cache_map }
    }

    pub fn cache_map(&self) -> HashMap<String, String> {
        let cache_map = self.cache_map.lock().unwrap();
        cache_map.clone()
    }
}

#[async_trait]
impl Caching for MockCache {
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

    async fn add(&self, html: String, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        let mut cache_map = self.cache_map.lock().unwrap();
        cache_map.insert(bookmark.id.clone(), html);
        Ok(())
    }

    async fn replace(&self, html: String, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        let mut cache_map = self.cache_map.lock().unwrap();
        cache_map.insert(bookmark.id.clone(), html);
        Ok(())
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

    fn clear(&self) -> Result<(), anyhow::Error> {
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
    async fn test_add() {
        let cache = MockCache::new();
        let now = Utc::now();
        let bookmark = TargetBookmark::new("url1", now, None);
        let content = "content".to_owned();
        cache.add(content, &bookmark).await.unwrap();
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 1);
    }

    #[tokio::test]
    async fn test_replace() {
        let cache = MockCache::new();
        let now = Utc::now();
        let bookmark = TargetBookmark::new("url1", now, None);
        let content = "content";
        cache.add(content.to_owned(), &bookmark).await.unwrap();
        cache.replace(content.to_owned(), &bookmark).await.unwrap();
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 1);
    }

    #[tokio::test]
    async fn test_remove() {
        let cache = MockCache::new();
        let now = Utc::now();
        let bookmark = TargetBookmark::new("url1", now, None);
        let content = "content";
        cache.add(content.to_owned(), &bookmark).await.unwrap();
        cache.remove(&bookmark).await.unwrap();
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 0);
    }

    #[tokio::test]
    async fn test_remove_all() {
        let cache = MockCache::new();
        let now = Utc::now();
        let bookmarks = vec![
            TargetBookmark::new("url1", now, None),
            TargetBookmark::new("url2", now, None),
        ];

        for bookmark in &bookmarks {
            cache.add("content".to_owned(), bookmark).await.unwrap();
        }

        let target_bookmarks = TargetBookmarks { bookmarks };

        cache.remove_all(&target_bookmarks).await.unwrap();
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 0);
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = MockCache::new();
        let now = Utc::now();
        let bookmark = TargetBookmark::new("url1", now, None);
        let content = "content";
        cache.add(content.to_owned(), &bookmark).await.unwrap();
        cache.clear().unwrap();
        let cache_map = cache.cache_map.lock().unwrap();
        assert_eq!(cache_map.keys().len(), 0);
    }
}
