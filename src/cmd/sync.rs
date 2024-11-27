use crate::{
    args::SyncArgs,
    bookmarks::{BookmarkManager, BookmarkService, RunMode, ServiceConfig},
    cache::CacheMode,
    client::ClientConfig,
    utils, Cache, Client, Config,
};
use chrono::Utc;
use log::debug;

/// Import the diff of source and target bookmarks. Fetch and cache websites for
/// new bookmarks; delete cache for removed bookmarks.
pub async fn sync(config: &Config, args: &SyncArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    if args.dry_run {
        println!("Running in dry mode ...")
    }

    let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);
    let client_config = ClientConfig::new(&config.settings);
    let client = Client::new(&client_config)?;
    let target_reader_writer = utils::open_file_in_read_write_mode(&config.target_bookmark_file)?;
    let now = Utc::now();
    let run_mode = if args.dry_run {
        RunMode::DryRun
    } else {
        RunMode::Sync
    };
    let service_config = ServiceConfig::new(
        run_mode,
        &config.settings.ignored_urls,
        config.settings.max_concurrent_requests,
    )?;
    let mut bookmark_manager = BookmarkManager::new(Box::new(target_reader_writer));
    bookmark_manager.add_sources(&config.settings.sources)?;
    let bookmark_service = BookmarkService::new(service_config, client, cache);

    bookmark_service.run(&mut bookmark_manager, now).await?;

    Ok(())
}
