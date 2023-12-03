use crate::{
    bookmark_reader::SourceReader, bookmarks::RawSource, cache::CacheMode, json, utils, Config,
    ConfigArgs, Settings,
};
use anyhow::Context;
use log::debug;
use std::{fs, io::Write};

/// Configure the source files to import the bookmarks, the cache mode, or the
/// ignoure urls .
pub fn configure(mut config: Config, args: ConfigArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    let cache_mode = args.set_cache_mode.cache_mode;
    let source_path = args
        .set_source
        .source
        .map(|source_path| fs::canonicalize(source_path).context("Invalid source path"))
        .transpose()?;
    let source_folders = args.set_source.folders;
    let source = source_path.map(|source_path| RawSource::new(source_path, source_folders));

    if let Some(ref source) = source {
        // Validate source file
        SourceReader::init(source)?;
    }

    let settings_file = utils::open_file_in_read_write_mode(&config.settings_path)?;

    configure_settings(
        &mut config.settings,
        source,
        cache_mode,
        &args.set_ignored_urls.ignore,
        settings_file,
    )?;

    Ok(())
}

fn configure_settings(
    settings: &mut Settings,
    source: Option<RawSource>,
    cache_mode: Option<CacheMode>,
    urls: &[String],
    mut writer: impl Write,
) -> Result<(), anyhow::Error> {
    let settings_read = settings.clone();

    if let Some(source) = source {
        settings.set_source(source)?;
    }

    settings.set_cache_mode(cache_mode);

    if !urls.is_empty() {
        for url in urls {
            settings.add_url(url.to_string())?;
        }
    }

    if &settings_read != settings {
        let settings_json = json::serialize(settings)?;
        writer.write_all(&settings_json)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Cursor, path::PathBuf};

    #[test]
    fn test_configure_source() {
        let mut cursor = Cursor::new(Vec::new());
        let mut settings = Settings::default();
        let source = RawSource {
            path: PathBuf::from("test_data/bookmarks_simple.txt"),
            folders: vec!["dev".to_string(), "articles".to_string()],
        };
        let urls = vec![];
        let cache_mode = None;

        let res = configure_settings(&mut settings, Some(source), cache_mode, &urls, &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let actual_settings = String::from_utf8(cursor.into_inner()).unwrap();
        let expected_settings = r#"{
    "sources": [
        {
            "source": "test_data/bookmarks_simple.txt",
            "folders": [
                "dev",
                "articles"
            ]
        }
    ],
    "ignored_urls": [],
    "cache_mode": "text",
    "max_parallel_requests": 100,
    "request_timeout": 60000,
    "request_throttling": 3000,
    "max_idle_connections_per_host": 10,
    "idle_connections_timeout": 5000
}"#;
        assert_eq!(actual_settings, expected_settings);
    }

    #[test]
    fn test_configure_cache_mode() {
        let mut cursor = Cursor::new(Vec::new());
        let mut settings = Settings::default();
        let urls = vec![];
        let cache_mode = Some(CacheMode::Html);
        let res = configure_settings(&mut settings, None, cache_mode, &urls, &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let actual_settings = String::from_utf8(cursor.into_inner()).unwrap();
        let expected_settings = r#"{
    "sources": [],
    "ignored_urls": [],
    "cache_mode": "html",
    "max_parallel_requests": 100,
    "request_timeout": 60000,
    "request_throttling": 3000,
    "max_idle_connections_per_host": 10,
    "idle_connections_timeout": 5000
}"#;
        assert_eq!(actual_settings, expected_settings);
    }

    #[test]
    fn test_configure_ignored_urls() {
        let mut cursor = Cursor::new(Vec::new());
        let mut settings = Settings::default();
        let urls = vec![
            "https://test_url1.com".to_string(),
            "https://test_url2.com".to_string(),
        ];
        let res = configure_settings(&mut settings, None, None, &urls, &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let actual_settings = String::from_utf8(cursor.into_inner()).unwrap();

        let expected_settings = r#"{
    "sources": [],
    "ignored_urls": [
        "https://test_url1.com/",
        "https://test_url2.com/"
    ],
    "cache_mode": "text",
    "max_parallel_requests": 100,
    "request_timeout": 60000,
    "request_throttling": 3000,
    "max_idle_connections_per_host": 10,
    "idle_connections_timeout": 5000
}"#;
        assert_eq!(actual_settings, expected_settings);
    }
}
