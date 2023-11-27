use crate::{json, BookmarksJson, Settings};
use anyhow::{anyhow, Context};
use log::{debug, trace};
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

const CONFIG_DIR: &str = "bogrep";
const SETTINGS_FILE: &str = "settings.json";
const BOOKMARKS_FILE: &str = "bookmarks.json";
const BOOKMARKS_LOCK_FILE: &str = "bookmarks-lock.json";
const CACHE_DIR: &str = "cache";

/// A configuration for running Bogrep.
#[derive(Debug, PartialEq, Default)]
pub struct Config {
    /// The path of the settings file.
    pub settings_path: PathBuf,
    /// The path to the cached websites.
    pub cache_path: PathBuf,
    /// The path to the generated bookmark file.
    pub target_bookmark_file: PathBuf,
    /// The path to the lock file to write bookmarks.
    pub target_bookmark_lock_file: PathBuf,
    /// The configured settings.
    pub settings: Settings,
}

impl Config {
    fn new(
        settings_path: &Path,
        cache_path: &Path,
        target_bookmark_file: &Path,
        target_bookmark_lock_file: &Path,
        settings: Settings,
    ) -> Self {
        Self {
            settings_path: settings_path.to_owned(),
            cache_path: cache_path.to_owned(),
            target_bookmark_file: target_bookmark_file.to_owned(),
            target_bookmark_lock_file: target_bookmark_lock_file.to_owned(),
            settings,
        }
    }

    pub fn init() -> Result<Config, anyhow::Error> {
        let config_path = if let Ok(bogreg_home) = env::var("BOGREP_HOME") {
            PathBuf::from(bogreg_home)
        } else if let Some(config_path) = dirs::config_dir() {
            config_path.join(CONFIG_DIR)
        } else {
            return Err(anyhow!("HOME environment variable not set"));
        };
        let settings_path = config_path.join(SETTINGS_FILE);
        let target_bookmark_path = config_path.join(BOOKMARKS_FILE);
        let target_bookmark_lock_path = config_path.join(BOOKMARKS_LOCK_FILE);
        let cache_path = config_path.join(CACHE_DIR);

        if !config_path.exists() {
            debug!("Create config at {}", config_path.display());
            fs::create_dir_all(&config_path).context(format!(
                "Can't create config directory: {}",
                config_path.display()
            ))?;
        }

        let settings = Settings::init(&settings_path)?;

        if !target_bookmark_path.exists() {
            debug!(
                "Create bookmarks file at {}",
                target_bookmark_path.display()
            );
            let bookmarks_json = BookmarksJson::default();
            let buf = json::serialize(bookmarks_json)?;
            let mut bookmark_file = File::create(&target_bookmark_path).context(format!(
                "Can't create `bookmarks.json` file: {}",
                target_bookmark_path.display()
            ))?;
            bookmark_file.write_all(&buf)?;
            bookmark_file.flush()?;
        }

        if !cache_path.exists() {
            debug!("Create cache at {}", cache_path.display());
            fs::create_dir_all(&cache_path).context(format!(
                "Can't create cache directory at {}",
                cache_path.display()
            ))?;
        }

        debug!("Reading config from {}", settings_path.display());

        let config = Config::new(
            &settings_path,
            &cache_path,
            &target_bookmark_path,
            &target_bookmark_lock_path,
            settings,
        );

        trace!("Config: {:#?}", config);

        Ok(config)
    }
}
