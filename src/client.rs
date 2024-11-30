use crate::{bookmarks::TargetBookmark, errors::BogrepError, Settings};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use log::{debug, error, trace};
use parking_lot::Mutex;
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, HOST, USER_AGENT},
    Client as ReqwestClient, Url,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};
use tokio::{
    task,
    time::{self, Duration},
};

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
#[derive(Clone)]
pub struct Client {
    client: ReqwestClient,
    browser: Browser,
    throttler: Option<Throttler>,
}

impl Client {
    pub fn new(config: &ClientConfig) -> Result<Self, BogrepError> {
        let request_timeout = config.request_timeout;
        let request_throttling = config.request_throttling;
        let client = ReqwestClient::builder()
            .timeout(Duration::from_millis(request_timeout))
            .pool_idle_timeout(Duration::from_millis(config.idle_connections_timeout))
            .pool_max_idle_per_host(config.max_idle_connections_per_host)
            .build()
            .map_err(BogrepError::CreateClient)?;
        let launch_options = LaunchOptionsBuilder::default()
            .headless(true)
            // .devtools(false)
            .idle_browser_timeout(Duration::from_secs(300))
            .build()
            .context("Can't build launch options")?;
        let browser = Browser::new(launch_options)?;
        let throttler = Some(Throttler::new(request_throttling));
        Ok(Self {
            client,
            browser,
            throttler,
        })
    }
}

#[async_trait]
impl Fetch for Client {
    async fn fetch(&self, bookmark: &TargetBookmark) -> Result<String, BogrepError> {
        debug!("Fetch bookmark ({})", bookmark.url());

        if let Some(throttler) = &self.throttler {
            throttler.throttle(bookmark).await?;
        }

        // let mut headers = HeaderMap::new();
        // headers.insert(
        //     USER_AGENT,
        //     HeaderValue::from_static(
        //         "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:132.0) Gecko/20100101 Firefox/132.0",
        //     ),
        // );
        // headers.insert(
        //     ACCEPT,
        //     HeaderValue::from_static(
        //         "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
        //     ),
        // );
        // headers.insert(
        //     ACCEPT_LANGUAGE,
        //     HeaderValue::from_static("en-US,en;q=0.7,de-DE;q=0.3"),
        // );
        // headers.insert(
        //     ACCEPT_ENCODING,
        //     HeaderValue::from_static("gzip,deflate,br,zstd"),
        // );
        // headers.insert(
        //     HeaderName::from_static("sec-fetch-dest"),
        //     HeaderValue::from_static("document"),
        // );
        // headers.insert(
        //     HeaderName::from_static("sec-fetch-mode"),
        //     HeaderValue::from_static("navigate"),
        // );
        // headers.insert(
        //     HeaderName::from_static("sec-fetch-site"),
        //     HeaderValue::from_static("none"),
        // );
        // headers.insert(
        //     HeaderName::from_static("upgrade-insecure-requests"),
        //     HeaderValue::from_static("1"),
        // );
        // headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        // headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
        // headers.insert(
        //     HOST,
        //     HeaderValue::from_str(
        //         bookmark
        //             .url()
        //             .host()
        //             .context("Can't get host")?
        //             .to_string()
        //             .as_str(),
        //     )
        //     .context("Can't get host header")?,
        // );

        // let request = self.client.get(bookmark.url().to_owned()).headers(headers);

        debug!("Fetch bookmark: {}", bookmark.url());
        // trace!(
        //     "Fetch bookmark ({}) with request: {:#?}",
        //     bookmark.url(),
        //     request.build().unwrap()
        // );

        let browser = self.browser.clone();
        let bookmark_url = bookmark.url().clone();

        let content = task::spawn_blocking(move || {
            let tab = browser.new_tab()?;
            tab.navigate_to(bookmark_url.as_str())?;
            tab.wait_until_navigated()?;
            let html = tab.get_content();
            html
        })
        .await;

        match content {
            Ok(Ok(html)) => Ok(html),
            Ok(Err(err)) => {
                error!("Can't fetch website: {err}");
                Err(BogrepError::EmptyResponse(bookmark.url().to_string()))
            }
            Err(err) => {
                error!("Can't get content: {err}");
                Err(BogrepError::EmptyResponse(bookmark.url().to_string()))
            }
        }

        // let response = self
        //     .client
        //     .get(bookmark.url().to_owned())
        //     .send()
        //     .await
        //     .map_err(BogrepError::HttpResponse)?;

        // if response.status().is_success() {
        //     if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
        //         let content_type = content_type.to_str()?;

        //         if !(content_type.starts_with("application/")
        //             || content_type.starts_with("image/")
        //             || content_type.starts_with("audio/")
        //             || content_type.starts_with("video/"))
        //         {
        //             let html = response
        //                 .text()
        //                 .await
        //                 .map_err(BogrepError::ParseHttpResponse)?;

        //             if !html.is_empty() {
        //                 Ok(html)
        //             } else {
        //                 Err(BogrepError::EmptyResponse(bookmark.url().to_string()))
        //             }
        //         } else {
        //             Err(BogrepError::BinaryResponse(bookmark.url().to_string()))
        //         }
        //     } else {
        //         Err(BogrepError::BinaryResponse(bookmark.url().to_string()))
        //     }
        // } else {
        //     Err(BogrepError::HttpStatus {
        //         status: response.status().to_string(),
        //         url: bookmark.url().to_string(),
        //     })
        // }
    }
}

