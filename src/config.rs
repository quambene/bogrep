use crate::{json, Settings, TargetBookmarks};
use anyhow::Context;
use log::{debug, info};
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

const CONFIG_PATH: &str = ".config/bogrep";
const SETTINGS_FILE: &str = "settings.json";
const IGNORE_FILE: &str = ".bogrepignore";
const BOOKMARKS_FILE: &str = "bookmarks.json";

#[derive(Debug, PartialEq)]
pub struct Config {
    /// The log level of the program.
    pub verbosity: u8,
    /// The path of the settings file.
    pub settings_path: PathBuf,
    /// The path to the ignored urls.
    pub ignore_path: PathBuf,
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
        ignore_path: &Path,
        cache_path: &Path,
        target_bookmark_file: &Path,
        settings: Settings,
    ) -> Self {
        Self {
            verbosity,
            settings_path: settings_path.to_owned(),
            ignore_path: ignore_path.to_owned(),
            cache_path: cache_path.to_owned(),
            target_bookmark_file: target_bookmark_file.to_owned(),
            settings,
        }
    }

    pub fn init(verbosity: u8) -> Result<Config, anyhow::Error> {
        let home_dir = env::var("HOME").context("HOME environment variable not set")?;
        let config_path = if let Ok(bogreg_home) = env::var("BOGREP_HOME") {
            bogreg_home
        } else {
            format!("{}/{}", home_dir, CONFIG_PATH)
        };

        let settings_path = format!("{}/{}", config_path, SETTINGS_FILE);
        let settings_path = Path::new(&settings_path);
        let ignore_path = format!("{}/{}", config_path, IGNORE_FILE);
        let ignore_path = Path::new(&ignore_path);
        let target_bookmark_path = format!("{}/{}", config_path, BOOKMARKS_FILE);
        let target_bookmark_path = Path::new(&target_bookmark_path);
        let cache_path = format!("{config_path}/cache");
        let cache_path = Path::new(&cache_path);
        let config_path = Path::new(&config_path);

        if !config_path.exists() {
            debug!("Create config at {}", config_path.display());
            fs::create_dir_all(config_path).context(format!(
                "Can't create config directory: {}",
                config_path.display()
            ))?;
        }

        let settings = Settings::init(settings_path)?;

        if !target_bookmark_path.exists() {
            debug!(
                "Create bookmarks file at {}",
                target_bookmark_path.display()
            );
            let target_bookmarks = TargetBookmarks::default();
            let json = json::serialize(target_bookmarks)?;
            let mut bookmark_file = File::create(target_bookmark_path).context(format!(
                "Can't create `bookmarks.json` file: {}",
                target_bookmark_path.display()
            ))?;
            bookmark_file.write_all(&json)?;
            bookmark_file.flush()?;
        }

        if !cache_path.exists() {
            debug!("Create cache at {}", cache_path.display());
            fs::create_dir_all(cache_path).context(format!(
                "Can't create cache directory at {}",
                cache_path.display()
            ))?;
        }

        if !ignore_path.exists() {
            debug!("Create ignore file at {}", ignore_path.display());
            File::create(ignore_path).context(format!(
                "Can't create `.bogrepignore` file: {}",
                settings_path.display()
            ))?;
        }

        if verbosity >= 1 {
            info!("Read config from {}", settings_path.display());
        }

        let config = Config::new(
            verbosity,
            settings_path,
            ignore_path,
            cache_path,
            target_bookmark_path,
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
                ignore_path: PathBuf::default(),
                cache_path: PathBuf::default(),
                target_bookmark_file: PathBuf::default(),
                settings: Settings::default(),
            }
        }
    }

    #[test]
    fn test_config() {
        let verbosity = 0;
        let project_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
        let config_path = format!("{}/test_data/bogrep", project_dir.to_string_lossy());

        // Prepare test
        env::set_var("BOGREP_HOME", &config_path);

        let res = Config::init(verbosity);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let config = res.unwrap();
        assert_eq!(
            config,
            Config {
                verbosity: 0,
                settings_path: PathBuf::from(format!("{config_path}/settings.json")),
                ignore_path: PathBuf::from(format!("{config_path}/.bogrepignore")),
                cache_path: PathBuf::from(format!("{config_path}/cache")),
                target_bookmark_file: PathBuf::from(format!("{config_path}/bookmarks.json")),
                settings: Settings::default()
            }
        );
    }
}
