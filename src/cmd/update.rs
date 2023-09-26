use super::fetch_and_add_all;
use crate::{
    args::UpdateArgs, bookmark_reader::SourceReader, utils, Cache, Caching, Client, Config,
    SourceBookmarks, TargetBookmarks,
};

/// Import the diff of source and target bookmarks. Fetch and cache websites for
/// new bookmarks; delete cache for removed bookmarks.
pub async fn update(config: &Config, args: &UpdateArgs) -> Result<(), anyhow::Error> {
    let mut source_reader = config
        .settings
        .sources
        .iter()
        .map(SourceReader::new)
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    let mut target_bookmark_file =
        utils::open_file_in_read_write_mode(&config.target_bookmark_file)?;

    let cache = Cache::new(&config.cache_path, &args.mode);
    let client = Client::new(config)?;

    let source_bookmarks = SourceBookmarks::read(source_reader.as_mut())?;
    let mut target_bookmarks = TargetBookmarks::read(&mut target_bookmark_file)?;

    let (mut bookmarks_to_add, bookmarks_to_remove) = target_bookmarks.update(source_bookmarks)?;

    if !bookmarks_to_add.is_empty() {
        // Fetch and cache new bookmarks.
        fetch_and_add_all(
            &client,
            &cache,
            &mut bookmarks_to_add,
            config.settings.max_concurrent_requests,
            false,
        )
        .await?;
    }

    // Clean up cache for missing bookmarks.
    for bookmark in bookmarks_to_remove {
        cache.remove(&bookmark).await?;
    }

    target_bookmarks.write(&mut target_bookmark_file)?;

    Ok(())
}
