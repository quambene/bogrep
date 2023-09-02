use super::fetch_and_add_all;
use crate::{Cache, Client, Config, InitArgs, TargetBookmarks};
use std::{rc::Rc, sync::Arc};

/// Import bookmarks, fetch bookmarks from url, and save fetched websites in cache.
pub async fn init(config: &Config, args: &InitArgs) -> Result<(), anyhow::Error> {
    let bookmarks = Rc::new(TargetBookmarks::init(config)?);
    let cache = Arc::new(Cache::init(config, &args.mode).await?);
    let client = Arc::new(Client::new(config)?);
    fetch_and_add_all(config, client, cache, bookmarks).await?;
    Ok(())
}
