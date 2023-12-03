use crate::{bookmarks::RawSource, cache::CacheMode, json};
use anyhow::Context;
use log::debug;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

/// The default `Settungs::max_parallel_requests`.
const MAX_PARALLEL_REQUESTS_DEFAULT: usize = 100;

/// The default for `Settings::request_timeout`.
const REQUEST_TIMEOUT_DEFAULT: u64 = 60_000;

/// The default for `Settings::request_throttling`.
const REQUEST_THROTTLING_DEFAULT: u64 = 3_000;

/// The  default for `Setting::max_idle_connections_per_host`.
const MAX_IDLE_CONNECTIONS_PER_HOST: usize = 10;

/// The  default for `Setting::idle_connections_timeout`.
const IDLE_CONNECTIONS_TIMEOUT: u64 = 5_000;

/// Describes the settings used in Bogrep.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Settings {
    /// The paths to the configured bookmark files.
    ///
    /// Source could be Firefox or Chrome.
    pub sources: Vec<RawSource>,
    /// The urls which are ignored and not imported.
    pub ignored_urls: Vec<String>,
    /// The file extension used to cache websites.
    pub cache_mode: CacheMode,
    /// The maximal number of concurrent requests.
    pub max_parallel_requests: usize,
    /// The request timeout in milliseconds.
    pub request_timeout: u64,
    /// The throttling between requests in milliseconds.
    pub request_throttling: u64,
    /// The maximum number of idle connections allowed in the connection pool.
    pub max_idle_connections_per_host: usize,
    /// The timeout for idle connections to be kept alive in milliseconds.
    pub idle_connections_timeout: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            ignored_urls: Vec::new(),
            cache_mode: CacheMode::default(),
            max_parallel_requests: MAX_PARALLEL_REQUESTS_DEFAULT,
            request_timeout: REQUEST_TIMEOUT_DEFAULT,
            request_throttling: REQUEST_THROTTLING_DEFAULT,
            max_idle_connections_per_host: MAX_IDLE_CONNECTIONS_PER_HOST,
            idle_connections_timeout: IDLE_CONNECTIONS_TIMEOUT,
        }
    }
}

impl Settings {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sources: Vec<RawSource>,
        ignored_urls: Vec<String>,
        cache_mode: CacheMode,
        max_parallel_requests: usize,
        request_timeout: u64,
        request_throttling: u64,
        max_idle_connections_per_host: usize,
        idle_connections_timeout: u64,
    ) -> Self {
        Self {
            sources,
            ignored_urls,
            cache_mode,
            max_parallel_requests,
            request_timeout,
            request_throttling,
            max_idle_connections_per_host,
            idle_connections_timeout,
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

    pub fn set_source(&mut self, source: RawSource) -> Result<(), anyhow::Error> {
        debug!("Set source to {}", source.path.display());

        if let Some(s) = self.sources.iter_mut().find(|s| s.path == source.path) {
            *s = source;
        } else {
            self.sources.push(source);
        }

        Ok(())
    }

    pub fn set_cache_mode(&mut self, cache_mode: Option<CacheMode>) {
        if let Some(cache_mode) = cache_mode {
            debug!("Set cache mode to {}", cache_mode);
            self.cache_mode = cache_mode;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
        let raw_source = RawSource::new(PathBuf::from("path/to/source"), vec![]);
        let mut settings = Settings::default();
        let res = settings.set_source(raw_source.clone());
        assert!(res.is_ok());
        assert_eq!(
            settings,
            Settings {
                sources: vec![raw_source],
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_set_source_overwrite() {
        let raw_source = RawSource::new(PathBuf::from("path/to/source"), vec![]);
        let raw_source_with_folders = RawSource::new(
            PathBuf::from("path/to/source"),
            vec!["dev".to_string(), "articles".to_string()],
        );
        let mut settings = Settings::default();
        settings.set_source(raw_source.clone()).unwrap();
        assert_eq!(
            settings,
            Settings {
                sources: vec![raw_source],
                ..Default::default()
            }
        );

        settings
            .set_source(raw_source_with_folders.clone())
            .unwrap();
        assert_eq!(
            settings,
            Settings {
                sources: vec![raw_source_with_folders],
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_set_source_and_folders() {
        let raw_source_with_folders = RawSource::new(
            PathBuf::from("path/to/source"),
            vec!["dev".to_string(), "articles".to_string()],
        );
        let mut settings = Settings::default();
        settings
            .set_source(raw_source_with_folders.clone())
            .unwrap();
        assert_eq!(
            settings,
            Settings {
                sources: vec![raw_source_with_folders],
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
}
