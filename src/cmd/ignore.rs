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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::{Cursor, Read},
    };

    #[test]
    fn test_ignore_urls() {
        let mut cursor = Cursor::new(Vec::new());
        let mut settings = Settings::default();
        let urls = vec![
            String::from("https://youtube.com/"),
            String::from("https://soundcloud.com/"),
        ];

        let res = ignore_urls(&mut settings, &urls, &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let actual_settings = String::from_utf8(cursor.into_inner()).unwrap();

        let mut expected_settings = String::new();
        let mut expected_file = File::open("test_data/ignore/settings.json").unwrap();
        expected_file
            .read_to_string(&mut expected_settings)
            .unwrap();
        assert_eq!(actual_settings, expected_settings);
    }
}
