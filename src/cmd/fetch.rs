use crate::{html, Cache, Client, Config, FetchArgs, TargetBookmark, TargetBookmarks};
use chrono::Utc;
use colored::Colorize;
use futures::{stream, StreamExt};
use log::{debug, error, trace, warn};
use similar::{ChangeTag, TextDiff};
use std::{path::Path, rc::Rc, sync::Arc};
use tokio::{sync::Mutex, task};

/// Fetch existing bookmarks and replace cached websites.
pub async fn fetch(config: &Config, args: &FetchArgs) -> Result<(), anyhow::Error> {
    let bookmarks = Rc::new(TargetBookmarks::read(config)?);
    let cache = Arc::new(Cache::init(config, &args.mode).await?);
    let client = Arc::new(Client::new(config)?);

    if args.all {
        fetch_and_replace_all(config, client, cache, bookmarks.clone()).await?;
    } else {
        fetch_and_add_all(config, client, cache, bookmarks.clone()).await?;
    }

    trace!("Fetched bookmarks: {bookmarks:#?}");

    // Write bookmarks with updated timestamps.
    bookmarks.write(config)?;

    Ok(())
}

async fn fetch_and_replace_all(
    config: &Config,
    client: Arc<Client>,
    cache: Arc<Cache>,
    bookmarks: Rc<TargetBookmarks>,
) -> Result<(), anyhow::Error> {
    let bookmarks = bookmarks
        .bookmarks
        .iter()
        .map(|bookmark| Arc::new(Mutex::new(bookmark.clone())))
        .collect::<Vec<_>>();
    let mut stream = stream::iter(bookmarks)
        .map(|bookmark| task::spawn(fetch_and_replace(client.clone(), cache.clone(), bookmark)))
        .buffer_unordered(config.settings.max_parallel_requests);

    while let Some(item) = stream.next().await {
        if let Err(err) = item? {
            error!("Can't fetch bookmark: {err}");
        }
    }

    Ok(())
}

/// Fetch bookmark and replace cached bookmark.
async fn fetch_and_replace(
    client: Arc<Client>,
    cache: Arc<Cache>,
    bookmark: Arc<Mutex<TargetBookmark>>,
) -> Result<(), anyhow::Error> {
    let mut bookmark = bookmark.lock().await;

    match client.fetch(&bookmark).await {
        Ok(website) => {
            let website = html::filter_html(&website)?;
            if let Err(err) = cache.replace(website, &bookmark).await {
                error!("Can't replace website {} in cache: {}", bookmark.url, err);
            } else {
                bookmark.last_cached = Some(Utc::now().timestamp_millis());
            }
        }
        Err(err) => {
            error!("Can't fetch website: {}", err);
        }
    }

    Ok(())
}

pub async fn fetch_and_add_all(
    config: &Config,
    client: Arc<Client>,
    cache: Arc<Cache>,
    bookmarks: Rc<TargetBookmarks>,
) -> Result<(), anyhow::Error> {
    let bookmarks = bookmarks
        .bookmarks
        .iter()
        .map(|bookmark| Arc::new(bookmark.clone()))
        .collect::<Vec<_>>();
    let mut stream = stream::iter(bookmarks)
        .map(|bookmark| task::spawn(fetch_and_add(client.clone(), cache.clone(), bookmark)))
        .buffer_unordered(config.settings.max_parallel_requests);

    while let Some(item) = stream.next().await {
        if let Err(err) = item {
            error!("Can't fetch bookmark: {err}");
        }
    }

    Ok(())
}

/// Fetch bookmark and add bookmark to cache if it does not exist yet.
async fn fetch_and_add(
    client: Arc<Client>,
    cache: Arc<Cache>,
    bookmark: Arc<TargetBookmark>,
) -> Result<(), anyhow::Error> {
    let cache_path = cache.get_path(&bookmark);
    let cache_file = Path::new(&cache_path);

    if !cache_file.exists() {
        debug!(
            "Fetch bookmark {} and add to cache at {}",
            bookmark.url,
            cache_file.display()
        );

        match client.fetch(&bookmark).await {
            Ok(website) => {
                let website = html::filter_html(&website)?;
                if let Err(err) = cache.add(website, &bookmark).await {
                    error!("Can't add bookmark {} to cache: {}", bookmark.url, err);
                }
            }
            Err(err) => {
                error!("Can't fetch website ({}): {}", bookmark.url, err);
            }
        }
    }

    Ok(())
}

pub async fn fetch_and_add_urls(
    config: &Config,
    client: Arc<Client>,
    cache: Arc<Cache>,
    urls: &[&str],
    bookmarks: Rc<parking_lot::Mutex<TargetBookmarks>>,
    now: chrono::DateTime<Utc>,
) -> Result<(), anyhow::Error> {
    let mut bookmarks = bookmarks.lock();

    for url in urls {
        let bookmark = TargetBookmark::new(*url, now, None);
        bookmarks.add(&bookmark);
    }

    let bookmarks = bookmarks
        .bookmarks
        .iter()
        .map(|bookmark| Arc::new(bookmark.clone()))
        .collect::<Vec<_>>();
    let mut stream = stream::iter(bookmarks)
        .map(|bookmark| task::spawn(fetch_and_add(client.clone(), cache.clone(), bookmark)))
        .buffer_unordered(config.settings.max_parallel_requests);

    while let Some(item) = stream.next().await {
        if let Err(err) = item {
            error!("Can't fetch bookmark: {err}");
        }
    }

    Ok(())
}

/// Fetch difference between cached and fetched website, and display changes.
pub async fn fetch_diff(config: &Config, args: FetchArgs) -> Result<(), anyhow::Error> {
    debug!("Diff content for urls: {:#?}", args.urls);
    let target_bookmarks = TargetBookmarks::read(config)?;
    let cache = Cache::new(&config.cache_path, &args.mode)?;
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
