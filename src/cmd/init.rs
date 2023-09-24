use super::fetch_and_add_all;
use crate::{
    bookmark_reader::SourceReader, utils, Cache, Client, Config, InitArgs, SourceBookmarks,
    TargetBookmarks,
};
use log::info;

/// Import bookmarks, fetch bookmarks from url, and save fetched websites in
/// cache if bookmarks were not imported yet.
pub async fn init(config: &Config, args: &InitArgs) -> Result<(), anyhow::Error> {
    let mut source_reader = config
        .settings
        .sources
        .iter()
        .map(SourceReader::new)
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    let mut target_bookmark_file =
        utils::open_file_in_read_write_mode(&config.target_bookmark_file)?;
    let target_bookmarks = TargetBookmarks::read(&mut target_bookmark_file)?;

    if !target_bookmarks.bookmarks.is_empty() {
        info!("Bookmarks already imported");
    } else {
        let source_bookmarks = SourceBookmarks::read(source_reader.as_mut())?;
        let target_bookmarks = TargetBookmarks::from(source_bookmarks);
        target_bookmarks.write(&mut target_bookmark_file)?;

        info!(
            "Imported {} bookmarks from {} sources: {}",
            target_bookmarks.bookmarks.len(),
            config.settings.sources.len(),
            config
                .settings
                .sources
                .iter()
                .map(|source| source.path.to_string_lossy())
                .collect::<Vec<_>>()
                .join(", ")
        );

        let cache = Cache::new(&config.cache_path, &args.mode);
        let client = Client::new(config)?;
        fetch_and_add_all(
            config.settings.max_concurrent_requests,
            &client,
            &cache,
            &target_bookmarks.bookmarks,
        )
        .await?;
    };

    Ok(())
}
