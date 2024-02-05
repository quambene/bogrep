use crate::{
    args::UpdateArgs,
    bookmark_reader::{SourceReader, TargetReaderWriter},
    bookmarks::BookmarkProcessor,
    cache::CacheMode,
    Cache, Caching, Client, Config, Fetch, Settings, SourceBookmarks, TargetBookmarks,
};
use log::debug;

/// Import the diff of source and target bookmarks. Fetch and cache websites for
/// new bookmarks; delete cache for removed bookmarks.
pub async fn update(config: &Config, args: &UpdateArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);
    let client = Client::new(config)?;

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

    update_bookmarks(
        &client,
        &cache,
        &mut source_reader,
        &mut target_bookmarks,
        &config.settings,
    )
    .await?;

    target_reader_writer.write(&target_bookmarks)?;
    target_reader_writer.close()?;

    Ok(())
}

async fn update_bookmarks(
    client: &impl Fetch,
    cache: &impl Caching,
    source_readers: &[SourceReader],
    target_bookmarks: &mut TargetBookmarks,
    settings: &Settings,
) -> Result<(), anyhow::Error> {
    let mut source_bookmarks = SourceBookmarks::default();

    for source_reader in source_readers {
        todo!()
    }

    target_bookmarks.update(&source_bookmarks)?;

    let bookmark_processor =
        BookmarkProcessor::new(client.clone(), cache.clone(), settings.clone());
    bookmark_processor
        .process_bookmarks(target_bookmarks.values_mut().collect())
        .await?;
    bookmark_processor
        .process_underlyings(target_bookmarks)
        .await?;

    target_bookmarks.clean_up();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bookmarks::RawSource, Action, MockCache, MockClient, TargetBookmark, UnderlyingType,
    };
    use chrono::Utc;
    use std::{
        collections::{HashMap, HashSet},
        path::Path,
    };
    use url::Url;

    #[tokio::test]
    async fn test_update_bookmarks_mode_html() {
        let now = Utc::now();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Html);
        let bookmark_path = Path::new("test_data/bookmarks_chromium.json");
        let source = RawSource::new(bookmark_path, vec![]);
        let source_reader = SourceReader::init(&source).unwrap();
        let settings = Settings::default();
        let url1 = Url::parse("https://www.deepl.com/translator").unwrap();
        let url2 = Url::parse(
            "https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/",
        )
        .unwrap();
        let url3 = Url::parse("https://en.wikipedia.org/wiki/Design_Patterns").unwrap();
        let url4 = Url::parse("https://doc.rust-lang.org/book/title-page.html").unwrap();
        let expected_bookmarks: HashSet<Url> =
            HashSet::from_iter([url1.clone(), url2.clone(), url3.clone(), url4.clone()]);
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                url1.clone(),
                TargetBookmark {
                    id: "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url: url1.clone(),
                    underlying_url: None,
                    underlying_type: UnderlyingType::None,
                    last_imported: now.timestamp_millis(),
                    last_cached: Some(now.timestamp_millis()),
                    sources: HashSet::new(),
                    cache_modes: HashSet::from_iter([CacheMode::Html]),
                    action: Action::FetchAndAdd,
                },
            ),
            (
                url2.clone(),
                TargetBookmark {
                    id: "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    url: url2.clone(),
                    underlying_url: None,
                    underlying_type: UnderlyingType::None,
                    last_imported: now.timestamp_millis(),
                    last_cached: Some(now.timestamp_millis()),
                    sources: HashSet::new(),
                    cache_modes: HashSet::from_iter([CacheMode::Html]),
                    action: Action::FetchAndAdd,
                },
            ),
        ]));
        for url in &expected_bookmarks {
            client
                .add(
                    "<html><head></head><body><img></img><p>Test content (fetched)</p></body></html>"
                        .to_owned(),
                    url,
                )
                .unwrap();
        }
        cache
            .add(
                "<html><head></head><body><p>Test content (already cached)</p></body></html>"
                    .to_owned(),
                target_bookmarks.get_mut(&url1).unwrap(),
            )
            .await
            .unwrap();
        cache
            .add(
                "<html><head></head><body><p>Test content (already cached)</p></body></html>"
                    .to_owned(),
                target_bookmarks.get_mut(&url2).unwrap(),
            )
            .await
            .unwrap();

        let res = update_bookmarks(
            &client,
            &cache,
            &mut [source_reader],
            &mut target_bookmarks,
            &settings,
        )
        .await;
        assert!(res.is_ok());
        assert_eq!(
            target_bookmarks
                .values()
                .map(|bookmark| bookmark.url.clone())
                .collect::<HashSet<_>>(),
            HashSet::from_iter([url1.clone(), url2.clone(), url3.clone(), url4.clone()]),
        );
        assert_eq!(
            cache
                .cache_map()
                .get("dd30381b-8e67-4e84-9379-0852f60a7cd7")
                .unwrap(),
            "<html><head></head><body><p>Test content (already cached)</p></body></html>"
        );
        assert_eq!(
            cache
                .cache_map()
                .get("25b6357e-6eda-4367-8212-84376c6efe05")
                .unwrap(),
            "<html><head></head><body><p>Test content (already cached)</p></body></html>"
        );
        assert_eq!(
            cache
                .cache_map()
                .get(&target_bookmarks.get(&url3).unwrap().id)
                .unwrap(),
            "<html><head></head><body><p>Test content (fetched)</p></body></html>"
        );
        assert_eq!(
            cache
                .cache_map()
                .get(&target_bookmarks.get(&url4).unwrap().id)
                .unwrap(),
            "<html><head></head><body><p>Test content (fetched)</p></body></html>"
        );
    }

    #[tokio::test]
    async fn test_update_bookmarks_mode_text() {
        let now = Utc::now();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Text);
        let bookmark_path = Path::new("test_data/bookmarks_chromium.json");
        let source = RawSource::new(bookmark_path, vec![]);
        let source_reader = SourceReader::init(&source).unwrap();
        let settings = Settings::default();
        let url1 = Url::parse("https://www.deepl.com/translator").unwrap();
        let url2 = Url::parse(
            "https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/",
        )
        .unwrap();
        let url3 = Url::parse("https://en.wikipedia.org/wiki/Design_Patterns").unwrap();
        let url4 = Url::parse("https://doc.rust-lang.org/book/title-page.html").unwrap();
        let expected_bookmarks: HashSet<_> =
            HashSet::from_iter([url1.clone(), url2.clone(), url3.clone(), url4.clone()]);
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                url1.clone(),
                TargetBookmark {
                    id: "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url: url1.clone(),
                    underlying_url: None,
                    underlying_type: UnderlyingType::None,
                    last_imported: now.timestamp_millis(),
                    last_cached: Some(now.timestamp_millis()),
                    sources: HashSet::new(),
                    cache_modes: HashSet::from_iter([CacheMode::Text]),
                    action: Action::FetchAndAdd,
                },
            ),
            (
                url2.clone(),
                TargetBookmark {
                    id: "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    url: url2.clone(),
                    underlying_url: None,
                    underlying_type: UnderlyingType::None,
                    last_imported: now.timestamp_millis(),
                    last_cached: Some(now.timestamp_millis()),
                    sources: HashSet::new(),
                    cache_modes: HashSet::from_iter([CacheMode::Text]),
                    action: Action::FetchAndAdd,
                },
            ),
        ]));
        for url in &expected_bookmarks {
            client
                .add(
                    "<html><head></head><body><img></img><p>Test content (fetched)</p></body></html>"
                        .to_owned(),
                    url,
                )
                .unwrap();
        }
        cache
            .add(
                "<html><head></head><body><p>Test content (already cached)</p></body></html>"
                    .to_owned(),
                target_bookmarks.get_mut(&url1).unwrap(),
            )
            .await
            .unwrap();
        cache
            .add(
                "<html><head></head><body><p>Test content (already cached)</p></body></html>"
                    .to_owned(),
                target_bookmarks.get_mut(&url2).unwrap(),
            )
            .await
            .unwrap();

        let res = update_bookmarks(
            &client,
            &cache,
            &mut [source_reader],
            &mut target_bookmarks,
            &settings,
        )
        .await;
        assert!(res.is_ok());
        assert_eq!(
            target_bookmarks
                .values()
                .map(|bookmark| bookmark.url.clone())
                .collect::<HashSet<_>>(),
            HashSet::from_iter([url1.clone(), url2.clone(), url3.clone(), url4.clone()]),
        );
        assert_eq!(
            cache
                .cache_map()
                .get("dd30381b-8e67-4e84-9379-0852f60a7cd7")
                .unwrap(),
            "Test content (already cached)"
        );
        assert_eq!(
            cache
                .cache_map()
                .get("25b6357e-6eda-4367-8212-84376c6efe05")
                .unwrap(),
            "Test content (already cached)"
        );
        assert_eq!(
            cache
                .cache_map()
                .get(&target_bookmarks.get(&url3).unwrap().id)
                .unwrap(),
            "Test content (fetched)"
        );
        assert_eq!(
            cache
                .cache_map()
                .get(&target_bookmarks.get(&url4).unwrap().id)
                .unwrap(),
            "Test content (fetched)"
        );
    }
}
