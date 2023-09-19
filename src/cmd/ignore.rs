use crate::{json, utils, Config, IgnoreArgs, Settings};
use std::io::Write;

/// Ignore the given urls and don't fetch and add these urls to the cache.
///
/// Ignored urls can be configured by `bogrep ignore <url>` or by adding them to
/// the `.bogrepignore` file.
pub fn ignore(config: &Config, ignore_args: IgnoreArgs) -> Result<(), anyhow::Error> {
    let mut settings = config.settings.clone();
    let urls = ignore_args.urls;
    let mut settings_file = utils::open_file_in_read_write_mode(&config.settings_path)?;
    ignore_urls(&mut settings, &urls, &mut settings_file)?;

    Ok(())
}

fn ignore_urls(
    settings: &mut Settings,
    urls: &[String],
    writer: &mut impl Write,
) -> Result<(), anyhow::Error> {
    for url in urls {
        settings.add_url(url.to_string())?;
    }

    let settings_json = json::serialize(settings)?;
    writer.write_all(&settings_json)?;

    Ok(())
}
