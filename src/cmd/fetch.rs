use crate::{
    bookmarks::{BookmarkManager, BookmarkService, RunMode, ServiceConfig},
    cache::CacheMode,
    client::ClientConfig,
    utils, Cache, Client, Config, FetchArgs,
};
use chrono::Utc;
use log::debug;

/// Fetch and cache bookmarks.
pub async fn fetch(config: &Config, args: &FetchArgs) -> Result<(), anyhow::Error> {
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
    } else if !args.diff.is_empty() {
        let diff_urls = utils::parse_urls(&args.diff)?;
        RunMode::FetchDiff(diff_urls)
    } else if !args.urls.is_empty() {
        let fetch_urls = utils::parse_urls(&args.urls)?;

        if args.replace {
            RunMode::FetchAllUrls(fetch_urls)
        } else {
            RunMode::FetchUrls(fetch_urls)
        }
    } else if args.replace {
        RunMode::FetchAll
    } else {
        RunMode::Fetch
    };
    let service_config =
        ServiceConfig::new(run_mode, &[], config.settings.max_concurrent_requests)?;
    let mut bookmark_manager = BookmarkManager::new(Box::new(target_reader_writer));
    let bookmark_service = BookmarkService::new(service_config, client, cache);

    bookmark_service.run(&mut bookmark_manager, now).await?;

    Ok(())
}
