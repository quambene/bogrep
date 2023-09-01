use crate::cache::CacheMode;
use anyhow::Context;
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

#[derive(Debug, Serialize, Deserialize)]
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
            fs::create_dir_all(config_path).context("Can't create config directory: {}")?;
            let settings = Settings::default();
            settings.write(settings_path)?;
            Ok(settings)
        }
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

    pub fn write(&self, settings_path: &Path) -> Result<(), anyhow::Error> {
        let json = serde_json::to_string_pretty(&self)?;
        let mut settings_file = File::create(settings_path).context(format!(
            "Can't create settings file: {}",
            settings_path.display()
        ))?;
        settings_file.write_all(json.as_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
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
