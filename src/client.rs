use crate::{bookmarks::TargetBookmark, errors::BogrepError, utils, Settings};
use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::debug;
use parking_lot::Mutex;
use reqwest::{Client as ReqwestClient, Url};
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};
use tokio::time::{self, Duration};

/// A trait to fetch websites from a real or mock client.
#[async_trait]
pub trait Fetch: Clone {
    /// Fetch content of a website as HTML.
    async fn fetch(&self, bookmark: &TargetBookmark) -> Result<String, BogrepError>;
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// The request timeout in milliseconds.
    pub request_timeout: u64,
    /// The throttling between requests in milliseconds.
    pub request_throttling: u64,
    /// The maximum number of idle connections allowed in the connection pool.
    pub max_idle_connections_per_host: usize,
    /// The timeout for idle connections to be kept alive in milliseconds.
    pub idle_connections_timeout: u64,
}

impl ClientConfig {
    pub fn new(settings: &Settings) -> Self {
        Self {
            request_timeout: settings.request_timeout,
            request_throttling: settings.request_throttling,
            max_idle_connections_per_host: settings.max_idle_connections_per_host,
            idle_connections_timeout: settings.idle_connections_timeout,
        }
    }
}

/// A client to fetch websites.
#[derive(Debug, Clone)]
pub struct Client {
    client: ReqwestClient,
    throttler: Option<Throttler>,
}

impl Client {
    pub fn new(config: &ClientConfig) -> Result<Self, BogrepError> {
        let request_timeout = config.request_timeout;
        let request_throttling = config.request_throttling;
        let client = ReqwestClient::builder()
            .timeout(Duration::from_millis(request_timeout))
            // Fix "Too many open files" and DNS errors (rate limit for DNS
            // server) by choosing a sensible value for `pool_idle_timeout()`
            // and `pool_max_idle_per_host()`.
            .pool_idle_timeout(Duration::from_millis(config.idle_connections_timeout))
            .pool_max_idle_per_host(config.max_idle_connections_per_host)
            .build()
            .map_err(BogrepError::CreateClient)?;
        let throttler = Some(Throttler::new(request_throttling));
        Ok(Self { client, throttler })
    }
}

#[async_trait]
impl Fetch for Client {
    async fn fetch(&self, bookmark: &TargetBookmark) -> Result<String, BogrepError> {
        debug!("Fetch bookmark ({})", bookmark.url());

        if let Some(throttler) = &self.throttler {
            throttler.throttle(bookmark).await?;
        }

        let response = self
            .client
            .get(bookmark.url().to_owned())
            .send()
            .await
            .map_err(BogrepError::HttpResponse)?;

        if response.status().is_success() {
            if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
                let content_type = content_type.to_str()?;

                if !(content_type.starts_with("application/")
                    || content_type.starts_with("image/")
                    || content_type.starts_with("audio/")
                    || content_type.starts_with("video/"))
                {
                    let html = response
                        .text()
                        .await
                        .map_err(BogrepError::ParseHttpResponse)?;

                    if !html.is_empty() {
                        Ok(html)
                    } else {
                        Err(BogrepError::EmptyResponse(bookmark.url().to_string()))
                    }
                } else {
                    Err(BogrepError::BinaryResponse(bookmark.url().to_string()))
                }
            } else {
                Err(BogrepError::BinaryResponse(bookmark.url().to_string()))
            }
        } else {
            Err(BogrepError::HttpStatus {
                status: response.status().to_string(),
                url: bookmark.url().to_string(),
            })
        }
    }
}

/// A throttler to limit the number of requests.
#[derive(Debug, Clone)]
struct Throttler {
    last_fetched: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
    /// The throttling between requests in milliseconds.
    request_throttling: u64,
}

impl Throttler {
    pub fn new(request_throttling: u64) -> Self {
        Self {
            last_fetched: Arc::new(Mutex::new(HashMap::new())),
            request_throttling,
        }
    }

    /// Wait some time before fetching bookmarks for the same host to prevent rate limiting.
    pub async fn throttle(&self, bookmark: &TargetBookmark) -> Result<(), BogrepError> {
        debug!("Throttle bookmark ({})", bookmark.url());
        let now = Utc::now();

        if let Some(last_fetched) = self.last_fetched(bookmark, now)? {
            let duration_since_last_fetched = now - last_fetched;
            let throttling = utils::convert_to_duration(self.request_throttling as i64)?;

            if duration_since_last_fetched < throttling / 2 {
                debug!("Wait for bookmark ({})", bookmark.url());
                time::sleep(Duration::from_millis(self.request_throttling)).await;
            } else if throttling / 2 < duration_since_last_fetched
                && duration_since_last_fetched < throttling
            {
                debug!("Wait for bookmark ({})", bookmark.url());
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
    ) -> Result<Option<DateTime<Utc>>, BogrepError> {
        let bookmark_host = bookmark
            .url()
            .host_str()
            .ok_or(BogrepError::ConvertHost(bookmark.url().to_string()))?;

        let mut map = self.last_fetched.lock();
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

/// A mock client to fetch websites used in testing.
#[derive(Debug, Default, Clone)]
pub struct MockClient {
    /// Mock the the HTML content.
    client_map: Arc<Mutex<HashMap<Url, String>>>,
}

impl MockClient {
    pub fn new() -> Self {
        let client_map = Arc::new(Mutex::new(HashMap::new()));
        Self { client_map }
    }

    pub fn add(&self, html: String, bookmark_url: &Url) -> Result<(), anyhow::Error> {
        let mut client_map = self.client_map.lock();
        client_map.insert(bookmark_url.clone(), html);
        Ok(())
    }

    pub fn get(&self, bookmark_url: &Url) -> Option<String> {
        let client_map = self.client_map.lock();
        client_map
            .get(bookmark_url)
            .map(|content| content.to_owned())
    }
}

#[async_trait]
impl Fetch for MockClient {
    async fn fetch(&self, bookmark: &TargetBookmark) -> Result<String, BogrepError> {
        let html = self
            .get(bookmark.url())
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
        let url1 = Url::parse("https://url/path1.com").unwrap();
        let url2 = Url::parse("https://url/path2.com").unwrap();
        let throttler = Throttler::new(request_throttling);
        let bookmark1 = TargetBookmark::new(url1, now);
        let bookmark2 = TargetBookmark::new(url2, now);

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
        let url1 = Url::parse("https://url/path1.com").unwrap();
        let url2 = Url::parse("https://url/path2.com").unwrap();
        let throttler = Throttler::new(request_throttling);
        let bookmark1 = TargetBookmark::new(url1, now);
        let bookmark2 = TargetBookmark::new(url2, now);

        let last_fetched = throttler.last_fetched(&bookmark1, now).unwrap();
        assert!(last_fetched.is_none());

        let last_fetched = throttler.last_fetched(&bookmark2, now).unwrap();
        assert_eq!(last_fetched, Some(now));
    }
}
