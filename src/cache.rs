use crate::{
    bookmarks::TargetBookmark,
    utils::{self, read_file},
    TargetBookmarks,
};
use anyhow::anyhow;
use async_trait::async_trait;
use clap::ValueEnum;
use log::{debug, warn};
use readability::extractor;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tokio::{fs, io::AsyncWriteExt};

#[derive(Debug, ValueEnum, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CacheMode {
    Html,
    Markdown,
    #[default]
    Text,
}

impl CacheMode {
    pub fn extension(&self) -> &str {
        match self {
            Self::Html => "html",
            Self::Markdown => "md",
            Self::Text => "txt",
        }
    }

    pub fn suffix(&self) -> &str {
        match self {
            Self::Html => ".html",
            Self::Markdown => ".md",
            Self::Text => ".txt",
        }
    }
}

#[async_trait]
pub trait Caching {
    fn get_path(&self, bookmark: &TargetBookmark) -> PathBuf;

    fn get(&self, bookmark: &TargetBookmark) -> Result<Option<String>, anyhow::Error>;

    async fn add(&self, html: String, bookmark: &TargetBookmark) -> Result<(), anyhow::Error>;

    async fn replace(&self, html: String, bookmark: &TargetBookmark) -> Result<(), anyhow::Error>;

    async fn remove(&self, bookmark: &TargetBookmark) -> Result<(), anyhow::Error>;

    async fn remove_all(&self, bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error>;
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
    pub fn new(cache_path: &Path, cache_mode: &Option<CacheMode>) -> Self {
        Self {
            path: cache_path.to_owned(),
            mode: cache_mode.clone().unwrap_or_default(),
        }
    }
}

#[async_trait]
impl Caching for Cache {
    /// Get the path of a cached website.
    fn get_path(&self, bookmark: &TargetBookmark) -> PathBuf {
        self.path
            .join(bookmark.id.clone())
            .with_extension(self.mode.extension())
    }

