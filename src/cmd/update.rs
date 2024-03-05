use crate::{
    args::UpdateArgs,
    bookmark_reader::{SourceReader, TargetReaderWriter},
    bookmarks::{RunMode, ServiceConfig},
    cache::CacheMode,
    cmd::import_and_process_bookmarks,
    Cache, Client, Config,
};
use log::debug;

/// Import the diff of source and target bookmarks. Fetch and cache websites for
/// new bookmarks; delete cache for removed bookmarks.
pub async fn update(config: &Config, args: &UpdateArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    if args.dry_run {
        println!("Running in dry mode ...")
    }

    let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);
    let client = Client::new(config)?;

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
    let run_mode = if args.dry_run {
        RunMode::DryRun
    } else {
        RunMode::FetchAll
    };
    let service_config = ServiceConfig::new(run_mode, vec![]);

    import_and_process_bookmarks(
        &config.settings,
        service_config,
        client,
        cache,
        &mut source_readers,
        &mut target_reader_writer.reader(),
        &mut target_reader_writer.writer(),
    )
    .await?;

    target_reader_writer.close()?;

    Ok(())
}
