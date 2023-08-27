use crate::{bookmarks::TargetBookmark, utils::read_file, Config, TargetBookmarks};
use anyhow::anyhow;
use clap::ValueEnum;
use log::{debug, warn};
use readability::extractor;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    io::Cursor,
    path::{Path, PathBuf},
};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};

#[derive(Debug, ValueEnum, Clone, Serialize, Deserialize, Default)]
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

/// The cache to store fetched bookmarks.
pub struct Cache {
    /// The path to the cache directory.
    pub path: PathBuf,
    /// The file extension of the cached files.
    pub mode: CacheMode,
}

impl Cache {
    /// Create new cache.
    pub fn new(cache_path: &Path, cache_mode: &Option<CacheMode>) -> Result<Self, anyhow::Error> {
        if cache_path.exists() {
            Ok(Self {
                path: cache_path.to_owned(),
                mode: cache_mode.clone().unwrap_or_default(),
            })
        } else {
            Err(anyhow!("Missing cache, run `bogrep init` first"))
        }
    }

    /// Init cache.
    ///
    /// Creates cache directory, usually at ~/.config/bogrep/cache.
    pub async fn init(
        config: &Config,
        cache_mode: &Option<CacheMode>,
    ) -> Result<Cache, anyhow::Error> {
        let cache_path = &config.cache_path;

        let cache = if cache_path.exists() {
            Cache::new(cache_path, cache_mode)
        } else {
            debug!("Create cache at {}", cache_path.display());
            fs::create_dir_all(&cache_path).await?;
            Cache::new(cache_path, cache_mode)
        }?;

        Ok(cache)
    }

    /// Get the path of a cached website.
    pub fn get_path(&self, bookmark: &TargetBookmark) -> PathBuf {
        self.path
            .join(bookmark.id.clone())
            .with_extension(self.mode.extension())
    }

    /// Get a bookmark from cache.
    // TODO: make get async
    pub fn get(&self, bookmark: &TargetBookmark) -> Result<Option<String>, anyhow::Error> {
        let cache_path = &self.path;

        if cache_path.is_dir() {
            match std::fs::read_dir(&cache_path) {
                Ok(entries) => {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            if let Some(file_name) = entry.file_name().to_str() {
                                if let Some(file_name) = file_name.strip_suffix(self.mode.suffix())
                                {
                                    if file_name == bookmark.id {
                                        let bookmark_path = self.get_path(bookmark);
                                        debug!(
                                            "Found website in cache: {}",
                                            bookmark_path.display()
                                        );
                                        let file = read_file(&bookmark_path)?;
                                        return Ok(Some(String::from_utf8(file)?));
                                    }
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
    pub async fn add(&self, html: String, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        let cache_file = self.get_path(bookmark);

        let website = match self.mode {
            CacheMode::Html => html,
            CacheMode::Markdown => convert_to_markdown(&html),
            CacheMode::Text => convert_to_text(&html, &bookmark.url)?,
        };

        if !cache_file.exists() {
            debug!("Add website to cache: {}", cache_file.display());
            let mut file = File::create(&cache_file).await?;
            file.write_all(website.as_bytes()).await?;
            file.flush().await?;
        }

        Ok(())
    }

    /// Replace bookmark in cache.
    pub async fn replace(
        &self,
        html: String,
        bookmark: &TargetBookmark,
    ) -> Result<(), anyhow::Error> {
        let cache_file = self.get_path(bookmark);
        debug!("Replace website in cache: {}", cache_file.display());

        let website = match self.mode {
            CacheMode::Html => html,
            CacheMode::Markdown => convert_to_markdown(&html),
            CacheMode::Text => convert_to_text(&html, &bookmark.url)?,
        };

        let mut file = File::create(&cache_file).await?;
        file.write_all(website.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }

    /// Remove bookmark from cache.
    pub async fn remove(&self, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        let cache_file = self.get_path(bookmark);

        if cache_file.exists() {
            debug!("Remove website from cache: {}", cache_file.display());
            fs::remove_file(cache_file).await?;
        }

        Ok(())
    }

    /// Remove multiple bookmarks from cache.
    pub async fn remove_all(&self, bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error> {
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
    let bookmark_url = Url::parse(&bookmark_url)?;
    let product = extractor::extract(&mut cursor, &bookmark_url)?;
    Ok(product.text)
}

fn convert_to_markdown(html: &str) -> String {
    html2md::parse_html(html)
}

#[cfg(test)]
mod tests {
    use super::*;

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
