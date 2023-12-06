use bogrep::{
    cmd, errors::BogrepError, html, Cache, CacheMode, Caching, Fetch, MockClient, TargetBookmark,
    TargetBookmarks,
};
use chrono::Utc;
use criterion::{criterion_group, criterion_main, Criterion};
use futures::{stream, StreamExt};
use log::{debug, trace, warn};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    io::Write,
    sync::Arc,
};
use tempfile::tempdir;
use tokio::sync::Mutex;

fn bench_fetch(c: &mut Criterion) {
    c.bench_function("concurrent 10", |b| {
        b.iter(|| fetch_concurrently(10));
    });

    c.bench_function("parallel 10", |b| {
        b.iter(|| fetch_in_parallel(10));
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_fetch
);
criterion_main!(benches);

async fn fetch_concurrently(max_concurrent_requests: usize) {
    let now = Utc::now();
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    let cache_path = temp_path.join("cache");
    let cache = Cache::new(&cache_path, CacheMode::Text);
    let client = MockClient::new();

    let url1 = "https://url1.com";
    let content1 = "<html><head></head><body><p>Test content 1</p></body></html>";
    client.add(content1.to_owned(), url1).unwrap();

    let mut bookmarks = TargetBookmarks::new(HashMap::from_iter([(
        url1.to_owned(),
        TargetBookmark::new(
            url1,
            now,
            None,
            HashSet::new(),
            HashSet::new(),
            bogrep::Action::Fetch,
        ),
    )]));

    cmd::fetch_and_cache_bookmarks(
        &client,
        &cache,
        bookmarks.values_mut().collect(),
        max_concurrent_requests,
    )
    .await
    .unwrap();
}

async fn fetch_in_parallel(max_parallel_requests: usize) {
    let now = Utc::now();
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());
    let cache_path = temp_path.join("cache");
    let cache = Cache::new(&cache_path, CacheMode::Text);
    let client = MockClient::new();

    let url1 = "https://url1.com";
    let content1 = "<html><head></head><body><p>Test content 1</p></body></html>";
    client.add(content1.to_owned(), url1).unwrap();

    let bookmarks = TargetBookmarks::new(HashMap::from_iter([(
        url1.to_owned(),
        TargetBookmark::new(
            url1,
            now,
            None,
            HashSet::new(),
            HashSet::new(),
            bogrep::Action::Fetch,
        ),
    )]));
    let bookmarks = bookmarks
        .iter()
        .map(|bookmark| Arc::new(Mutex::new(bookmark.1.clone())))
        .collect::<Vec<_>>();

    let client = Arc::new(client);
    let cache = Arc::new(cache);

    fetch_and_cache_bookmarks_in_parallel(client, cache, &bookmarks, max_parallel_requests, true)
        .await
        .unwrap();
}

pub async fn fetch_and_cache_bookmarks_in_parallel(
    client: Arc<impl Fetch + Send + Sync + 'static>,
    cache: Arc<impl Caching + Send + Sync + 'static>,
    bookmarks: &[Arc<Mutex<TargetBookmark>>],
    max_parallel_requests: usize,
    fetch_all: bool,
) -> Result<(), BogrepError> {
    let mut processed = 0;
    let mut cached = 0;
    let mut failed_response = 0;
    let mut binary_response = 0;
    let mut empty_response = 0;
    let total = bookmarks.len();

    let mut stream = stream::iter(bookmarks)
        .map(|bookmark| {
            tokio::spawn(fetch_and_cache_bookmark(
                client.clone(),
                cache.clone(),
                bookmark.clone(),
                fetch_all,
            ))
        })
        .buffer_unordered(max_parallel_requests);

    while let Some(item) = stream.next().await {
        processed += 1;

        print!("Processing bookmarks ({processed}/{total})\r");

        if let Ok(Err(err)) = item {
            match err {
                BogrepError::HttpResponse(ref error) => {
                    // Usually, a lot of fetching errors are expected because of
                    // invalid or outdated urls in the bookmarks, so we are
                    // using a warning message only if the issue is on our side.
                    if let Some(error) = error.source() {
                        if error.to_string().contains("Too many open files") {
                            warn!("{err}");
                        } else {
                            debug!("{err} ");
                        }
                    } else {
                        debug!("{err} ");
                    }

                    failed_response += 1;
                }
                BogrepError::HttpStatus { .. } => {
                    debug!("{err}");
                    failed_response += 1;
                }
                BogrepError::ParseHttpResponse(_) => {
                    debug!("{err}");
                    failed_response += 1;
                }
                BogrepError::BinaryResponse(_) => {
                    debug!("{err}");
                    binary_response += 1;
                }
                BogrepError::EmptyResponse(_) => {
                    debug!("{err}");
                    empty_response += 1;
                }
                BogrepError::ConvertHost(_) => {
                    warn!("{err}");
                    failed_response += 1;
                }
                BogrepError::CreateFile { .. } => {
                    // Write errors are expected if there are "Too many open
                    // files", so we are issuing a warning instead of returning
                    // a hard failure.
                    warn!("{err}");
                    failed_response += 1;
                }
                // We are aborting if there is an unexpected error.
                err => {
                    return Err(err);
                }
            }
        } else {
            cached += 1;
        }

        std::io::stdout().flush().map_err(BogrepError::FlushFile)?;
    }

    println!();
    println!(
        "Processed {total} bookmarks, {cached} cached, {} ignored, {failed_response} failed",
        binary_response + empty_response
    );

    Ok(())
}

async fn fetch_and_cache_bookmark(
    client: Arc<impl Fetch>,
    cache: Arc<impl Caching>,
    bookmark: Arc<Mutex<TargetBookmark>>,
    fetch_all: bool,
) -> Result<(), BogrepError> {
    let mut bookmark = bookmark.lock().await;

    if fetch_all {
        let website = client.fetch(&bookmark).await?;
        trace!("Fetched website: {website}");
        let html = html::filter_html(&website)?;
        cache.replace(html, &mut bookmark).await?;
    } else if !cache.exists(&bookmark) {
        let website = client.fetch(&bookmark).await?;
        trace!("Fetched website: {website}");
        let html = html::filter_html(&website)?;
        cache.add(html, &mut bookmark).await?;
    }

    Ok(())
}
