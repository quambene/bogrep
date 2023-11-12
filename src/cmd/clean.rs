use crate::{
    args::CleanArgs, bookmark_reader::TargetReaderWriter, cache::CacheMode, utils, Cache, Caching,
    Config, TargetBookmarks,
};

/// Clean up cache for removed bookmarks.
pub async fn clean(config: &Config, args: &CleanArgs) -> Result<(), anyhow::Error> {
    let target_bookmark_file = utils::open_file(&config.target_bookmark_file)?;
    let mut target_reader_writer = TargetReaderWriter::new(target_bookmark_file);
    let mut target_bookmarks = TargetBookmarks::default();
    target_reader_writer.read(&mut target_bookmarks)?;
    let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);

    if args.all {
        cache.clear(&target_bookmarks)?;
    } else {
        cache.remove_all(&target_bookmarks).await?;
    }

    Ok(())
}
