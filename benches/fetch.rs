use bogrep::{cmd, Cache, CacheMode, MockClient, TargetBookmark, TargetBookmarks};
use chrono::Utc;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::{HashMap, HashSet};
use tempfile::tempdir;

async fn fetch(max_concurrent_requests: usize) {
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
            bogrep::Action::Add,
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

fn bench_fetch(c: &mut Criterion) {
    let max_concurrent_requests = 1;

    c.bench_with_input(
        BenchmarkId::new("fetch", max_concurrent_requests),
        &max_concurrent_requests,
        |b, s| {
            b.to_async(tokio::runtime::Runtime::new().expect("Can't create tokio runtime"))
                .iter(|| fetch(*s));
        },
    );
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_fetch
);
criterion_main!(benches);
