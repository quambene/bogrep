use crate::{
    bookmark_reader::{BookmarkReaders, SourceReader},
    cache::CacheMode,
    json, utils, Config, ConfigArgs, Settings, Source,
};
use anyhow::Context;
use log::info;
use std::{fs, io::Write};

/// Configure the source files to import the bookmarks, the cache mode, or the
/// ignoure urls .
pub fn configure(mut config: Config, args: ConfigArgs) -> Result<(), anyhow::Error> {
    if config.verbosity >= 1 {
        info!("{args:?}");
    }

    let cache_mode = args.set_cache_mode.cache_mode;
    let source_path = args
        .set_source
        .source
        .map(|source_path| fs::canonicalize(source_path).context("Invalid source path"))
        .transpose()?;
    let source_folders = args.set_source.folders;
    let source = source_path.map(|source_path| Source::new(source_path, source_folders));
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
    source: Option<Source>,
    cache_mode: Option<CacheMode>,
    urls: &[String],
    mut writer: impl Write,
) -> Result<(), anyhow::Error> {
    let settings_read = settings.clone();
    let bookmark_readers = BookmarkReaders::new();

    if let Some(source) = source {
        // Validate source file
        let _ = SourceReader::select_reader(&source.path, bookmark_readers)?;
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
        let source = Source {
            path: PathBuf::from("test_data/bookmarks_simple.txt"),
            folders: vec!["dev".to_string(), "articles".to_string()],
        };
        let urls = vec![];
        let cache_mode = None;
        let res = configure_settings(&mut settings, Some(source), cache_mode, &urls, &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let actual_settings = String::from_utf8(cursor.into_inner()).unwrap();
        let expected_settings = r#"{
    "bookmark_files": [
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
    "max_concurrent_requests": 100,
    "request_timeout": 60000,
    "request_throttling": 3000
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
    "bookmark_files": [],
    "ignored_urls": [],
    "cache_mode": "html",
    "max_concurrent_requests": 100,
    "request_timeout": 60000,
    "request_throttling": 3000
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
    "bookmark_files": [],
    "ignored_urls": [
        "https://test_url1.com/",
        "https://test_url2.com/"
    ],
    "cache_mode": "text",
    "max_concurrent_requests": 100,
    "request_timeout": 60000,
    "request_throttling": 3000
}"#;
        assert_eq!(actual_settings, expected_settings);
    }
}
