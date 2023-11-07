use crate::{args::CleanArgs, cache::CacheMode, utils, Cache, Caching, Config, TargetBookmarks};

/// Clean up cache for removed bookmarks.
pub async fn clean(config: &Config, args: &CleanArgs) -> Result<(), anyhow::Error> {
    let mut target_bookmark_file = utils::open_file(&config.target_bookmark_file)?;
    let bookmarks = TargetBookmarks::read(&mut target_bookmark_file)?;
    let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);

    if args.all {
        cache.clear().await;
    } else {
        cache.remove_all(&bookmarks).await?;
    }

    Ok(())
}
