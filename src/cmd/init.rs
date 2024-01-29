use crate::{
    bookmark_reader::{SourceReader, TargetReaderWriter},
    bookmarks::BookmarkProcessor,
    cache::CacheMode,
    Action, Cache, Caching, Client, Config, Fetch, InitArgs, Settings, SourceBookmarks,
    TargetBookmarks,
};
use log::debug;

/// Import bookmarks, fetch bookmarks from url, and save fetched websites in
/// cache if bookmarks were not imported yet.
pub async fn init(config: &Config, args: &InitArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    let mut source_reader = config
        .settings
        .sources
        .iter()
        .map(SourceReader::init)
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

    let mut target_bookmarks = TargetBookmarks::default();
    let mut target_reader_writer = TargetReaderWriter::new(
        &config.target_bookmark_file,
        &config.target_bookmark_lock_file,
    )?;
    target_reader_writer.read(&mut target_bookmarks)?;

    if !target_bookmarks.is_empty() {
        println!("Bookmarks already imported");
    } else {
        let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
        let cache = Cache::new(&config.cache_path, cache_mode);
        let client = Client::new(config)?;

        let target_bookmarks =
            init_bookmarks(&client, &cache, source_reader.as_mut(), &config.settings).await?;
        target_reader_writer.write(&target_bookmarks)?;
        target_reader_writer.close()?;
    }

    Ok(())
}

async fn init_bookmarks(
    client: &impl Fetch,
    cache: &impl Caching,
    source_reader: &mut [SourceReader],
    settings: &Settings,
) -> Result<TargetBookmarks, anyhow::Error> {
    let mut source_bookmarks = SourceBookmarks::default();

    for reader in source_reader.iter_mut() {
        reader.read_and_parse(&mut source_bookmarks)?;
    }

    let mut target_bookmarks = TargetBookmarks::try_from(source_bookmarks)?;

    target_bookmarks.set_action(&Action::FetchAndAdd);

    println!(
        "Imported {} bookmarks from {} sources: {}",
        target_bookmarks.len(),
        source_reader.len(),
        source_reader
            .iter()
            .map(|reader| reader.source().path.to_owned())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let bookmark_processor =
        BookmarkProcessor::new(client.clone(), cache.clone(), settings.clone());
    bookmark_processor
        .process_bookmarks(target_bookmarks.values_mut().collect())
        .await?;
    bookmark_processor
        .process_underlyings(&mut target_bookmarks)
        .await?;

    Ok(target_bookmarks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bookmarks::RawSource, MockCache, MockClient};
    use std::{
        collections::{HashMap, HashSet},
        path::Path,
    };
    use url::Url;

    #[tokio::test]
    async fn test_init_bookmarks_mode_html() {
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Html);
        let bookmark_path = Path::new("test_data/bookmarks_chromium.json");
        let source = RawSource::new(bookmark_path, vec![]);
        let source_reader = SourceReader::init(&source).unwrap();
        let settings = Settings::default();
        let expected_bookmarks: HashSet<Url> = HashSet::from_iter([
            Url::parse("https://www.deepl.com/translator").unwrap(),
            Url::parse("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/").unwrap(),
            Url::parse("https://en.wikipedia.org/wiki/Design_Patterns").unwrap(),
            Url::parse("https://doc.rust-lang.org/book/title-page.html").unwrap(),
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

        let res = init_bookmarks(&client, &cache, &mut [source_reader], &settings).await;
        assert!(res.is_ok());

        let target_bookmarks = res.unwrap();
        assert_eq!(
            target_bookmarks
                .values()
                .map(|bookmark| bookmark.url.clone())
                .collect::<HashSet<_>>(),
            expected_bookmarks,
        );
        assert!(target_bookmarks
            .values()
            .all(|bookmark| bookmark.last_cached.is_some()));
        assert_eq!(
            cache.cache_map(),
            target_bookmarks
                .values()
                .fold(HashMap::new(), |mut acc, bookmark| {
                    acc.insert(
                        bookmark.id.clone(),
                        "<html><head></head><body><p>Test content</p></body></html>".to_owned(),
                    );
                    acc
                })
        );
    }

    #[tokio::test]
    async fn test_init_bookmarks_mode_text() {
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Text);
        let bookmark_path = Path::new("test_data/bookmarks_chromium.json");
        let source = RawSource::new(bookmark_path, vec![]);
        let source_reader = SourceReader::init(&source).unwrap();
        let settings = Settings::default();
        let expected_bookmarks: HashSet<Url> = HashSet::from_iter([
            Url::parse("https://www.deepl.com/translator").unwrap(),
            Url::parse("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/").unwrap(),
            Url::parse("https://en.wikipedia.org/wiki/Design_Patterns").unwrap(),
            Url::parse("https://doc.rust-lang.org/book/title-page.html").unwrap(),
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

        let res = init_bookmarks(&client, &cache, &mut [source_reader], &settings).await;
        assert!(res.is_ok());

        let target_bookmarks = res.unwrap();
        assert_eq!(
            target_bookmarks
                .values()
                .map(|bookmark| bookmark.url.clone())
                .collect::<HashSet<_>>(),
            expected_bookmarks,
        );
        assert!(target_bookmarks
            .values()
            .all(|bookmark| bookmark.last_cached.is_some()));
        assert_eq!(
            cache.cache_map(),
            target_bookmarks
                .values()
                .fold(HashMap::new(), |mut acc, bookmark| {
                    acc.insert(bookmark.id.clone(), "Test content".to_owned());
                    acc
                })
        );
    }
}
