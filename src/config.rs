use crate::{Args, Settings};
use anyhow::Context;
use log::info;
use std::{
    env,
    path::{Path, PathBuf},
};

const CONFIG_PATH: &str = ".config/bogrep";
const SETTINGS_FILE: &str = "settings.json";
const IGNORE_FILE: &str = ".bogrepignore";
const BOOKMARKS_FILE: &str = "bookmarks.json";

#[derive(Debug)]
pub struct Config {
    /// The log level of the program.
    pub verbosity: u8,
    /// The path of the settings file, usually ~/.config/bogrep/settings.json.
    pub settings_path: PathBuf,
    /// The path to the ignored urls, usually ~/.config/bogrep/.bogrepignore.
    pub ignore_path: PathBuf,
    /// The path to the cached websites, usually ~/.config/bogrep/cache.
    pub cache_path: PathBuf,
    /// The path to the generated bookmark file, usually ~/.config/bogrep/bookmarks.json.
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

    pub fn init(args: &Args) -> Result<Config, anyhow::Error> {
        let verbosity = args.verbose;
        let home_dir = env::var("HOME").context("HOME environment variable not set")?;
        let config_path = format!("{}/{}", home_dir, CONFIG_PATH);
        let config_path = Path::new(&config_path);
        let settings_path = format!("{}/{}/{}", home_dir, CONFIG_PATH, SETTINGS_FILE);
        let settings_path = Path::new(&settings_path);
        let ignore_path = format!("{}/{}/{}", home_dir, CONFIG_PATH, IGNORE_FILE);
        let ignore_path = Path::new(&ignore_path);
        let target_bookmark_file = format!("{}/{}/{}", home_dir, CONFIG_PATH, BOOKMARKS_FILE);
        let target_bookmark_file = Path::new(&target_bookmark_file);
        let cache_path = format!("{}/{}/cache", &home_dir, CONFIG_PATH);
        let cache_path = Path::new(&cache_path);

        let settings = Settings::init(config_path, settings_path)?;

        if verbosity >= 1 {
            info!("Read config from {}", settings_path.display());
        }

        let config = Config::new(
            verbosity,
            settings_path,
            ignore_path,
            cache_path,
            target_bookmark_file,
            settings,
        );

        Ok(config)
    }
}
