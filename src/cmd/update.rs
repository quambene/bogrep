use crate::{
    args::UpdateArgs,
    bookmark_reader::{ReadTarget, SourceReader, TargetReaderWriter, WriteTarget},
    bookmarks::{BookmarkManager, BookmarkProcessor, ProcessReport},
    cache::CacheMode,
    Cache, Caching, Client, Config, Fetch, Settings, TargetBookmarks,
};
use chrono::Utc;
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

    let mut target_bookmarks = TargetBookmarks::default();
    let mut target_reader_writer = TargetReaderWriter::new(
        &config.target_bookmark_file,
        &config.target_bookmark_lock_file,
    )?;
    target_reader_writer.read(&mut target_bookmarks)?;

    update_bookmarks(
        &client,
        &cache,
        &mut source_readers,
        &mut target_reader_writer.reader(),
        &mut target_reader_writer.writer(),
        &config.settings,
        args.dry_run,
    )
    .await?;

    target_reader_writer.close()?;

    Ok(())
}

async fn update_bookmarks(
    client: &impl Fetch,
    cache: &impl Caching,
    source_readers: &mut [SourceReader],
    target_reader: &mut impl ReadTarget,
    target_writer: &mut impl WriteTarget,
    settings: &Settings,
    dry_run: bool,
) -> Result<(), anyhow::Error> {
    let now = Utc::now();
    let mut bookmark_manager = BookmarkManager::new(dry_run);

    target_reader.read(&mut bookmark_manager.target_bookmarks_mut())?;

    bookmark_manager.import(source_readers)?;
    bookmark_manager.add_bookmarks(now)?;
    bookmark_manager.remove_bookmarks();
    bookmark_manager.set_actions();

    let bookmark_processor = BookmarkProcessor::new(
        client.clone(),
        cache.clone(),
        settings.clone(),
        ProcessReport::init(dry_run),
    );
    bookmark_processor
        .process_bookmarks(
            bookmark_manager
                .target_bookmarks_mut()
                .values_mut()
                .collect(),
        )
        .await?;
    bookmark_processor
        .process_underlyings(bookmark_manager.target_bookmarks_mut())
        .await?;

    bookmark_manager.print_report(&source_readers);
    bookmark_manager.finish();

    target_writer.write(&bookmark_manager.target_bookmarks())?;

    Ok(())
}
