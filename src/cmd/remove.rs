use crate::{
    args::RemoveArgs,
    bookmarks::{BookmarkManager, RunMode, ServiceConfig},
    client::ClientConfig,
    utils, BookmarkService, Cache, CacheMode, Client, Config,
};
use anyhow::anyhow;
use chrono::Utc;
use log::debug;

/// Remove urls from bookmarks.
pub async fn remove(config: Config, args: RemoveArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    let urls = utils::parse_urls(&args.urls)?;

    if urls.is_empty() {
        return Err(anyhow!("Invalid argument: Specify the URLs to be removed"));
    }

    let now = Utc::now();
    let service_config = ServiceConfig::new(
        RunMode::RemoveUrls(urls.clone()),
        &[],
        config.settings.max_concurrent_requests,
    )?;
    let client_config = ClientConfig::new(&config.settings);
    let cache_mode = CacheMode::new(&None, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);
    let client = Client::new(&client_config)?;
    let target_reader_writer = utils::open_file_in_read_write_mode(&config.target_bookmark_file)?;
    let mut bookmark_manager = BookmarkManager::new(Box::new(target_reader_writer));
    let bookmark_service = BookmarkService::new(service_config, client, cache);

    bookmark_service.run(&mut bookmark_manager, now).await?;

    Ok(())
}