    /// Get a bookmark from cache.
    // TODO: make get async
    fn get(&self, bookmark: &TargetBookmark) -> Result<Option<String>, anyhow::Error> {
        let cache_path = &self.path;

        if cache_path.is_dir() {
            match std::fs::read_dir(cache_path) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        if let Some(file_name) = entry.file_name().to_str() {
                            if let Some(file_name) = file_name.strip_suffix(self.mode.suffix()) {
                                if file_name == bookmark.id {
                                    let bookmark_path = self.get_path(bookmark);
                                    debug!("Found website in cache: {}", bookmark_path.display());
                                    let file = read_file(&bookmark_path)?;
                                    return Ok(Some(String::from_utf8(file)?));
                                }
                            }
                        }
                    }

                    warn!("Can't find cached website for {}", bookmark.url);
                    Ok(None)
                }
                Err(err) => Err(anyhow!(
                    "Can't read directoy '{}': {}",
                    cache_path.display(),
                    err
                )),
            }
        } else {
            Err(anyhow!("Cache path is not a directory"))
        }
    }

    /// Add bookmark to cache.
    async fn add(&self, html: String, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        let cache_file = self.get_path(bookmark);

        let website = match self.mode {
            CacheMode::Html => html,
            CacheMode::Markdown => convert_to_markdown(&html),
            CacheMode::Text => convert_to_text(&html, &bookmark.url)?,
        };

        if !cache_file.exists() {
            debug!("Add website to cache: {}", cache_file.display());
            let mut file = utils::create_file_async(&cache_file).await?;
            file.write_all(website.as_bytes()).await?;
            file.flush().await?;
        }

        Ok(())
    }

    /// Replace bookmark in cache.
    async fn replace(&self, html: String, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        let cache_file = self.get_path(bookmark);
        debug!("Replace website in cache: {}", cache_file.display());

        let website = match self.mode {
            CacheMode::Html => html,
            CacheMode::Markdown => convert_to_markdown(&html),
            CacheMode::Text => convert_to_text(&html, &bookmark.url)?,
        };

        let mut file = utils::create_file_async(&cache_file).await?;
        file.write_all(website.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }

    /// Remove bookmark from cache.
    async fn remove(&self, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        let cache_file = self.get_path(bookmark);

        if cache_file.exists() {
            debug!("Remove website from cache: {}", cache_file.display());
            fs::remove_file(cache_file).await?;
        }

        Ok(())
    }

    /// Remove multiple bookmarks from cache.
    async fn remove_all(&self, bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error> {
        println!("Remove all");
        for bookmark in &bookmarks.bookmarks {
            let cache_file = self.get_path(bookmark);

            if cache_file.exists() {
                debug!("Remove website from cache: {}", cache_file.display());
                fs::remove_file(cache_file).await?;
            }
        }

        Ok(())
    }
}

fn convert_to_text(html: &str, bookmark_url: &str) -> Result<String, anyhow::Error> {
    let mut cursor = Cursor::new(html);
    let bookmark_url = Url::parse(bookmark_url)?;
    let product = extractor::extract(&mut cursor, &bookmark_url)?;
    Ok(product.text)
}

fn convert_to_markdown(html: &str) -> String {
    html2md::parse_html(html)
}

/// The cache to store fetched bookmarks.
pub struct MockCache {
    /// Mock the files system.
    cache_map: Mutex<HashMap<String, String>>,
    /// The file extension of the cached files.
    pub mode: CacheMode,
}

impl MockCache {
    pub fn new(mode: CacheMode) -> Self {
        let cache_map = Mutex::new(HashMap::new());
        Self { cache_map, mode }
    }
}

#[async_trait]
impl Caching for MockCache {
    fn get_path(&self, _bookmark: &TargetBookmark) -> PathBuf {
        todo!()
    }

    fn get(&self, _bookmark: &TargetBookmark) -> Result<Option<String>, anyhow::Error> {
        todo!()
    }

    async fn add(&self, html: String, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        let mut cache_map = self.cache_map.lock().unwrap();
        cache_map.insert(bookmark.id.clone(), html);
        Ok(())
    }

    async fn replace(
        &self,
        _html: String,
        _bookmark: &TargetBookmark,
    ) -> Result<(), anyhow::Error> {
        todo!()
    }

    async fn remove(&self, _bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        todo!()
    }

    async fn remove_all(&self, bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error> {
        let mut cache_map = self.cache_map.lock().unwrap();

        for bookmark in &bookmarks.bookmarks {
            cache_map.remove(&bookmark.id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_remove_all() {
        let cache = MockCache::new(CacheMode::Text);
        let now = Utc::now();
        let bookmark1 = TargetBookmark::new("url1", now, None);
        let bookmark2 = TargetBookmark::new("url2", now, None);
        let target_bookmarks = TargetBookmarks {
            bookmarks: vec![bookmark1.clone(), bookmark2.clone()],
        };
        cache.add("content1".to_owned(), &bookmark1).await.unwrap();
        cache.add("content2".to_owned(), &bookmark2).await.unwrap();
        {
            let cache_map = cache.cache_map.lock().unwrap();
            assert_eq!(cache_map.keys().len(), 2);
        }

        cache.remove_all(&target_bookmarks).await.unwrap();
        {
            let cache_map = cache.cache_map.lock().unwrap();
            assert_eq!(cache_map.keys().len(), 0);
        }
    }

    #[test]
    fn test_convert_to_text() {
        let html = r#"
        <html>

        <head>
            <title>title_content</title>
            <meta>
        </head>

        <body>
            <div>
                <p>paragraph_content_1</p>
                <div>
                    <p>paragraph_content_2</p>
                </div>
            </div>
        </body>

        </html>
        "#;
        let url = "https://example.net";
        let res = convert_to_text(html, url);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let text = res.unwrap();
        // TODO: fix line breaks
        // TODO: fix missing "paragraph_content_2"
        assert_eq!(text, "title_contentparagraph_content_1");
    }

    #[test]
    fn test_convert_to_markdown() {
        let html = r#"
        <html>

        <head>
            <title>title_content</title>
            <meta>
        </head>

        <body>
            <div>
                <p>paragraph_content_1</p>
                <div>
                    <p>paragraph_content_2</p>
                </div>
            </div>
        </body>

        </html>
        "#;
        let expected_markdown = " title_content\n\nparagraph_content_1\n\nparagraph_content_2";

        let markdown = convert_to_markdown(&html);
        // TODO: fix superfluous backslashes
        assert_eq!(markdown.replace("\\", ""), expected_markdown);
    }
}
