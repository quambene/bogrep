use crate::{
    bookmark_reader::{ReadTarget, WriteTarget},
    cache::CacheMode,
    html, utils, Cache, Caching, Client, Config, Fetch, FetchArgs, TargetBookmark, TargetBookmarks,
};
use chrono::Utc;
use colored::Colorize;
use futures::{stream, StreamExt};
use log::{debug, info, trace, warn};
use similar::{ChangeTag, TextDiff};
use std::collections::HashSet;

/// Fetch and cache bookmarks.
pub async fn fetch(config: &Config, args: &FetchArgs) -> Result<(), anyhow::Error> {
    let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);
    let client = Client::new(config)?;
    let mut target_reader = utils::open_file_in_read_mode(&config.target_bookmark_file)?;
    let mut target_writer = utils::open_and_truncate_file(&config.target_bookmark_lock_file)?;

    if args.urls.is_empty() {
        fetch_and_cache(
            &client,
            &cache,
            &mut target_reader,
            &mut target_writer,
            config.settings.max_concurrent_requests,
            args.all,
        )
        .await?;
    } else {
        fetch_urls(
            &args.urls,
            &client,
            &cache,
            &mut target_reader,
            &mut target_writer,
        )
        .await?;
    }

    utils::close_and_rename(
        (target_writer, &config.target_bookmark_lock_file),
        (target_reader, &config.target_bookmark_file),
    )?;

    Ok(())
}

pub async fn fetch_urls(
    urls: &[String],
    client: &impl Fetch,
    cache: &impl Caching,
    target_reader: &mut impl ReadTarget,
    target_writer: &mut impl WriteTarget,
) -> Result<(), anyhow::Error> {
    let now = Utc::now();
    let mut target_bookmarks = TargetBookmarks::default();
    target_reader.read(&mut target_bookmarks)?;

    for url in urls {
        let mut bookmark = TargetBookmark::new(url, now, None, HashSet::new());
        fetch_and_add(client, cache, &mut bookmark, true).await?;
        info!("Fetched website for {url}");
        target_bookmarks.insert(bookmark);
    }

    target_writer.write(&target_bookmarks)?;

    Ok(())
}

pub async fn fetch_and_cache(
    client: &impl Fetch,
    cache: &impl Caching,
    target_reader: &mut impl ReadTarget,
    target_writer: &mut impl WriteTarget,
    max_concurrent_requests: usize,
    fetch_all: bool,
) -> Result<(), anyhow::Error> {
    let mut target_bookmarks = TargetBookmarks::default();
    target_reader.read(&mut target_bookmarks)?;

    fetch_and_add_all(
        client,
        cache,
        target_bookmarks.values_mut().collect(),
        max_concurrent_requests,
        fetch_all,
    )
    .await?;

    trace!("Fetched bookmarks: {target_bookmarks:#?}");

    // Write bookmarks with updated timestamps.
    target_writer.write(&target_bookmarks)?;

    Ok(())
}

/// Fetch all bookmarks and add them to cache.
pub async fn fetch_and_add_all(
    client: &impl Fetch,
    cache: &impl Caching,
    bookmarks: Vec<&mut TargetBookmark>,
    max_concurrent_requests: usize,
    fetch_all: bool,
) -> Result<(), anyhow::Error> {
    let mut stream = stream::iter(bookmarks)
        .map(|bookmark| fetch_and_add(client, cache, bookmark, fetch_all))
        .buffer_unordered(max_concurrent_requests);

    while let Some(item) = stream.next().await {
        if let Err(err) = item {
            warn!("Can't fetch bookmark: {err}");
        }
    }

    Ok(())
}

/// Fetch and add bookmark to cache.
async fn fetch_and_add(
    client: &impl Fetch,
    cache: &impl Caching,
    bookmark: &mut TargetBookmark,
    fetch_all: bool,
) -> Result<(), anyhow::Error> {
    if fetch_all {
        match client.fetch(bookmark).await {
            Ok(website) => {
                trace!("Fetched website: {website}");
                let html = html::filter_html(&website)?;

                if let Err(err) = cache.replace(html, bookmark).await {
                    warn!("Can't replace website {} in cache: {}", bookmark.url, err);
                } else {
                    bookmark.last_cached = Some(Utc::now().timestamp_millis());
                }
            }
            Err(err) => {
                warn!("Can't fetch website: {}", err);
            }
        }
    } else if !cache.exists(bookmark) {
        match client.fetch(bookmark).await {
            Ok(website) => {
                trace!("Fetched website: {website}");
                let html = html::filter_html(&website)?;

                if let Err(err) = cache.add(html, bookmark).await {
                    warn!("Can't add website '{}' to cache: {}", bookmark.url, err);
                } else {
                    bookmark.last_cached = Some(Utc::now().timestamp_millis());
                }
            }
            Err(err) => {
                warn!("Can't fetch website from '{}': {}", bookmark.url, err);
            }
        }
    }

    Ok(())
}

