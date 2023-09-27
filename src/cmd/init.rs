use super::fetch_and_add_all;
use crate::{
    bookmark_reader::SourceReader, utils, Cache, Caching, Client, Config, Fetch, InitArgs, Source,
    SourceBookmarks, TargetBookmarks,
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
        let cache = Cache::new(&config.cache_path, &args.mode);
        let client = Client::new(config)?;
        let target_bookmarks = init_bookmarks(
            &client,
            &cache,
            source_reader.as_mut(),
            &config.settings.sources,
            config.settings.max_concurrent_requests,
        )
        .await?;
        target_bookmarks.write(&mut target_bookmark_file)?;
    }

    Ok(())
}

async fn init_bookmarks(
    client: &impl Fetch,
    cache: &impl Caching,
    source_reader: &mut [SourceReader],
    sources: &[Source],
    max_concurrent_requests: usize,
) -> Result<TargetBookmarks, anyhow::Error> {
    let source_bookmarks = SourceBookmarks::read(source_reader)?;
    let mut target_bookmarks = TargetBookmarks::from(source_bookmarks);

    info!(
        "Imported {} bookmarks from {} sources: {}",
        target_bookmarks.bookmarks.len(),
        sources.len(),
        sources
            .iter()
            .map(|source| source.path.to_string_lossy())
            .collect::<Vec<_>>()
            .join(", ")
    );

    fetch_and_add_all(
        client,
        cache,
        &mut target_bookmarks.bookmarks,
        max_concurrent_requests,
        false,
    )
    .await?;

    Ok(target_bookmarks)
}
