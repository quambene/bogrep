use crate::{args::CleanArgs, Cache, Config, TargetBookmarks};

/// Clean up cache for removed bookmarks.
pub async fn clean(config: &Config, args: &CleanArgs) -> Result<(), anyhow::Error> {
    let bookmarks = TargetBookmarks::read(config)?;
    let cache = Cache::new(&config.cache_path, &args.mode)?;
    cache.remove_all(&bookmarks).await?;

    Ok(())
}
