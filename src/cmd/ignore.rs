use crate::{Config, IgnoreArgs};
use log::warn;
use reqwest::Url;
use std::{
    fs::{File, OpenOptions},
    io::Write,
};

/// Ignore the given urls and don't fetch and add these urls to the cache.
///
/// Ignored urls can be configured by `bogrep ignore <url>` or by adding them to
/// the `.bogrepignore` file.
pub fn ignore(config: &Config, ignore_args: IgnoreArgs) -> Result<(), anyhow::Error> {
    if !config.ignore_path.exists() {
        File::create(&config.ignore_path)?;
    }

    let mut ignore_file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(&config.ignore_path)?;

    for url in ignore_args.urls {
        match Url::parse(&url) {
            Ok(url) => {
                writeln!(ignore_file, "{url}")?;
            }
            Err(err) => {
                warn!("Invalid url {url} is ignored: {err}");
            }
        }
    }

    Ok(())
}
