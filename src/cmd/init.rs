use super::fetch_and_add_all;
use crate::{Cache, Client, Config, InitArgs, TargetBookmarks};

/// Import bookmarks, fetch bookmarks from url, and save fetched websites in cache.
pub async fn init(config: &Config, args: &InitArgs) -> Result<(), anyhow::Error> {
    let bookmarks = TargetBookmarks::init(config)?;
    let cache = Cache::init(config, &args.mode).await?;
    let client = Client::new(config)?;
    fetch_and_add_all(config, &client, &cache, &bookmarks).await?;
    Ok(())
}
