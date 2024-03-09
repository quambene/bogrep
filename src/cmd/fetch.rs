use crate::{
    bookmark_reader::TargetReaderWriter,
    bookmarks::{BookmarkManager, BookmarkService, RunMode, ServiceConfig},
    cache::CacheMode,
    client::ClientConfig,
    utils, Cache, Client, Config, FetchArgs,
};
use chrono::Utc;
use log::debug;

/// Fetch and cache bookmarks.
pub async fn fetch(config: &Config, args: &FetchArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    if args.dry_run {
        println!("Running in dry mode ...")
    }

    let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);
    let client_config = ClientConfig::new(&config.settings);
    let client = Client::new(&client_config)?;
    let mut source_readers = [];
    let target_reader_writer = TargetReaderWriter::new(
        &config.target_bookmark_file,
        &config.target_bookmark_lock_file,
    )?;

    let now = Utc::now();
    let run_mode = if args.dry_run {
        RunMode::DryRun
    } else if !args.diff.is_empty() {
        let diff_urls = utils::parse_urls(&args.diff)?;
        RunMode::FetchDiff(diff_urls)
    } else if !args.urls.is_empty() {
        let fetch_urls = utils::parse_urls(&args.urls)?;
        RunMode::FetchUrls(fetch_urls)
    } else if args.all {
        RunMode::FetchAll
    } else {
        RunMode::Fetch
    };
    let service_config =
        ServiceConfig::new(run_mode, &[], config.settings.max_concurrent_requests)?;
    let mut bookmark_manager = BookmarkManager::new();
    let bookmark_service = BookmarkService::new(service_config, client, cache);

    bookmark_service
        .run(
            &mut bookmark_manager,
            &mut source_readers,
            &mut target_reader_writer.reader(),
            &mut target_reader_writer.writer(),
            now,
        )
        .await?;

    target_reader_writer.close()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bookmarks::ServiceReport, Action, BookmarkProcessor, Caching, MockCache, MockClient,
        Settings, TargetBookmark, TargetBookmarks,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use url::Url;

    #[tokio::test]
    async fn test_fetch_and_add_all_mode_html() {
        let now = Utc::now();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Html);
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                url1.clone(),
                TargetBookmark::builder_with_id(
                    "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url1,
                    now,
                )
                .with_action(Action::FetchAndReplace)
                .build(),
            ),
            (
                url2.clone(),
                TargetBookmark::builder_with_id(
                    "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    url2,
                    now,
                )
                .with_action(Action::FetchAndReplace)
                .build(),
            ),
        ]));
        for bookmark in target_bookmarks.values() {
            client
                .add(
                    "<html><head></head><body><img></img><p>Test content</p></body></html>"
                        .to_owned(),
                    &bookmark.url(),
                )
                .unwrap();
        }

        let settings = Settings::default();
        let bookmark_processor = BookmarkProcessor::new(
            client.clone(),
            cache.clone(),
            settings,
            ServiceReport::default(),
        );
        let res = bookmark_processor
            .process_bookmarks(target_bookmarks.values_mut().collect())
            .await;
        assert!(res.is_ok());
        assert_eq!(
            cache.cache_map(),
            HashMap::from_iter(vec![
                (
                    "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    "<html><head></head><body><p>Test content</p></body></html>".to_owned()
                ),
                (
                    "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    "<html><head></head><body><p>Test content</p></body></html>".to_owned()
                )
            ])
        )
    }

    #[tokio::test]
    async fn test_fetch_and_cache_mode_text() {
        let now = Utc::now();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Text);
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                url1.clone(),
                TargetBookmark::builder_with_id(
                    "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url1.clone(),
                    now,
                )
                .with_action(Action::FetchAndReplace)
                .build(),
            ),
            (
                url2.clone(),
                TargetBookmark::builder_with_id(
                    "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    url2.clone(),
                    now,
                )
                .with_action(Action::FetchAndReplace)
                .build(),
            ),
        ]));
        for bookmark in target_bookmarks.values() {
            client
                .add(
                    "<html><head></head><body><img></img><p>Test content</p></body></html>"
                        .to_owned(),
                    &bookmark.url(),
                )
                .unwrap();
        }

        let settings = Settings::default();
        let bookmark_processor = BookmarkProcessor::new(
            client.clone(),
            cache.clone(),
            settings,
            ServiceReport::default(),
        );
        let res = bookmark_processor
            .process_bookmarks(target_bookmarks.values_mut().collect())
            .await;
        assert!(res.is_ok());
        assert_eq!(
            cache.cache_map(),
            HashMap::from_iter(vec![
                (
                    "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    "Test content".to_owned()
                ),
                (
                    "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    "Test content".to_owned()
                )
            ])
        )
    }

    #[tokio::test]
    async fn test_fetch_and_cache_if_not_exists_mode_html() {
        let now = Utc::now();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Html);
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                url1.clone(),
                TargetBookmark::builder_with_id(
                    "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url1.clone(),
                    now,
                )
                .with_action(Action::FetchAndAdd)
                .build(),
            ),
            (
                url2.clone(),
                TargetBookmark::builder_with_id(
                    "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    url2.clone(),
                    now,
                )
                .with_action(Action::FetchAndAdd)
                .build(),
            ),
        ]));
        for bookmark in target_bookmarks.values() {
            client
                .add(
                    "<html><head></head><body><img></img><p>Test content (fetched)</p></body></html>"
                        .to_owned(),
                    &bookmark.url(),
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

        let settings = Settings::default();
        let bookmark_processor = BookmarkProcessor::new(
            client.clone(),
            cache.clone(),
            settings,
            ServiceReport::default(),
        );
        let res = bookmark_processor
            .process_bookmarks(target_bookmarks.values_mut().collect())
            .await;
        assert!(res.is_ok());
        assert_eq!(
            cache.cache_map(),
            HashMap::from_iter(vec![
                (
                    "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    "<html><head></head><body><p>Test content (already cached)</p></body></html>"
                        .to_owned()
                ),
                (
                    "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    "<html><head></head><body><p>Test content (fetched)</p></body></html>"
                        .to_owned()
                )
            ])
        );
    }

    #[tokio::test]
    async fn test_fetch_and_cache_if_not_exists_mode_text() {
        let now = Utc::now();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Text);
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                url1.clone(),
                TargetBookmark::builder_with_id(
                    "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url1.clone(),
                    now,
                )
                .with_action(Action::FetchAndAdd)
                .build(),
            ),
            (
                url2.clone(),
                TargetBookmark::builder_with_id(
                    "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    url2.clone(),
                    now,
                )
                .with_action(Action::FetchAndAdd)
                .build(),
            ),
        ]));
        for bookmark in target_bookmarks.values() {
            client
                .add(
                    "<html><head></head><body><img></img><p>Test content (fetched)</p></body></html>"
                        .to_owned(),
                    &bookmark.url(),
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

        let settings = Settings::default();
        let bookmark_processor = BookmarkProcessor::new(
            client.clone(),
            cache.clone(),
            settings,
            ServiceReport::default(),
        );
        let res = bookmark_processor
            .process_bookmarks(target_bookmarks.values_mut().collect())
            .await;
        assert!(res.is_ok());
        assert_eq!(
            cache.cache_map(),
            HashMap::from_iter(vec![
                (
                    "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    "Test content (already cached)".to_owned()
                ),
                (
                    "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    "Test content (fetched)".to_owned()
                )
            ])
        );
    }
}
