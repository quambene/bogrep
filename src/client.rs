use crate::{bookmarks::TargetBookmark, Config};
use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::debug;
use reqwest::{Client as ReqwestClient, Url};
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Mutex,
};
use tokio::time::{self, Duration};

#[async_trait]
pub trait Fetch {
    /// Fetch content of a website as HTML.
    async fn fetch(&self, bookmark: &TargetBookmark) -> Result<String, anyhow::Error>;
}

pub struct Client {
    client: ReqwestClient,
    throttler: Option<Throttler>,
}

impl Client {
    pub fn new(config: &Config) -> Result<Self, anyhow::Error> {
        let request_timeout = config.settings.request_timeout;
        let request_throttling = config.settings.request_throttling;
        let client = ReqwestClient::builder()
            .timeout(Duration::from_millis(request_timeout))
            // Workaround for "Too many open files" (see <https://github.com/hyperium/hyper/issues/2312>).
            .pool_idle_timeout(Duration::from_millis(0))
            // Workaround for "Too many open files" (see <https://github.com/hyperium/hyper/issues/2312>).
            .pool_max_idle_per_host(0)
            .build()?;
        let throttler = Some(Throttler::new(request_throttling));
        Ok(Self { client, throttler })
    }
}

#[async_trait]
impl Fetch for Client {
    async fn fetch(&self, bookmark: &TargetBookmark) -> Result<String, anyhow::Error> {
        debug!("Fetch bookmark: {}", bookmark.url);

        if let Some(throttler) = &self.throttler {
            throttler.throttle(bookmark).await?;
        }

        let response = self.client.get(&bookmark.url).send().await?;
        let html = response.text().await?;

        Ok(html)
    }
}

#[derive(Debug)]
struct Throttler {
    last_fetched: Mutex<HashMap<String, DateTime<Utc>>>,
    request_throttling: u64,
}

impl Throttler {
    pub fn new(request_throttling: u64) -> Self {
        Self {
            last_fetched: Mutex::new(HashMap::new()),
            request_throttling,
        }
    }

    /// Wait some time before fetching bookmarks for the same host to prevent rate limiting.
    pub async fn throttle(&self, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        debug!("Throttle bookmark {}", bookmark.url);
        let now = Utc::now();

        if let Some(last_fetched) = self.last_fetched(bookmark, now)? {
            let duration_since_last_fetched = now - last_fetched;

            if duration_since_last_fetched
                < chrono::Duration::milliseconds(self.request_throttling as i64 / 2)
            {
                debug!("Wait for bookmark {}", bookmark.url);
                time::sleep(Duration::from_millis(self.request_throttling)).await;
            } else if chrono::Duration::milliseconds(self.request_throttling as i64 / 2)
                < duration_since_last_fetched
                && duration_since_last_fetched
                    < chrono::Duration::milliseconds(self.request_throttling as i64)
            {
                debug!("Wait for bookmark {}", bookmark.url);
                time::sleep(Duration::from_millis(self.request_throttling / 2)).await;
            }
        }

        Ok(())
    }

    /// Update last_fetched timestamp and return previous value.
    fn last_fetched(
        &self,
        bookmark: &TargetBookmark,
        now: DateTime<Utc>,
    ) -> Result<Option<DateTime<Utc>>, anyhow::Error> {
        let bookmark_url = Url::parse(&bookmark.url)?;
        let bookmark_host = bookmark_url.host_str().ok_or(anyhow!("Can't get host"))?;

        let mut map = self.last_fetched.lock().unwrap();
        let entry = map.entry(bookmark_host.to_string());

        match entry {
            Entry::Occupied(mut entry) => {
                let last_fetched = entry.insert(now);
                Ok(Some(last_fetched))
            }
            Entry::Vacant(entry) => {
                entry.insert(now);
                Ok(None)
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct MockClient {
    /// Mock the the HTML content.
    client_map: Mutex<HashMap<String, String>>,
}

impl MockClient {
    pub fn new() -> Self {
        let client_map = Mutex::new(HashMap::new());
        Self { client_map }
    }

    pub fn add(&self, html: String, bookmark_url: &str) -> Result<(), anyhow::Error> {
        let mut client_map = self.client_map.lock().unwrap();
        client_map.insert(bookmark_url.to_owned(), html);
        Ok(())
    }

    pub fn get(&self, bookmark_url: &str) -> Option<String> {
        let client_map = self.client_map.lock().unwrap();
        client_map
            .get(bookmark_url)
            .map(|content| content.to_owned())
    }
}

#[async_trait]
impl Fetch for MockClient {
    async fn fetch(&self, bookmark: &TargetBookmark) -> Result<String, anyhow::Error> {
        let html = self
            .get(&bookmark.url)
            .ok_or(anyhow!("Can't fetch bookmark"))?;
        Ok(html)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Instant;

    #[tokio::test]
    async fn test_throttle() {
        tokio::time::pause();
        let now = Utc::now();
        let request_throttling = 1000;
        let throttler = Throttler::new(request_throttling);
        let bookmark1 = TargetBookmark::new(
            "https://en.wikipedia.org/wiki/Applicative_functor",
            now,
            None,
        );
        let bookmark2 = TargetBookmark::new(
            "https://en.wikipedia.org/wiki/Monad_(functional_programming)",
            now,
            None,
        );

        let start_instant = Instant::now();
        throttler.throttle(&bookmark1).await.unwrap();
        assert_eq!(Instant::now().duration_since(start_instant).as_millis(), 0);

        let start_instant = Instant::now();
        throttler.throttle(&bookmark2).await.unwrap();
        assert_eq!(
            Instant::now().duration_since(start_instant).as_millis(),
            (request_throttling + 1) as u128
        );
    }

    #[test]
    fn test_last_fetched() {
        let now = Utc::now();
        let request_throttling = 1000;
        let throttler = Throttler::new(request_throttling);
        let bookmark1 = TargetBookmark::new(
            "https://en.wikipedia.org/wiki/Applicative_functor",
            now,
            None,
        );
        let bookmark2 = TargetBookmark::new(
            "https://en.wikipedia.org/wiki/Monad_(functional_programming)",
            now,
            None,
        );

        let last_fetched = throttler.last_fetched(&bookmark1, now).unwrap();
        assert!(last_fetched.is_none());

        let last_fetched = throttler.last_fetched(&bookmark2, now).unwrap();
        assert_eq!(last_fetched, Some(now));
    }
}
