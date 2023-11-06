use crate::{cache::CacheMode, json};
use anyhow::{anyhow, Context};
use log::debug;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// The default `Settungs::max_concurrent_requests`.
const MAX_CONCURRENT_REQUESTS_DEFAULT: usize = 100;

/// The default for `Settings::request_timeout`.
const REQUEST_TIMEOUT_DEFAULT: u64 = 60_000;

/// The default for `Settings::request_throttling`.
const REQUEST_THROTTLING_DEFAULT: u64 = 3_000;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Settings {
    /// The paths to the configured bookmark files.
    ///
    /// Source could be Firefox or Chrome.
    #[serde(rename = "bookmark_files")]
    pub sources: Vec<Source>,
    /// The urls which are ignored and not imported.
    pub ignored_urls: Vec<String>,
    /// The file extension used to cache websites.
    pub cache_mode: CacheMode,
    /// The maximal number of concurrent requests.
    pub max_concurrent_requests: usize,
    /// The request timeout in milliseconds.
    pub request_timeout: u64,
    /// The throttling between requests in milliseconds.
    pub request_throttling: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            ignored_urls: Vec::new(),
            cache_mode: CacheMode::default(),
            max_concurrent_requests: MAX_CONCURRENT_REQUESTS_DEFAULT,
            request_timeout: REQUEST_TIMEOUT_DEFAULT,
            request_throttling: REQUEST_THROTTLING_DEFAULT,
        }
    }
}

impl Settings {
    pub fn new(
        sources: Vec<Source>,
        ignored_urls: Vec<String>,
        cache_mode: CacheMode,
        max_concurrent_requests: usize,
        request_timeout: u64,
        request_throttling: u64,
    ) -> Self {
        Self {
            sources,
            ignored_urls,
            cache_mode,
            max_concurrent_requests,
            request_timeout,
            request_throttling,
        }
    }

    pub fn init(settings_path: &Path) -> Result<Settings, anyhow::Error> {
        if settings_path.exists() {
            debug!("Reading settings file at {}", settings_path.display());
            let mut buf = Vec::new();
            let mut settings_file = File::open(settings_path)?;
            settings_file
                .read_to_end(&mut buf)
                .context("Can't read `settings.json` file")?;
            let settings = json::deserialize::<Settings>(&buf)?;
            Ok(settings)
        } else {
            debug!("Create settings file at {}", settings_path.display());
            let settings = Settings::default();
            let settings_json = json::serialize(&settings)?;
            let mut settings_file = File::create(settings_path).context(format!(
                "Can't create `settings.json` file: {}",
                settings_path.display()
            ))?;
            settings_file.write_all(&settings_json)?;
            Ok(settings)
        }
    }

    pub fn add_url(&mut self, url: String) -> Result<(), anyhow::Error> {
        if !self.ignored_urls.contains(&url) {
            let url = Url::parse(&url).context(format!("Invalid url {url}"))?;
            self.ignored_urls.push(url.to_string());
        }

        Ok(())
    }

    pub fn set_source(&mut self, source: Source) -> Result<(), anyhow::Error> {
        debug!("Set source to {}", source.path.display());

        let source_paths = self
            .sources
            .iter()
            .map(|source| &source.path)
            .collect::<Vec<_>>();

        if !source_paths.contains(&&source.path) {
            self.sources.push(source);
            Ok(())
        } else {
            Err(anyhow!(format!(
                "Source already configured: {}",
                source.path.display()
            )))
        }
    }

    pub fn set_cache_mode(&mut self, cache_mode: Option<CacheMode>) {
        if let Some(cache_mode) = cache_mode {
            debug!("Set cache mode to {:#?}", cache_mode);
            self.cache_mode = cache_mode;
        }
    }

    pub fn configure(
        &mut self,
        source: Source,
        cache_mode: Option<CacheMode>,
    ) -> Result<(), anyhow::Error> {
        self.set_source(source)?;
        self.set_cache_mode(cache_mode);
        Ok(())
    }
}

/// The source of bookmarks.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Source {
    /// The path to the source file.
    #[serde(rename = "source")]
    pub path: PathBuf,
    /// The folders to be imported.
    ///
    /// If no folders are selected, all bookmarks in the source file will be
    /// imported.
    pub folders: Vec<String>,
}

impl Source {
    pub fn new(path: impl Into<PathBuf>, folders: Vec<String>) -> Self {
        Self {
            path: path.into(),
            folders,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_urls() {
        let mut settings = Settings::default();
        assert!(settings.ignored_urls.is_empty());
        let urls = vec![
            String::from("https://youtube.com/"),
            String::from("https://youtube.com/"),
            String::from("https://soundcloud.com/"),
        ];

        for url in urls {
            let res = settings.add_url(url);
            assert!(res.is_ok());
        }

        assert_eq!(
            settings.ignored_urls,
            vec![
                String::from("https://youtube.com/"),
                String::from("https://soundcloud.com/"),
            ]
        );
    }

    #[test]
    fn test_add_url_invalid() {
        let mut settings = Settings::default();
        let url = String::from("youtube.com/");
        let res = settings.add_url(url);
        assert!(res.is_err());
    }

    #[test]
    fn test_set_source() {
        let mut settings = Settings::default();
        settings
            .set_source(Source {
                path: PathBuf::from("path/to/source"),
                folders: vec![],
            })
            .unwrap();
        assert_eq!(
            settings,
            Settings {
                sources: vec![Source {
                    path: PathBuf::from("path/to/source"),
                    folders: vec![]
                }],
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_set_source_duplicate() {
        let mut settings = Settings::default();
        settings
            .set_source(Source {
                path: PathBuf::from("path/to/source"),
                folders: vec![],
            })
            .unwrap();
        assert_eq!(
            settings,
            Settings {
                sources: vec![Source {
                    path: PathBuf::from("path/to/source"),
                    folders: vec![]
                }],
                ..Default::default()
            }
        );

        let res = settings.set_source(Source {
            path: PathBuf::from("path/to/source"),
            folders: vec![],
        });
        assert!(res.is_err());
    }

    #[test]
    fn test_set_source_and_folders() {
        let mut settings = Settings::default();
        settings
            .set_source(Source {
                path: PathBuf::from("path/to/source"),
                folders: vec![String::from("dev,science,article")],
            })
            .unwrap();
        assert_eq!(
            settings,
            Settings {
                sources: vec![Source {
                    path: PathBuf::from("path/to/source"),
                    folders: vec![String::from("dev,science,article")]
                }],
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_set_cache_mode() {
        let mut settings = Settings::default();
        settings.set_cache_mode(Some(CacheMode::Html));
        assert_eq!(
            settings,
            Settings {
                cache_mode: CacheMode::Html,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_configure() {
        let mut settings = Settings::default();
        let source = Source {
            path: PathBuf::from("path/to/source"),
            folders: vec![String::from("dev,science,article")],
        };
        settings.configure(source, Some(CacheMode::Html)).unwrap();
        assert_eq!(
            settings,
            Settings {
                sources: vec![Source {
                    path: PathBuf::from("path/to/source"),
                    folders: vec![String::from("dev,science,article")]
                }],
                cache_mode: CacheMode::Html,
                ..Default::default()
            }
        );
    }
}
