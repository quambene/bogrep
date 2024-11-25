use crate::{
    args::UpdateArgs,
    bookmark_reader::{SourceReader, TargetReaderWriter},
    bookmarks::{BookmarkManager, BookmarkService, RunMode, ServiceConfig},
    cache::CacheMode,
    client::ClientConfig,
    Cache, Client, Config,
};
use chrono::Utc;
use log::debug;

/// Import the diff of source and target bookmarks. Fetch and cache websites for
/// new bookmarks; delete cache for removed bookmarks.
pub async fn sync(config: &Config, args: &UpdateArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    if args.dry_run {
        println!("Running in dry mode ...")
    }

    let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);
    let client_config = ClientConfig::new(&config.settings);
    let client = Client::new(&client_config)?;

    let mut source_readers = config
        .settings
        .sources
        .iter()
        .map(SourceReader::init)
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    let target_reader_writer = TargetReaderWriter::new(
        &config.target_bookmark_file,
        &config.target_bookmark_lock_file,
    )?;
    let now = Utc::now();
    let run_mode = if args.dry_run {
        RunMode::DryRun
    } else {
        RunMode::Update
    };
    let service_config = ServiceConfig::new(
        run_mode,
        &config.settings.ignored_urls,
        config.settings.max_concurrent_requests,
    )?;
    let mut bookmark_manager = BookmarkManager::default();
    let bookmark_service = BookmarkService::new(service_config, client, cache);

    bookmark_service
        .run(
            &mut bookmark_manager,
            &mut source_readers,
            &mut target_reader_writer.reader(),
            &mut target_reader_writer.writer(),
            now,
        )
        .await?;

    target_reader_writer.close()?;

    Ok(())
}
