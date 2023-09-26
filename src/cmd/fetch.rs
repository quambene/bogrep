use std::io::{Read, Write};

use crate::{
    html, utils, Cache, Caching, Client, Config, Fetch, FetchArgs, TargetBookmark, TargetBookmarks,
};
use chrono::Utc;
use colored::Colorize;
use futures::{stream, StreamExt};
use log::{debug, error, trace, warn};
use similar::{ChangeTag, TextDiff};

/// Fetch and cache bookmarks.
pub async fn fetch(config: &Config, args: &FetchArgs) -> Result<(), anyhow::Error> {
    let mut target_bookmark_file =
        utils::open_file_in_read_write_mode(&config.target_bookmark_file)?;
    let cache = Cache::new(&config.cache_path, &args.mode);
    let client = Client::new(config)?;
    fetch_and_cache(
        &mut target_bookmark_file,
        &cache,
        &client,
        config.settings.max_concurrent_requests,
        args.all,
    )
    .await?;
    Ok(())
}

pub async fn fetch_and_cache(
    target_bookmark_file: &mut (impl Read + Write),
    cache: &impl Caching,
    client: &impl Fetch,
    max_concurrent_requests: usize,
    fetch_all: bool,
) -> Result<(), anyhow::Error> {
    let mut bookmarks = TargetBookmarks::read(target_bookmark_file)?;

    fetch_and_add_all(
        client,
        cache,
        &mut bookmarks.bookmarks,
        max_concurrent_requests,
        fetch_all,
    )
    .await?;

    trace!("Fetched bookmarks: {bookmarks:#?}");

    // Write bookmarks with updated timestamps.
    bookmarks.write(target_bookmark_file)?;
    Ok(())
}

/// Fetch all bookmarks and add them to cache.
pub async fn fetch_and_add_all(
    client: &impl Fetch,
    cache: &impl Caching,
    bookmarks: &mut [TargetBookmark],
    max_concurrent_requests: usize,
    fetch_all: bool,
) -> Result<(), anyhow::Error> {
    let mut stream = stream::iter(bookmarks)
        .map(|bookmark| fetch_and_add(client, cache, bookmark, fetch_all))
        .buffer_unordered(max_concurrent_requests);

    while let Some(item) = stream.next().await {
        if let Err(err) = item {
            error!("Can't fetch bookmark: {err}");
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
                let website = html::filter_html(&website)?;
                if let Err(err) = cache.replace(website, bookmark).await {
                    error!("Can't replace website {} in cache: {}", bookmark.url, err);
                } else {
                    bookmark.last_cached = Some(Utc::now().timestamp_millis());
                }
            }
            Err(err) => {
                error!("Can't fetch website: {}", err);
            }
        }
    } else if !cache.exists(bookmark) {
        match client.fetch(bookmark).await {
            Ok(website) => {
                let website = html::filter_html(&website)?;
                if let Err(err) = cache.add(website, bookmark).await {
                    error!("Can't add website '{}' to cache: {}", bookmark.url, err);
                }
            }
            Err(err) => {
                error!("Can't fetch website from '{}': {}", bookmark.url, err);
            }
        }
    }

    Ok(())
}

/// Fetch difference between cached and fetched website, and display changes.
pub async fn fetch_diff(config: &Config, args: FetchArgs) -> Result<(), anyhow::Error> {
    debug!("Diff content for urls: {:#?}", args.urls);
    let mut target_bookmark_file = utils::open_file(&config.target_bookmark_file)?;
    let target_bookmarks = TargetBookmarks::read(&mut target_bookmark_file)?;
    let cache = Cache::new(&config.cache_path, &args.mode);
    let client = Client::new(config)?;

    for url in args.urls {
        let bookmark = target_bookmarks.find(&url);

        if let Some(bookmark) = bookmark {
            if let Some(cached_website) = cache.get(bookmark)? {
                let fetched_website = client.fetch(bookmark).await?;
                let fetched_website = html::filter_html(&fetched_website)?;

                let diff = TextDiff::from_lines(&cached_website, &fetched_website);

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

                // Cache fetched website
                cache.replace(fetched_website, bookmark).await?;
            }
        } else {
            warn!("Bookmark missing: add bookmark first before running `bogrep fetch --diff`");
        }
    }

    Ok(())
}
