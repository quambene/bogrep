use crate::{
    args::CleanArgs, bookmark_reader::ReadTarget, cache::CacheMode, utils, Cache, Caching, Config,
    TargetBookmarks,
};

/// Clean up cache for removed bookmarks.
pub async fn clean(config: &Config, args: &CleanArgs) -> Result<(), anyhow::Error> {
    let mut target_bookmarks = TargetBookmarks::default();
    let mut target_reader = utils::open_file_in_read_mode(&config.target_bookmark_file)?;
    target_reader.read(&mut target_bookmarks)?;
    let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);

    if args.all {
        cache.clear(&target_bookmarks)?;
    } else {
        cache.remove_all(&target_bookmarks).await?;
    }

    Ok(())
}
