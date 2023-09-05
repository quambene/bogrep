use crate::{
    args::{SetCacheMode, SetSource},
    cache::CacheMode,
    json, utils,
};
use anyhow::Context;
use log::debug;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
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
    pub source_bookmark_files: Vec<SourceFile>,
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
            source_bookmark_files: Vec::new(),
            cache_mode: CacheMode::default(),
            max_concurrent_requests: MAX_CONCURRENT_REQUESTS_DEFAULT,
            request_timeout: REQUEST_TIMEOUT_DEFAULT,
            request_throttling: REQUEST_THROTTLING_DEFAULT,
        }
    }
}

impl Settings {
    pub fn new(
        source_bookmark_files: Vec<SourceFile>,
        cache_mode: CacheMode,
        max_concurrent_requests: usize,
        request_timeout: u64,
        request_throttling: u64,
    ) -> Self {
        Self {
            source_bookmark_files,
            cache_mode,
            max_concurrent_requests,
            request_timeout,
            request_throttling,
        }
    }

    pub fn init(config_path: &Path, settings_path: &Path) -> Result<Settings, anyhow::Error> {
        if settings_path.exists() {
            let settings = Settings::read(settings_path)?;
            Ok(settings)
        } else {
            fs::create_dir_all(config_path).context("Can't create config directory at {}")?;
            let settings = Settings::default();
            let settings_json = json::serialize(&settings)?;
            let mut settings_file =
                utils::create_file(settings_path).context("Can't create settings file")?;
            settings_file.write_all(&settings_json)?;
            Ok(settings)
        }
    }

    pub fn set_source(&mut self, set_source: SetSource) {
        if let Some(source_path) = set_source.source {
            debug!("Set source to {source_path}");
            let source_path = PathBuf::from(source_path);
            let source_file = SourceFile::new(source_path, set_source.folders);
            self.source_bookmark_files.push(source_file);
        }
    }

    pub fn set_cache_mode(&mut self, set_cache_mode: SetCacheMode) {
        if let Some(cache_mode) = set_cache_mode.cache_mode {
            debug!("Set cache mode to {:#?}", cache_mode);
            self.cache_mode = cache_mode;
        }
    }

    pub fn configure(&mut self, set_source: SetSource, set_cache_mode: SetCacheMode) {
        self.set_source(set_source);
        self.set_cache_mode(set_cache_mode);
    }

    fn read(settings_path: &Path) -> Result<Settings, anyhow::Error> {
        let mut buffer = String::new();
        let mut settings_file = File::open(settings_path)
            .context("Missing settings file: Run `bogrep config <my_bookmarks_file>`")?;
        settings_file
            .read_to_string(&mut buffer)
            .context("Can't read settings file")?;
        let settings = serde_json::from_str(&buffer)?;
        Ok(settings)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SourceFile {
    /// The source file for bookmarks.
    pub source: PathBuf,
    /// The folders to be imported.
    ///
    /// If no folders are selected, all bookmarks in the source file will be
    /// imported.
    pub folders: Vec<String>,
}

impl SourceFile {
    pub fn new(source: impl Into<PathBuf>, folders: Vec<String>) -> Self {
        Self {
            source: source.into(),
            folders,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_source() {
        let mut settings = Settings::default();
        settings.set_source(SetSource {
            source: Some(String::from("path/to/source")),
            folders: vec![String::from("dev,science,article")],
        });
        assert_eq!(
            settings,
            Settings {
                source_bookmark_files: vec![SourceFile {
                    source: PathBuf::from("path/to/source"),
                    folders: vec![String::from("dev,science,article")]
                }],
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_set_cache_mode() {
        let mut settings = Settings::default();
        settings.set_cache_mode(SetCacheMode {
            cache_mode: Some(CacheMode::Html),
        });
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
        let set_source = SetSource {
            source: Some(String::from("path/to/source")),
            folders: vec![String::from("dev,science,article")],
        };
        let set_cache_mode = SetCacheMode {
            cache_mode: Some(CacheMode::Html),
        };
        settings.configure(set_source, set_cache_mode);
        assert_eq!(
            settings,
            Settings {
                source_bookmark_files: vec![SourceFile {
                    source: PathBuf::from("path/to/source"),
                    folders: vec![String::from("dev,science,article")]
                }],
                cache_mode: CacheMode::Html,
                ..Default::default()
            }
        );
    }
}
