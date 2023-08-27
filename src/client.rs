use crate::bookmarks::TargetBookmark;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use log::debug;
use reqwest::{Client as ReqwestClient, Url};
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Mutex,
};
use tokio::time::{self, Duration};

/// The request timeout in milliseconds.
const REQUEST_TIMEOUT: u64 = 60_000;

/// The throttling between requests in milliseconds.
const THROTTLING: u64 = 1_000;

pub struct Client {
    client: ReqwestClient,
    throttler: Option<Throttler>,
}

impl Client {
    pub fn new() -> Result<Self, anyhow::Error> {
        let client = ReqwestClient::builder()
            .timeout(Duration::from_millis(REQUEST_TIMEOUT))
            .build()?;
        let throttler = Some(Throttler::new());
        Ok(Self { client, throttler })
    }

    pub async fn fetch(&self, bookmark: &TargetBookmark) -> Result<String, anyhow::Error> {
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
}

impl Throttler {
    pub fn new() -> Self {
        Self {
            last_fetched: Mutex::new(HashMap::new()),
        }
    }

    /// Wait some time before fetching bookmarks for the same host to prevent rate limiting.
    pub async fn throttle(&self, bookmark: &TargetBookmark) -> Result<(), anyhow::Error> {
        debug!("Throttle bookmark {}", bookmark.url);
        let now = Utc::now();

        if let Some(last_fetched) = self.last_fetched(bookmark, now)? {
            let duration_since_last_fetched = now - last_fetched;

            if duration_since_last_fetched < chrono::Duration::milliseconds(THROTTLING as i64 / 2) {
                debug!("Wait for bookmark {}", bookmark.url);
                time::sleep(Duration::from_millis(THROTTLING)).await;
            } else if chrono::Duration::milliseconds(THROTTLING as i64 / 2)
                < duration_since_last_fetched
                && duration_since_last_fetched < chrono::Duration::milliseconds(THROTTLING as i64)
            {
                debug!("Wait for bookmark {}", bookmark.url);
                time::sleep(Duration::from_millis(THROTTLING / 2)).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Instant;

    #[tokio::test]
    async fn test_throttle() {
        tokio::time::pause();
        let now = Utc::now();
        let throttler = Throttler::new();
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
            (THROTTLING + 1) as u128
        );
    }

    #[test]
    fn test_last_fetched() {
        let now = Utc::now();
        let throttler = Throttler::new();
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
