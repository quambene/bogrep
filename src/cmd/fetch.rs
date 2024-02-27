use crate::{
    bookmark_reader::{ReadTarget, TargetReaderWriter, WriteTarget},
    bookmarks::{Action, BookmarkProcessor, ProcessReport},
    cache::CacheMode,
    errors::BogrepError,
    html, utils, Cache, Caching, Client, Config, Fetch, FetchArgs, Settings, SourceType,
    TargetBookmark, TargetBookmarks,
};
use chrono::{DateTime, Utc};
use colored::Colorize;
use log::{debug, trace, warn};
use similar::{ChangeTag, TextDiff};
use std::collections::HashSet;
use url::Url;

/// Fetch and cache bookmarks.
pub async fn fetch(config: &Config, args: &FetchArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    if args.dry_run {
        println!("Running in dry mode ...")
    }

    let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);
    let client = Client::new(config)?;
    let target_reader_writer = TargetReaderWriter::new(
        &config.target_bookmark_file,
        &config.target_bookmark_lock_file,
    )?;

    fetch_bookmarks(
        &config.settings,
        args,
        client,
        cache,
        &mut target_reader_writer.reader(),
        &mut target_reader_writer.writer(),
    )
    .await?;

    target_reader_writer.close()?;

    Ok(())
}

pub async fn fetch_bookmarks(
    settings: &Settings,
    args: &FetchArgs,
    client: impl Fetch,
    cache: impl Caching,
    target_reader: &mut impl ReadTarget,
    target_writer: &mut impl WriteTarget,
) -> Result<(), anyhow::Error> {
    let now = Utc::now();
    let mut target_bookmarks = TargetBookmarks::default();
    target_reader.read(&mut target_bookmarks)?;

    if cache.is_empty() {
        debug!("Cache is empty");
        target_bookmarks.reset_cache_status();
    }

    set_actions(&mut target_bookmarks, now, args)?;

    let report = ProcessReport::init(args.dry_run);
    let bookmark_processor = BookmarkProcessor::new(client, cache, settings.to_owned(), report);

    bookmark_processor
        .process_bookmarks(target_bookmarks.values_mut().collect())
        .await?;
    bookmark_processor
        .process_underlyings(&mut target_bookmarks)
        .await?;

    trace!("Fetched bookmarks: {target_bookmarks:#?}");

    // Write bookmarks with updated timestamps.
    target_writer.write(&target_bookmarks)?;

    Ok(())
}

/// Set actions for bookmarks.
pub fn set_actions(
    target_bookmarks: &mut TargetBookmarks,
    now: DateTime<Utc>,
    args: &FetchArgs,
) -> Result<(), anyhow::Error> {
    if args.all {
        target_bookmarks.set_action(&Action::FetchAndReplace);
    } else if args.dry_run {
        target_bookmarks.set_action(&Action::DryRun);
    } else if !args.urls.is_empty() {
        let urls = args
            .urls
            .iter()
            .map(|url| Url::parse(url))
            .collect::<Result<Vec<_>, _>>()?;

        for url in &urls {
            if let Some(target_bookmark) = target_bookmarks.get_mut(url) {
                target_bookmark.set_action(Action::FetchAndReplace);
                target_bookmark.set_source(SourceType::Internal);
            } else {
                let mut sources = HashSet::new();
                sources.insert(SourceType::Internal);
                let target_bookmark = TargetBookmark::new(
                    url.to_owned(),
                    None,
                    now,
                    None,
                    sources,
                    HashSet::new(),
                    Action::FetchAndReplace,
                );
                target_bookmarks.insert(target_bookmark);
            }
        }
    } else {
        target_bookmarks.set_action(&Action::FetchAndAdd);
    }

    Ok(())
}

/// Fetch difference between cached and fetched website, and display changes.
pub async fn fetch_diff(config: &Config, args: FetchArgs) -> Result<(), BogrepError> {
    debug!("Diff content for urls: {:#?}", args.diff);
    let cache_mode = CacheMode::new(&args.mode, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);
    let client = Client::new(config)?;
    let urls = args
        .diff
        .iter()
        .map(|url| Url::parse(url))
        .collect::<Result<Vec<_>, _>>()?;

    let mut target_bookmarks = TargetBookmarks::default();
    let mut target_reader = utils::open_file_in_read_mode(&config.target_bookmark_file)?;
    target_reader.read(&mut target_bookmarks)?;

    for url in urls {
        let bookmark = target_bookmarks.get_mut(&url);

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
    use crate::{bookmarks::ProcessReport, MockCache, MockClient, UnderlyingType};
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
                TargetBookmark {
                    id: "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url: url1.clone(),
                    underlying_url: None,
                    underlying_type: UnderlyingType::None,
                    last_imported: now.timestamp_millis(),
                    last_cached: None,
                    sources: HashSet::new(),
                    cache_modes: HashSet::new(),
                    action: Action::FetchAndReplace,
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
                    last_cached: None,
                    sources: HashSet::new(),
                    cache_modes: HashSet::new(),
                    action: Action::FetchAndReplace,
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

        let settings = Settings::default();
        let bookmark_processor = BookmarkProcessor::new(
            client.clone(),
            cache.clone(),
            settings,
            ProcessReport::default(),
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
                TargetBookmark {
                    id: "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url: url1.clone(),
                    underlying_url: None,
                    underlying_type: UnderlyingType::None,
                    last_imported: now.timestamp_millis(),
                    last_cached: None,
                    sources: HashSet::new(),
                    cache_modes: HashSet::new(),
                    action: Action::FetchAndReplace,
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
                    last_cached: None,
                    sources: HashSet::new(),
                    cache_modes: HashSet::new(),
                    action: Action::FetchAndReplace,
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

        let settings = Settings::default();
        let bookmark_processor = BookmarkProcessor::new(
            client.clone(),
            cache.clone(),
            settings,
            ProcessReport::default(),
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
        let now = Utc::now().timestamp_millis();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Html);
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                url1.clone(),
                TargetBookmark {
                    id: "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url: url1.clone(),
                    underlying_url: None,
                    underlying_type: UnderlyingType::None,
                    last_imported: now,
                    last_cached: Some(now),
                    sources: HashSet::new(),
                    cache_modes: HashSet::new(),
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
                    last_imported: now,
                    last_cached: None,
                    sources: HashSet::new(),
                    cache_modes: HashSet::new(),
                    action: Action::FetchAndAdd,
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
                target_bookmarks.get_mut(&url1).unwrap(),
            )
            .await
            .unwrap();

        let settings = Settings::default();
        let bookmark_processor = BookmarkProcessor::new(
            client.clone(),
            cache.clone(),
            settings,
            ProcessReport::default(),
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
        let now = Utc::now().timestamp_millis();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Text);
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let mut target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                url1.clone(),
                TargetBookmark {
                    id: "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url: url1.clone(),
                    underlying_url: None,
                    underlying_type: UnderlyingType::None,
                    last_imported: now,
                    last_cached: Some(now),
                    sources: HashSet::new(),
                    cache_modes: HashSet::new(),
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
                    last_imported: now,
                    last_cached: None,
                    sources: HashSet::new(),
                    cache_modes: HashSet::new(),
                    action: Action::FetchAndAdd,
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
                target_bookmarks.get_mut(&url1).unwrap(),
            )
            .await
            .unwrap();

        let settings = Settings::default();
        let bookmark_processor = BookmarkProcessor::new(
            client.clone(),
            cache.clone(),
            settings,
            ProcessReport::default(),
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
