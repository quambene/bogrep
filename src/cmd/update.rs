use super::fetch::fetch_and_add_urls;
use crate::{args::UpdateArgs, Cache, Client, Config, SourceBookmarks, TargetBookmarks};
use chrono::Utc;
use log::info;

/// Determine diff of source and target bookmarks. Fetch and cache websites for
/// new bookmarks; delete cache for removed bookmarks.
pub async fn update(config: &Config, args: &UpdateArgs) -> Result<(), anyhow::Error> {
    let cache = Cache::new(&config.cache_path, &args.mode)?;
    let client = Client::new()?;

    let mut source_bookmarks = SourceBookmarks::new();
    source_bookmarks.read(config)?;
    let mut target_bookmarks = TargetBookmarks::read(config)?;
    let now = Utc::now();
    let bookmarks_to_add = target_bookmarks.filter_to_add(&source_bookmarks);
    let bookmarks_to_remove = target_bookmarks.filter_to_remove(&source_bookmarks);

    if bookmarks_to_add.is_empty() && bookmarks_to_remove.is_empty() {
        info!("Bookmarks already up to date");
        Ok(())
    } else {
        // Clean up cache for missing bookmarks.
        for bookmark in bookmarks_to_remove {
            target_bookmarks.remove(&bookmark);
            cache.remove(&bookmark).await?;
        }

        // Fetch and cache new bookmarks.
        fetch_and_add_urls(
            config,
            &client,
            &cache,
            &bookmarks_to_add,
            &mut target_bookmarks,
            now,
        )
        .await?;

        target_bookmarks.write(config)?;

        Ok(())
    }
}