/// Fetch difference between cached and fetched website, and display changes.
pub async fn fetch_diff(config: &Config, args: FetchArgs) -> Result<(), anyhow::Error> {
    debug!("Diff content for urls: {:#?}", args.diff);
    let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);
    let client = Client::new(config)?;

    let mut target_bookmarks = TargetBookmarks::default();
    let mut target_reader = utils::open_file_in_read_mode(&config.target_bookmark_file)?;
    target_reader.read(&mut target_bookmarks)?;

    for url in args.diff {
        let bookmark = target_bookmarks.get(&url);

        if let Some(bookmark) = bookmark {
            if let Some(cached_website_before) = cache.get(bookmark)? {
                let fetched_website = client.fetch(bookmark).await?;
                trace!("Fetched website: {fetched_website}");
                let html = html::filter_html(&fetched_website)?;

                // Cache fetched website
                let cached_website_after = cache.replace(html, bookmark).await?;

                let diff = TextDiff::from_lines(&cached_website_before, &cached_website_after);

                for change in diff.iter_all_changes() {
                    match change.tag() {
                        ChangeTag::Delete => {
                            if let Some(change) = change.as_str() {
                                print!("{}{}", "-".red(), change.red());
                            }
                        }
                        ChangeTag::Insert => {
                            if let Some(change) = change.as_str() {
                                print!("{}{}", "+".green(), change.green());
                            }
                        }
                        ChangeTag::Equal => continue,
                    }
                }
            }
        } else {
            warn!("Bookmark missing: add bookmark first before running `bogrep fetch --diff`");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockCache, MockClient};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_fetch_and_add_all_mode_html() {
        let now = Utc::now();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Html);
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                "https://test_url1.com".to_owned(),
                TargetBookmark {
                    id: "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url: "https://test_url1.com".to_owned(),
                    last_imported: now.timestamp_millis(),
                    last_cached: None,
                    sources: HashSet::new(),
                },
            ),
            (
                "https://test_url2.com".to_owned(),
                TargetBookmark {
                    id: "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    url: "https://test_url2.com".to_owned(),
                    last_imported: now.timestamp_millis(),
                    last_cached: None,
                    sources: HashSet::new(),
                },
            ),
        ]));
        for bookmark in target_bookmarks.values() {
            client
                .add(
                    "<html><head></head><body><img></img><p>Test content</p></body></html>"
                        .to_owned(),
                    &bookmark.url,
                )
                .unwrap();
        }

        let res = fetch_and_add_all(
            &client,
            &cache,
            target_bookmarks.values_mut().collect(),
            100,
            true,
        )
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
    async fn test_fetch_and_add_all_mode_text() {
        let now = Utc::now();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Text);
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                "https://test_url1.com".to_owned(),
                TargetBookmark {
                    id: "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url: "https://test_url1.com".to_owned(),
                    last_imported: now.timestamp_millis(),
                    last_cached: None,
                    sources: HashSet::new(),
                },
            ),
            (
                "https://test_url2.com".to_owned(),
                TargetBookmark {
                    id: "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    url: "https://test_url2.com".to_owned(),
                    last_imported: now.timestamp_millis(),
                    last_cached: None,
                    sources: HashSet::new(),
                },
            ),
        ]));
        for bookmark in target_bookmarks.values() {
            client
                .add(
                    "<html><head></head><body><img></img><p>Test content</p></body></html>"
                        .to_owned(),
                    &bookmark.url,
                )
                .unwrap();
        }

        let res = fetch_and_add_all(
            &client,
            &cache,
            target_bookmarks.values_mut().collect(),
            100,
            true,
        )
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
    async fn test_fetch_and_add_all_if_not_exists_mode_html() {
        let now = Utc::now().timestamp_millis();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Html);
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                "https://test_url1.com".to_owned(),
                TargetBookmark {
                    id: "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url: "https://test_url1.com".to_owned(),
                    last_imported: now,
                    last_cached: Some(now),
                    sources: HashSet::new(),
                },
            ),
            (
                "https://test_url2.com".to_owned(),
                TargetBookmark {
                    id: "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    url: "https://test_url2.com".to_owned(),
                    last_imported: now,
                    last_cached: None,
                    sources: HashSet::new(),
                },
            ),
        ]));
        for bookmark in target_bookmarks.values() {
            client
                .add(
                    "<html><head></head><body><img></img><p>Test content (fetched)</p></body></html>"
                        .to_owned(),
                    &bookmark.url,
                )
                .unwrap();
        }
        cache
            .add(
                "<html><head></head><body><p>Test content (already cached)</p></body></html>"
                    .to_owned(),
                &target_bookmarks.get("https://test_url1.com").unwrap(),
            )
            .await
            .unwrap();

        let res = fetch_and_add_all(
            &client,
            &cache,
            target_bookmarks.values_mut().collect(),
            100,
            false,
        )
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
    async fn test_fetch_and_add_all_if_not_exists_mode_text() {
        let now = Utc::now().timestamp_millis();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Text);
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                "https://test_url1.com".to_owned(),
                TargetBookmark {
                    id: "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url: "https://test_url1.com".to_owned(),
                    last_imported: now,
                    last_cached: Some(now),
                    sources: HashSet::new(),
                },
            ),
            (
                "https://test_url2.com".to_owned(),
                TargetBookmark {
                    id: "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                    url: "https://test_url2.com".to_owned(),
                    last_imported: now,
                    last_cached: None,
                    sources: HashSet::new(),
                },
            ),
        ]));
        for bookmark in target_bookmarks.values() {
            client
                .add(
                    "<html><head></head><body><img></img><p>Test content (fetched)</p></body></html>"
                        .to_owned(),
                    &bookmark.url,
                )
                .unwrap();
        }
        cache
            .add(
                "<html><head></head><body><p>Test content (already cached)</p></body></html>"
                    .to_owned(),
                &target_bookmarks.get("https://test_url1.com").unwrap(),
            )
            .await
            .unwrap();

        let res = fetch_and_add_all(
            &client,
            &cache,
            target_bookmarks.values_mut().collect(),
            100,
            false,
        )
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
