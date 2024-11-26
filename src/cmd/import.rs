use crate::{
    args::ImportArgs,
    bookmark_reader::TargetReaderWriter,
    bookmarks::{BookmarkManager, BookmarkService, RunMode, ServiceConfig},
    client::ClientConfig,
    cmd, json, utils, Cache, CacheMode, Client, Config,
};
use anyhow::anyhow;
use chrono::Utc;
use log::debug;
use std::io::Write;

/// Import bookmarks from the configured source files and store unique bookmarks
/// in cache.
pub async fn import(config: Config, args: ImportArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    if args.dry_run {
        println!("Running in dry mode ...")
    }

    let mut config = config;
    let home_dir = dirs::home_dir().ok_or(anyhow!("Missing home dir"))?;

    if config.settings.sources.is_empty() {
        if let Some(source_os) = utils::get_supported_os() {
            cmd::configure_sources(&mut config, &home_dir, &source_os)?;

            if !args.dry_run {
                let mut settings_file = utils::open_and_truncate_file(&config.settings_path)?;
                let settings_json = json::serialize(config.settings.clone())?;
                settings_file.write_all(&settings_json)?;
            }
        }
    }

    let cache_mode = CacheMode::new(&None, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);
    let client_config = ClientConfig::new(&config.settings);
    let client = Client::new(&client_config)?;
    let target_reader_writer = TargetReaderWriter::new(
        &config.target_bookmark_file,
        &config.target_bookmark_lock_file,
    )?;
    let now = Utc::now();
    let run_mode = if args.dry_run {
        RunMode::DryRun
    } else {
        RunMode::Import
    };
    let service_config = ServiceConfig::new(
        run_mode,
        &config.settings.ignored_urls,
        config.settings.max_concurrent_requests,
    )?;
    let mut bookmark_manager = BookmarkManager::from_sources(&config.settings.sources)?;
    let bookmark_service = BookmarkService::new(service_config, client, cache);

    bookmark_service
        .run(
            &mut bookmark_manager,
            &mut target_reader_writer.reader(),
            &mut target_reader_writer.writer(),
            now,
        )
        .await?;

    target_reader_writer.close()?;

    Ok(())
}