/// A throttler to limit the number of requests.
#[derive(Debug, Clone)]
struct Throttler {
    /// The times in milliseconds at which the next request to the domain is
    /// allowed to be executed.
    next_request_times: Arc<Mutex<HashMap<String, i64>>>,
    /// The throttling between requests in milliseconds.
    request_throttling: u64,
}

impl Throttler {
    pub fn new(request_throttling: u64) -> Self {
        Self {
            next_request_times: Arc::new(Mutex::new(HashMap::new())),
            request_throttling,
        }
    }

    /// Wait some time before fetching bookmarks for the same host to prevent rate limiting.
    pub async fn throttle(&self, bookmark: &TargetBookmark) -> Result<(), BogrepError> {
        debug!("Throttle bookmark ({})", bookmark.url());
        let now = Utc::now();

        if let Some(next_fetch_time) = self.update_fetch_time(bookmark, now)? {
            let duration_until_next_fetch = next_fetch_time - now.timestamp_millis();

            if duration_until_next_fetch > 0 {
                debug!(
                    "Wait {duration_until_next_fetch} milliseconds for bookmark ({})",
                    bookmark.url()
                );
                time::sleep(Duration::from_millis(duration_until_next_fetch as u64)).await;
            }
        }

        Ok(())
    }

    /// Update the fetch time and return previous value.
    fn update_fetch_time(
        &self,
        bookmark: &TargetBookmark,
        now: DateTime<Utc>,
    ) -> Result<Option<i64>, BogrepError> {
        let bookmark_host = bookmark
            .url()
            .host_str()
            .ok_or(BogrepError::ConvertHost(bookmark.url().to_string()))?;

        let mut next_request_times = self.next_request_times.lock();
        let entry = next_request_times.entry(bookmark_host.to_string());

        match entry {
            Entry::Occupied(mut entry) => {
                let next_request_time = entry.get_mut();
                let last_request_time = *next_request_time;

                if now.timestamp_millis() < *next_request_time {
                    *next_request_time += self.request_throttling as i64;
                }

                Ok(Some(last_request_time))
            }
            Entry::Vacant(entry) => {
                let next_request_time = now.timestamp_millis() + self.request_throttling as i64;
                entry.insert(next_request_time);
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
    use tokio::{time::Instant, try_join};

    #[tokio::test]
    async fn test_throttle() {
        tokio::time::pause();
        let now = Utc::now();
        let request_throttling = 1000;
        let url1 = Url::parse("https://url/path1.com").unwrap();
        let url2 = Url::parse("https://url/path2.com").unwrap();
        let url3 = Url::parse("https://url/path3.com").unwrap();
        let throttler = Throttler::new(request_throttling);
        let bookmark1 = TargetBookmark::new(url1, now);
        let bookmark2 = TargetBookmark::new(url2, now);
        let bookmark3 = TargetBookmark::new(url3, now);

        let start_instant = Instant::now();

        try_join!(
            throttler.throttle(&bookmark1),
            throttler.throttle(&bookmark2),
            throttler.throttle(&bookmark3)
        )
        .unwrap();

        assert_eq!(
            Instant::now().duration_since(start_instant).as_millis(),
            2001
        );
    }

    #[test]
    fn test_last_fetched() {
        let now = Utc::now();
        let request_throttling = 1000;
        let url1 = Url::parse("https://url/path1.com").unwrap();
        let url2 = Url::parse("https://url/path2.com").unwrap();
        let url3 = Url::parse("https://url/path3.com").unwrap();
        let throttler = Throttler::new(request_throttling);
        let bookmark1 = TargetBookmark::new(url1, now);
        let bookmark2 = TargetBookmark::new(url2, now);
        let bookmark3 = TargetBookmark::new(url3, now);

        let last_fetched = throttler.update_fetch_time(&bookmark1, now).unwrap();
        assert!(last_fetched.is_none());

        let last_fetched = throttler.update_fetch_time(&bookmark2, now).unwrap();
        assert_eq!(last_fetched, Some(now.timestamp_millis() + 1000));

        let last_fetched = throttler.update_fetch_time(&bookmark3, now).unwrap();
        assert_eq!(last_fetched, Some(now.timestamp_millis() + 2000));
    }
}
