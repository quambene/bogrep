use crate::{json, Settings, TargetBookmarks};
use anyhow::{anyhow, Context};
use log::{debug, info};
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

const CONFIG_PATH: &str = "bogrep";
const SETTINGS_FILE: &str = "settings.json";
const BOOKMARKS_FILE: &str = "bookmarks.json";
const CACHE_DIR: &str = "cache";

#[derive(Debug, PartialEq)]
pub struct Config {
    /// The log level of the program.
    pub verbosity: u8,
    /// The path of the settings file.
    pub settings_path: PathBuf,
    /// The path to the cached websites.
    pub cache_path: PathBuf,
    /// The path to the generated bookmark file.
    pub target_bookmark_file: PathBuf,
    /// The configured settings.
    pub settings: Settings,
}

impl Config {
    fn new(
        verbosity: u8,
        settings_path: &Path,
        cache_path: &Path,
        target_bookmark_file: &Path,
        settings: Settings,
    ) -> Self {
        Self {
            verbosity,
            settings_path: settings_path.to_owned(),
            cache_path: cache_path.to_owned(),
            target_bookmark_file: target_bookmark_file.to_owned(),
            settings,
        }
    }

    pub fn init(verbosity: u8) -> Result<Config, anyhow::Error> {
        let config_path = if let Ok(bogreg_home) = env::var("BOGREP_HOME") {
            PathBuf::from(bogreg_home)
        } else if let Some(config_path) = dirs::config_dir() {
            config_path.join(CONFIG_PATH)
        } else {
            return Err(anyhow!("HOME environment variable not set"));
        };
        let config_path = Path::new(&config_path);
        let settings_path = config_path.join(SETTINGS_FILE);
        let target_bookmark_path = config_path.join(BOOKMARKS_FILE);
        let cache_path = config_path.join(CACHE_DIR);

        if !config_path.exists() {
            debug!("Create config at {}", config_path.display());
            fs::create_dir_all(config_path).context(format!(
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
            let target_bookmarks = TargetBookmarks::default();
            let json = json::serialize(target_bookmarks)?;
            let mut bookmark_file = File::create(&target_bookmark_path).context(format!(
                "Can't create `bookmarks.json` file: {}",
                target_bookmark_path.display()
            ))?;
            bookmark_file.write_all(&json)?;
            bookmark_file.flush()?;
        }

        if !cache_path.exists() {
            debug!("Create cache at {}", cache_path.display());
            fs::create_dir_all(&cache_path).context(format!(
                "Can't create cache directory at {}",
                cache_path.display()
            ))?;
        }

        if verbosity >= 1 {
            info!("Read config from {}", settings_path.display());
        }

        let config = Config::new(
            verbosity,
            &settings_path,
            &cache_path,
            &target_bookmark_path,
            settings,
        );

        if verbosity >= 1 {
            info!("Config: {:#?}", config);
        }

        Ok(config)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    impl Default for Config {
        fn default() -> Self {
            Self {
                verbosity: u8::default(),
                settings_path: PathBuf::default(),
                cache_path: PathBuf::default(),
                target_bookmark_file: PathBuf::default(),
                settings: Settings::default(),
            }
        }
    }
}
