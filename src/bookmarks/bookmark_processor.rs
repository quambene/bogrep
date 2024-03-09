use super::ServiceReport;
use crate::{
    bookmarks::TargetBookmarkBuilder, errors::BogrepError, html, Action, Caching, Fetch, Settings,
    SourceType, TargetBookmark, TargetBookmarks,
};
use chrono::Utc;
use futures::{stream, StreamExt};
use log::{debug, trace, warn};
use parking_lot::Mutex;
use std::{error::Error, io::Write, rc::Rc};

#[derive(Debug)]
pub struct BookmarkProcessor<C: Caching, F: Fetch> {
    client: F,
    cache: C,
    settings: Settings,
    underlying_bookmarks: Rc<Mutex<Vec<TargetBookmark>>>,
    report: Mutex<ServiceReport>,
}

impl<C, F> BookmarkProcessor<C, F>
where
    F: Fetch,
    C: Caching,
{
    pub fn new(client: F, cache: C, settings: Settings, report: ServiceReport) -> Self
    where
        F: Fetch,
        C: Caching,
    {
        Self {
            client,
            cache,
            settings,
            underlying_bookmarks: Rc::new(Mutex::new(vec![])),
            report: Mutex::new(report),
        }
    }

    pub fn cache(&self) -> &impl Caching {
        &self.cache
    }

    pub fn underlying_bookmarks(&self) -> Vec<TargetBookmark> {
        let underlying_bookmarks = self.underlying_bookmarks.lock();
        underlying_bookmarks.clone()
    }

    /// Process bookmarks for all actions except [`Action::None`].
    pub async fn process_bookmarks(
        &self,
        bookmarks: Vec<&mut TargetBookmark>,
    ) -> Result<(), BogrepError> {
        let bookmarks = bookmarks
            .into_iter()
            .filter(|bookmark| bookmark.action() != &Action::None)
            .collect::<Vec<_>>();
        {
            let mut report = self.report.lock();
            report.set_total(bookmarks.len());
        }

        let mut stream = stream::iter(bookmarks)
            .map(|bookmark| self.execute_actions(bookmark))
            .buffer_unordered(self.settings.max_concurrent_requests);

        while let Some(item) = stream.next().await {
            let mut report = self.report.lock();
            report.increment_processed();
            report.print();

            if let Err(err) = item {
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

                        report.increment_failed_response();
                    }
                    BogrepError::HttpStatus { .. } => {
                        debug!("{err}");
                        report.increment_failed_response();
                    }
                    BogrepError::ParseHttpResponse(_) => {
                        debug!("{err}");
                        report.increment_failed_response();
                    }
                    BogrepError::BinaryResponse(_) => {
                        debug!("{err}");
                        report.increment_binary_response();
                    }
                    BogrepError::EmptyResponse(_) => {
                        debug!("{err}");
                        report.increment_empty_response();
                    }
                    BogrepError::ConvertHost(_) => {
                        warn!("{err}");
                        report.increment_failed_response();
                    }
                    BogrepError::CreateFile { .. } => {
                        // Write errors are expected if there are "Too many open
                        // files", so we are issuing a warning instead of returning
                        // a hard failure.
                        warn!("{err}");
                        report.increment_failed_response();
                    }
                    // We are aborting if there is an unexpected error.
                    err => {
                        return Err(err);
                    }
                }
            } else {
                report.increment_cached();
            }

            std::io::stdout().flush().map_err(BogrepError::FlushFile)?;
        }

        self.report.lock().print_summary();

        Ok(())
    }

    /// Process underlying bookmarks for all actions except [`Action::None`].
    pub async fn process_underlyings(
        self,
        target_bookmarks: &mut TargetBookmarks,
    ) -> Result<(), BogrepError> {
        self.add_underlyings(target_bookmarks);

        if self.underlying_bookmarks().is_empty() {
            return Ok(());
        }

        println!("Processing underlying bookmarks");
        self.process_bookmarks(target_bookmarks.values_mut().collect())
            .await?;

        Ok(())
    }

    /// Fetch and add bookmark to cache.
    async fn execute_actions(&self, bookmark: &mut TargetBookmark) -> Result<(), BogrepError> {
        let client = &self.client;
        let cache = &self.cache;

        match bookmark.action() {
            Action::FetchAndReplace => {
                let website = self.client.fetch(bookmark).await?;
                trace!("Fetched website: {website}");
                self.add_underlying(bookmark, &website)?;
                let html = html::filter_html(&website)?;
                cache.replace(html, bookmark).await?;
            }
            Action::FetchAndAdd => {
                if !cache.exists(bookmark) {
                    let website = client.fetch(bookmark).await?;
                    trace!("Fetched website: {website}");
                    self.add_underlying(bookmark, &website)?;
                    let html = html::filter_html(&website)?;
                    cache.add(html, bookmark).await?;
                }
            }
            Action::FetchAndDiff => {
                if let Some(website_before) = cache.get(bookmark)? {
                    todo!()
                }
            }
            Action::Remove => {
                cache.remove(bookmark).await?;
            }
            // We don't reset the action to `Action::None` in a dry run.
            Action::DryRun => return Ok(()),
            Action::None => (),
        }

        bookmark.set_action(Action::None);

        Ok(())
    }

    fn add_underlyings(&self, bookmarks: &mut TargetBookmarks) {
        let underlying_bookmarks = self.underlying_bookmarks.lock();

        for underlying_bookmark in underlying_bookmarks.iter() {
            bookmarks.insert(underlying_bookmark.clone());
        }
    }

    fn add_underlying(
        &self,
        bookmark: &mut TargetBookmark,
        website: &str,
    ) -> Result<(), BogrepError> {
        debug!("Add underlying");

        if bookmark.underlying_url().is_none() {
            let underlying_url = html::select_underlying(website, bookmark.underlying_type())?;

            if let Some(underlying_url) = underlying_url {
                bookmark.set_underlying_url(underlying_url.clone());

                let underlying_bookmark =
                    TargetBookmarkBuilder::new(underlying_url.to_owned(), Utc::now())
                        .add_source(SourceType::Underlying(bookmark.url().to_string()))
                        .with_action(Action::FetchAndAdd)
                        .build();

                debug!("Added underlying bookmark: {underlying_bookmark:#?}");

                let mut underlying_bookmarks = self.underlying_bookmarks.lock();
                underlying_bookmarks.push(underlying_bookmark);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bookmarks::TargetBookmarkBuilder, CacheMode, MockCache, MockClient, UnderlyingType,
    };
    use std::collections::HashSet;
    use url::Url;

    #[test]
    fn test_add_underlying() {
        let now = Utc::now();
        let settings = Settings::default();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Text);
        let bookmark_processor =
            BookmarkProcessor::new(client, cache, settings, ServiceReport::default());
        let url = Url::parse("https://news.ycombinator.com").unwrap();
        let website = r#"
            <html>

            <head>
                <title>title_content</title>
                <meta>
                <script>script_content_1</script>
            </head>

            <body>
                <td class="title">
                    <span class="titleline">
                        <a href="https://underlying_url.com">The underlying article</a>
                        <span class="sitebit comhead"> (<a href="from?site=underlying_url.com">
                                <span class="sitestr">underlying_url.com</span></a>)
                        </span>
                    </span>
                </td>
            </body>

            </html>
        "#;
        let mut bookmark = TargetBookmarkBuilder::new(url.to_owned(), now)
            .add_source(SourceType::Internal)
            .add_cache_mode(CacheMode::Text)
            .build();

        let res = bookmark_processor.add_underlying(&mut bookmark, website);
        assert!(res.is_ok());

        assert!(bookmark
            .underlying_url()
            .is_some_and(|url| url == &Url::parse("https://underlying_url.com").unwrap()));
        assert_eq!(bookmark.underlying_type(), &UnderlyingType::HackerNews);
        assert_eq!(bookmark.sources(), &HashSet::from([SourceType::Internal]));
        assert!(bookmark.last_cached().is_none());

        let underlying_bookmarks = bookmark_processor.underlying_bookmarks();
        assert_eq!(underlying_bookmarks.len(), 1);

        let underlying_bookmark = &underlying_bookmarks[0];
        assert_eq!(
            underlying_bookmark.url(),
            &Url::parse("https://underlying_url.com").unwrap()
        );
        assert!(underlying_bookmark.underlying_url().is_none());
        assert_eq!(underlying_bookmark.underlying_type(), &UnderlyingType::None);
        assert!(underlying_bookmark.last_cached().is_none());
        assert_eq!(
            underlying_bookmark.sources(),
            &HashSet::from_iter([SourceType::Underlying(
                "https://news.ycombinator.com/".to_owned()
            )])
        );
        assert!(underlying_bookmark.cache_modes().is_empty());
        assert_eq!(underlying_bookmark.action(), &Action::FetchAndAdd);
    }
}
