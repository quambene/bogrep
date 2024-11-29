use crate::{
    bookmark_reader::SourceReader, bookmarks::RawSource, cmd::init_sources, config, json,
    settings::SettingsArgs, utils, Config, ConfigArgs, Settings,
};
use anyhow::{anyhow, Context};
use log::{debug, warn};
use std::{fs, io::Write};

/// Configure the source files to import the bookmarks, the cache mode, or the
/// ignoure urls .
pub fn configure(config: Config, args: ConfigArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    if args.dry_run {
        println!("Running in dry mode ...")
    }

    let mut config = config;
    let home_dir = dirs::home_dir().ok_or(anyhow!("Missing home dir"))?;

    if config.settings.sources.is_empty() {
        if let Some(source_os) = utils::get_supported_os() {
            init_sources(&mut config.settings, &home_dir, &source_os)?;

            if !args.dry_run {
                let mut settings_file = utils::open_and_truncate_file(&config.settings_path)?;
                let settings_json = json::serialize(config.settings.clone())?;
                settings_file.write_all(&settings_json)?;
                settings_file.flush()?;
            }
        }
    }

    let source_path = args
        .set_source
        .source
        .as_ref()
        .map(|source_path| fs::canonicalize(source_path).context("Invalid source path"))
        .transpose()?;
    let source_folders = &args.set_source.folders;
    let source = source_path.map(|source_path| RawSource::new(source_path, source_folders.clone()));

    if let Some(ref source) = source {
        // Validate source file
        SourceReader::init(source)?;
    }

    let settings_file = utils::open_and_truncate_file(&config.settings_path)?;

    let settings_args = SettingsArgs::new(
        source,
        args.set_ignored_urls.ignore,
        args.set_underlying_urls.underlying,
        args.set_cache_mode.cache_mode,
        args.set_max_open_files.max_open_files,
        args.set_max_concurrent_requests.max_concurrent_requests,
        args.set_request_timeout.request_timeout,
        args.set_request_throttling.request_throttling,
        args.set_max_idle_connections_per_host
            .max_idle_connections_per_host,
        args.set_idle_connections_timeout.idle_connections_timeout,
    );

    configure_settings(&mut config.settings, &settings_args, settings_file)?;

    Ok(())
}

fn configure_settings(
    settings: &mut Settings,
    settings_args: &SettingsArgs,
    mut writer: impl Write,
) -> Result<(), anyhow::Error> {
    if let Some(source) = &settings_args.source {
        settings.set_source(source.clone())?;
    }

    if let Some(cache_mode) = &settings_args.cache_mode {
        settings.set_cache_mode(cache_mode.clone());
    }

    if let Some(max_open_files) = settings_args.max_open_files {
        settings.set_max_open_files(max_open_files);
    }

    if let Some(request_timeout) = settings_args.request_timeout {
        settings.set_request_timeout(request_timeout);
    }

    if let Some(request_throttling) = settings_args.request_throttling {
        settings.set_request_throttling(request_throttling);
    }

    if let Some(max_concurrent_requests) = settings_args.max_concurrent_requests {
        settings.set_max_concurrent_requests(max_concurrent_requests);
    }

    if let Some(max_idle_connections_per_host) = settings_args.max_idle_connections_per_host {
        settings.set_max_idle_connections_per_host(max_idle_connections_per_host);
    }

    if let Some(idle_connections_timeout) = settings_args.idle_connections_timeout {
        settings.set_idle_connections_timeout(idle_connections_timeout);
    }

    if settings_args.max_open_files.is_some() && settings_args.max_concurrent_requests.is_some() {
        #[cfg(not(any(target_os = "windows")))]
        config::set_file_descriptor_limit(
            settings.max_open_files + settings.max_concurrent_requests as u64,
        )?;
    }

    for ignored_url in settings_args.ignored_urls.as_slice() {
        if let Err(err) = settings.add_ignored_url(ignored_url) {
            warn!("{err}");
        }
    }

    for underlying_url in settings_args.underlying_urls.as_slice() {
        if let Err(err) = settings.add_underlying_url(underlying_url) {
            warn!("{err}");
        };
    }

    let settings_json = json::serialize(settings)?;
    writer.write_all(&settings_json)?;
    writer.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CacheMode;
    use std::{io::Cursor, path::PathBuf};

    #[test]
    fn test_configure_source() {
        let mut cursor = Cursor::new(Vec::new());
        let mut settings = Settings::default();
        let settings_args = SettingsArgs {
            source: Some(RawSource {
                path: PathBuf::from("test_data/bookmarks_simple.txt"),
                folders: vec!["dev".to_string(), "articles".to_string()],
            }),
            ..Default::default()
        };

        let res = configure_settings(&mut settings, &settings_args, &mut cursor);
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
    "underlying_urls": [],
    "cache_mode": "text",
    "max_open_files": 500,
    "max_concurrent_requests": 500,
    "request_timeout": 60000,
    "request_throttling": 3000,
    "max_idle_connections_per_host": 1,
    "idle_connections_timeout": 5000
}"#;
        assert_eq!(actual_settings, expected_settings);
    }

    #[test]
    fn test_configure_cache_mode() {
        let mut cursor = Cursor::new(Vec::new());
        let mut settings = Settings::default();
        let settings_args = SettingsArgs {
            cache_mode: Some(CacheMode::Html),
            ..Default::default()
        };

        let res = configure_settings(&mut settings, &settings_args, &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let actual_settings = String::from_utf8(cursor.into_inner()).unwrap();
        let expected_settings = r#"{
    "sources": [],
    "ignored_urls": [],
    "underlying_urls": [],
    "cache_mode": "html",
    "max_open_files": 500,
    "max_concurrent_requests": 500,
    "request_timeout": 60000,
    "request_throttling": 3000,
    "max_idle_connections_per_host": 1,
    "idle_connections_timeout": 5000
}"#;
        assert_eq!(actual_settings, expected_settings);
    }

    #[test]
    fn test_configure_urls() {
        let mut cursor = Cursor::new(Vec::new());
        let mut settings = Settings::default();
        let settings_args = SettingsArgs {
            ignored_urls: vec![
                "https://url1.com".to_string(),
                "https://url2.com".to_string(),
            ],
            underlying_urls: vec!["https://news.ycombinator.com".to_string()],
            ..Default::default()
        };

        let res = configure_settings(&mut settings, &settings_args, &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let actual_settings = String::from_utf8(cursor.into_inner()).unwrap();

        let expected_settings = r#"{
    "sources": [],
    "ignored_urls": [
        "https://url1.com/",
        "https://url2.com/"
    ],
    "underlying_urls": [
        "https://news.ycombinator.com/"
    ],
    "cache_mode": "text",
    "max_open_files": 500,
    "max_concurrent_requests": 500,
    "request_timeout": 60000,
    "request_throttling": 3000,
    "max_idle_connections_per_host": 1,
    "idle_connections_timeout": 5000
}"#;
        assert_eq!(actual_settings, expected_settings);
    }

    #[test]
    fn test_configure_urls_twice() {
        let mut cursor = Cursor::new(Vec::new());
        let mut settings = Settings::default();
        let settings_args = SettingsArgs {
            ignored_urls: vec![
                "https://url1.com".to_string(),
                "https://url2.com".to_string(),
            ],
            underlying_urls: vec!["https://news.ycombinator.com".to_string()],
            ..Default::default()
        };

        let res = configure_settings(&mut settings, &settings_args, &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let res = configure_settings(&mut settings, &settings_args, &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());
    }
}
