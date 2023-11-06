use crate::{
    bookmark_reader::{BookmarkReaders, SourceReader},
    cache::CacheMode,
    json, utils, Config, ConfigArgs, Settings, Source,
};
use anyhow::{anyhow, Context};
use log::info;
use std::{fs, io::Write};

/// Configure the source files to import the bookmarks.
pub fn configure(config: Config, args: ConfigArgs) -> Result<(), anyhow::Error> {
    if config.verbosity >= 1 {
        info!("{args:?}");
    }

    let cache_mode = args.set_cache_mode.cache_mode;
    let source_path = args
        .set_source
        .source
        .map(|source_path| fs::canonicalize(source_path).context("Invalid source path"))
        .transpose()?
        .ok_or(anyhow!("Invalid source path"))?;
    let source_folders = args.set_source.folders;
    let source = Source::new(source_path, source_folders);
    let settings_file = utils::open_file_in_read_write_mode(&config.settings_path)?;

    configure_settings(config.settings, source, cache_mode, settings_file)?;

    Ok(())
}

fn configure_settings(
    settings: Settings,
    source: Source,
    cache_mode: Option<CacheMode>,
    mut writer: impl Write,
) -> Result<(), anyhow::Error> {
    let mut settings = settings;
    let settings_read = settings.clone();
    let bookmark_readers = BookmarkReaders::new();
    // Validate source file
    let _ = SourceReader::select_reader(&source.path, &bookmark_readers.0)?;
    settings.configure(source, cache_mode)?;

    if settings_read != settings {
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
    fn test_configure_settings() {
        let mut cursor = Cursor::new(Vec::new());
        let settings = Settings::default();
        let source = Source {
            path: PathBuf::from("test_data/source/bookmarks_simple.txt"),
            folders: vec![],
        };
        let cache_mode = None;
        let res = configure_settings(settings, source, cache_mode, &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let actual_settings = String::from_utf8(cursor.into_inner()).unwrap();

        let expected_settings = r#"{
    "bookmark_files": [
        {
            "source": "test_data/source/bookmarks_simple.txt",
            "folders": []
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
}
