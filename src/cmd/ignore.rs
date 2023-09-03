use crate::{utils, Config, IgnoreArgs};
use log::warn;
use reqwest::Url;
use std::io::Write;

/// Ignore the given urls and don't fetch and add these urls to the cache.
///
/// Ignored urls can be configured by `bogrep ignore <url>` or by adding them to
/// the `.bogrepignore` file.
pub fn ignore(config: &Config, ignore_args: IgnoreArgs) -> Result<(), anyhow::Error> {
    if !config.ignore_path.exists() {
        utils::create_file(&config.ignore_path)?;
    }

    let mut ignore_file = utils::append_file(&config.ignore_path)?;

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
