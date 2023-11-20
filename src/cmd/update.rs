use super::fetch_and_add_all;
use crate::{
    args::UpdateArgs,
    bookmark_reader::{ReadTarget, SourceReader, WriteTarget},
    cache::CacheMode,
    utils, Cache, Caching, Client, Config, Fetch, SourceBookmarks, TargetBookmarks,
};

/// Import the diff of source and target bookmarks. Fetch and cache websites for
/// new bookmarks; delete cache for removed bookmarks.
pub async fn update(config: &Config, args: &UpdateArgs) -> Result<(), anyhow::Error> {
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
    let mut target_reader = utils::open_file_in_read_mode(&config.target_bookmark_file)?;
    let mut target_writer = utils::open_and_truncate_file(&config.target_bookmark_lock_file)?;
    target_reader.read(&mut target_bookmarks)?;

    update_bookmarks(
        &client,
        &cache,
        &mut source_reader,
        &mut target_bookmarks,
        config.settings.max_concurrent_requests,
    )
    .await?;

    target_writer.write(&target_bookmarks)?;

    utils::close_and_rename(
        (target_writer, &config.target_bookmark_lock_file),
        (target_reader, &config.target_bookmark_file),
    )?;
    Ok(())
}

async fn update_bookmarks(
    client: &impl Fetch,
    cache: &impl Caching,
    source_reader: &mut [SourceReader],
    target_bookmarks: &mut TargetBookmarks,
    max_concurrent_requests: usize,
) -> Result<(), anyhow::Error> {
    let mut source_bookmarks = SourceBookmarks::default();

    for reader in source_reader.iter_mut() {
        reader.read_and_parse(&mut source_bookmarks)?;
    }

    let (mut bookmarks_to_add, bookmarks_to_remove) = target_bookmarks.update(&source_bookmarks)?;

    if !bookmarks_to_add.is_empty() {
        // Fetch and cache new bookmarks.
        fetch_and_add_all(
            client,
            cache,
            bookmarks_to_add.iter_mut().collect(),
            max_concurrent_requests,
            false,
        )
        .await?;
    }

    // Clean up cache for missing bookmarks.
    for bookmark in bookmarks_to_remove {
        cache.remove(&bookmark).await?;
    }

    // Update the `last_cached` timestamp.
    for bookmark in bookmarks_to_add {
        target_bookmarks.insert(bookmark)
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bookmarks::RawSource, MockCache, MockClient, TargetBookmark};
    use chrono::Utc;
    use std::{
        collections::{HashMap, HashSet},
        path::Path,
    };

    #[tokio::test]
    async fn test_update_bookmarks_mode_html() {
        let now = Utc::now();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Html);
        let bookmark_path = Path::new("test_data/bookmarks_chromium.json");
        let source = RawSource::new(bookmark_path, vec![]);
        let source_reader = SourceReader::init(&source).unwrap();
        let max_concurrent_requests = 100;
        let expected_bookmarks: HashSet<String> = HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]);
        let mut target_bookmarks = TargetBookmarks::new(
            HashMap::from_iter([
                ("https://www.deepl.com/translator".to_owned(),
                TargetBookmark {
                    id: "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url: "https://www.deepl.com/translator".to_owned(),
                    last_imported: now.timestamp_millis(),
                    last_cached: Some(now.timestamp_millis()),
                    sources: HashSet::new(),
                }),("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/".to_owned(),
                TargetBookmark {
                    id: "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    url: "https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/".to_owned(),
                    last_imported: now.timestamp_millis(),
                    last_cached: Some(now.timestamp_millis()),
                    sources: HashSet::new()
                }),
            ]),
            );
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
                &target_bookmarks
                    .get("https://www.deepl.com/translator")
                    .unwrap(),
            )
            .await
            .unwrap();
        cache
            .add(
                "<html><head></head><body><p>Test content (already cached)</p></body></html>"
                    .to_owned(),
                &target_bookmarks.get("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/").unwrap(),
            )
            .await
            .unwrap();

        let res = update_bookmarks(
            &client,
            &cache,
            &mut [source_reader],
            &mut target_bookmarks,
            max_concurrent_requests,
        )
        .await;
        assert!(res.is_ok());
        assert_eq!(
            target_bookmarks
                
                .values()
                .map(|bookmark| bookmark.url.clone())
                .collect::<HashSet<_>>(),
                HashSet::from_iter([
                    String::from("https://www.deepl.com/translator"),
                    String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
                    String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
                    String::from("https://doc.rust-lang.org/book/title-page.html"),
                ]),
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
                .get(
                    &target_bookmarks
                        .get("https://en.wikipedia.org/wiki/Design_Patterns")
                        .unwrap()
                        .id
                )
                .unwrap(),
            "<html><head></head><body><p>Test content (fetched)</p></body></html>"
        );
        assert_eq!(
            cache
                .cache_map()
                .get(
                    &target_bookmarks
                        .get("https://doc.rust-lang.org/book/title-page.html")
                        .unwrap()
                        .id
                )
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
        let max_concurrent_requests = 100;
        let expected_bookmarks: HashSet<String> = HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]);
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([("https://www.deepl.com/translator".to_owned(), TargetBookmark {
            id: "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
            url: "https://www.deepl.com/translator".to_owned(),
            last_imported: now.timestamp_millis(),
            last_cached: Some(now.timestamp_millis()),
            sources: HashSet::new()
        }), ("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/".to_owned(), TargetBookmark {
            id: "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
            url: "https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/".to_owned(),
            last_imported: now.timestamp_millis(),
            last_cached: Some(now.timestamp_millis()),
            sources: HashSet::new()
        })]));
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
                &target_bookmarks.get("https://www.deepl.com/translator").unwrap(),
            )
            .await
            .unwrap();
        cache
            .add(
                "<html><head></head><body><p>Test content (already cached)</p></body></html>"
                    .to_owned(),
                &target_bookmarks.get("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/").unwrap(),
            )
            .await
            .unwrap();

        let res = update_bookmarks(
            &client,
            &cache,
            &mut [source_reader],
            &mut target_bookmarks,
            max_concurrent_requests,
        )
        .await;
        assert!(res.is_ok());
        assert_eq!(
            target_bookmarks
                
                .values()
                .map(|bookmark| bookmark.url.clone())
                .collect::<HashSet<_>>(),
                HashSet::from_iter([
                    String::from("https://www.deepl.com/translator"),
                    String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
                    String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
                    String::from("https://doc.rust-lang.org/book/title-page.html"),
                ]),
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
                .get(
                    &target_bookmarks
                        .get("https://en.wikipedia.org/wiki/Design_Patterns")
                        .unwrap()
                        .id
                )
                .unwrap(),
            "Test content (fetched)"
        );
        assert_eq!(
            cache
                .cache_map()
                .get(
                    &target_bookmarks
                        .get("https://doc.rust-lang.org/book/title-page.html")
                        .unwrap()
                        .id
                )
                .unwrap(),
            "Test content (fetched)"
        );
    }
}
