use crate::{
    args::CleanArgs, bookmark_reader::ReadTarget, cache::CacheMode, Cache, Caching, Config,
    TargetBookmarks, TargetReaderWriter,
};
use log::debug;

/// Clean up cache for removed bookmarks.
pub async fn clean(
    config: &Config,
    args: &CleanArgs,
    target_reader_writer: &TargetReaderWriter,
) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    let mut target_bookmarks = TargetBookmarks::default();
    let mut target_reader = target_reader_writer.reader();
    target_reader.read(&mut target_bookmarks)?;
    let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);

    if args.all {
        cache.clear(&mut target_bookmarks)?;
    } else {
        cache.remove_all(&mut target_bookmarks).await?;
    }

    Ok(())
}
