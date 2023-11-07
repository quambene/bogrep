use crate::{args::CleanArgs, utils, Cache, Caching, Config, TargetBookmarks};

/// Clean up cache for removed bookmarks.
pub async fn clean(config: &Config, _args: &CleanArgs) -> Result<(), anyhow::Error> {
    let mut target_bookmark_file = utils::open_file(&config.target_bookmark_file)?;
    let bookmarks = TargetBookmarks::read(&mut target_bookmark_file)?;
    let cache = Cache::new(&config.cache_path, &config.settings.cache_mode);
    cache.remove_all(&bookmarks).await?;

    Ok(())
}
