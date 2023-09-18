use super::fetch::fetch_and_add_urls;
use crate::{
    args::UpdateArgs, bookmark_reader::SourceReader, utils, Cache, Client, Config, SourceBookmarks,
    TargetBookmarks,
};
use chrono::Utc;
use log::info;

/// Determine diff of source and target bookmarks. Fetch and cache websites for
/// new bookmarks; delete cache for removed bookmarks.
pub async fn update(config: &Config, args: &UpdateArgs) -> Result<(), anyhow::Error> {
    let cache = Cache::new(&config.cache_path, &args.mode)?;
    let client = Client::new(config)?;

    let mut source_reader = config
        .settings
        .sources
        .iter()
        .map(SourceReader::new)
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    let mut target_bookmark_file =
        utils::open_file_in_read_write_mode(&config.target_bookmark_file)?;

    let source_bookmarks = SourceBookmarks::read(source_reader.as_mut())?;
    let mut target_bookmarks = TargetBookmarks::read(&mut target_bookmark_file)?;
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

        target_bookmarks.write(&mut target_bookmark_file)?;

        Ok(())
    }
}
