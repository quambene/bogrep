use crate::{utils, Config, IgnoreArgs};
use log::warn;
use reqwest::Url;
use std::io::Write;

/// Ignore the given urls and don't fetch and add these urls to the cache.
///
/// Ignored urls can be configured by `bogrep ignore <url>` or by adding them to
/// the `.bogrepignore` file.
pub fn ignore(config: &Config, ignore_args: IgnoreArgs) -> Result<(), anyhow::Error> {
    let mut ignore_file = utils::append_file(&config.ignore_path)?;

    ignore_urls(&mut ignore_file, &ignore_args.urls)?;

    Ok(())
}

fn ignore_urls(writer: &mut impl Write, urls: &[String]) -> Result<(), anyhow::Error> {
    for url in urls {
        match Url::parse(&url) {
            Ok(url) => {
                writeln!(writer, "{url}")?;
            }
            Err(err) => {
                warn!("Invalid url {url} is ignored: {err}");
            }
        }
    }

    Ok(())
}
