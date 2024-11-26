use crate::{
    args::CleanArgs, cache::CacheMode, client::ClientConfig, BookmarkManager, BookmarkService,
    Cache, Client, Config, RunMode, ServiceConfig, TargetReaderWriter,
};
use chrono::Utc;
use log::debug;

/// Clean up cache for removed bookmarks.
pub async fn clean(config: &Config, args: &CleanArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    let now = Utc::now();
    let run_mode = if args.all {
        RunMode::RemoveAll
    } else {
        RunMode::Remove
    };
    let service_config =
        ServiceConfig::new(run_mode, &[], config.settings.max_concurrent_requests)?;
    let client_config = ClientConfig::new(&config.settings);
    let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);
    let client = Client::new(&client_config)?;
    let target_reader_writer = TargetReaderWriter::new(
        &config.target_bookmark_file,
        &config.target_bookmark_lock_file,
    )?;
    let mut bookmark_manager = BookmarkManager::default();
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
