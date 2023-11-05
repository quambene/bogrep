use super::fetch_and_add_all;
use crate::{
    bookmark_reader::{BookmarkReaders, SourceReader},
    utils, Cache, Caching, Client, Config, Fetch, InitArgs, SourceBookmarks, TargetBookmarks,
};
use log::info;
use std::io::Seek;

/// Import bookmarks, fetch bookmarks from url, and save fetched websites in
/// cache if bookmarks were not imported yet.
pub async fn init(config: &Config, args: &InitArgs) -> Result<(), anyhow::Error> {
    let bookmark_readers = BookmarkReaders::new();
    let mut source_reader = config
        .settings
        .sources
        .iter()
        .map(|source| SourceReader::new(source, &bookmark_readers.0))
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
            config.settings.max_concurrent_requests,
        )
        .await?;
        // Rewind after reading the content from the file and overwrite it with the
        // updated content.
        target_bookmark_file.rewind()?;
        target_bookmarks.write(&mut target_bookmark_file)?;
    }

    Ok(())
}

async fn init_bookmarks(
    client: &impl Fetch,
    cache: &impl Caching,
    source_reader: &mut [SourceReader],
    max_concurrent_requests: usize,
) -> Result<TargetBookmarks, anyhow::Error> {
    let source_bookmarks = SourceBookmarks::read(source_reader)?;
    let mut target_bookmarks = TargetBookmarks::from(source_bookmarks);

    info!(
        "Imported {} bookmarks from {} sources: {}",
        target_bookmarks.bookmarks.len(),
        source_reader.len(),
        source_reader
            .iter()
            .map(|reader| reader.source().path.to_string_lossy())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockCache, MockClient, Source};
    use std::{
        collections::{HashMap, HashSet},
        path::Path,
    };

    #[tokio::test]
    async fn test_init_bookmarks() {
        let client = MockClient::new();
        let cache = MockCache::new();
        let bookmark_path = Path::new("test_data/source/bookmarks_google-chrome.json");
        let bookmark_readers = BookmarkReaders::new();
        let source = Source::new(bookmark_path, vec![]);
        let source_reader = SourceReader::new(&source, &bookmark_readers.0).unwrap();
        let max_concurrent_requests = 100;
        let expected_bookmarks: HashSet<String> = HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]);
        for url in &expected_bookmarks {
            client
                .add(
                    "<html><head></head><body><img></img><p>Test content</p></body></html>"
                        .to_owned(),
                    url,
                )
                .unwrap();
        }

        let res = init_bookmarks(
            &client,
            &cache,
            &mut [source_reader],
            max_concurrent_requests,
        )
        .await;
        assert!(res.is_ok());

        let target_bookmarks = res.unwrap();
        assert_eq!(
            target_bookmarks
                .bookmarks
                .iter()
                .map(|bookmark| bookmark.url.clone())
                .collect::<HashSet<_>>(),
            expected_bookmarks,
        );
        assert!(target_bookmarks
            .bookmarks
            .iter()
            .all(|bookmark| bookmark.last_cached.is_some()));
        assert_eq!(
            cache.cache_map(),
            target_bookmarks
                .bookmarks
                .iter()
                .fold(HashMap::new(), |mut acc, bookmark| {
                    acc.insert(
                        bookmark.id.clone(),
                        "<html><head></head><body><p>Test content</p></body></html>".to_owned(),
                    );
                    acc
                })
        );
    }
}
