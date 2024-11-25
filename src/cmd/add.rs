use crate::{
    args::AddArgs,
    bookmark_reader::TargetReaderWriter,
    bookmarks::{BookmarkManager, BookmarkService, RunMode, ServiceConfig},
    client::ClientConfig,
    utils, Cache, CacheMode, Client, Config,
};
use anyhow::anyhow;
use chrono::Utc;
use log::debug;

pub async fn add(
    config: Config,
    args: AddArgs,
    target_reader_writer: &TargetReaderWriter,
) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    let urls = utils::parse_urls(&args.urls)?;

    if urls.is_empty() {
        return Err(anyhow!("Invalid argument: Specify the URLs to be added"));
    }

    let now = Utc::now();
    let service_config = ServiceConfig::new(
        RunMode::AddUrls(urls),
        &config.settings.ignored_urls,
        config.settings.max_concurrent_requests,
    )?;
    let client_config = ClientConfig::new(&config.settings);
    let cache_mode = CacheMode::new(&None, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);
    let client = Client::new(&client_config)?;
    let mut bookmark_manager = BookmarkManager::default();
    let bookmark_service = BookmarkService::new(service_config, client, cache);

    bookmark_service
        .run(
            &mut bookmark_manager,
            &mut [],
            &mut target_reader_writer.reader(),
            &mut target_reader_writer.writer(),
            now,
        )
        .await?;

    Ok(())
}
