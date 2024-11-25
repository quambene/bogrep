use super::{BookmarkManager, RunMode};
use crate::{
    bookmark_reader::{ReadTarget, SourceReader, WriteTarget},
    errors::BogrepError,
    html, utils, Action, Caching, Fetch, ServiceReport, Source, SourceType, Status, TargetBookmark,
    TargetBookmarkBuilder,
};
use chrono::{DateTime, Utc};
use colored::Colorize;
use futures::{stream, StreamExt};
use log::{debug, trace, warn};
use parking_lot::Mutex;
use similar::{ChangeTag, TextDiff};
use std::{error::Error, io::Write, rc::Rc};
use url::Url;

#[derive(Debug, Default)]
pub struct ServiceConfig {
    run_mode: RunMode,
    ignored_urls: Vec<Url>,
    max_concurrent_requests: usize,
}

impl ServiceConfig {
    pub fn new(
        run_mode: RunMode,
        ignored_urls: &[String],
        max_concurrent_requests: usize,
    ) -> Result<Self, BogrepError> {
        let ignored_urls = utils::parse_urls(ignored_urls)?;

        Ok(Self {
            run_mode,
            ignored_urls,
            max_concurrent_requests,
        })
    }

    pub fn run_mode(&self) -> &RunMode {
        &self.run_mode
    }

    pub fn ignored_urls(&self) -> &[Url] {
        &self.ignored_urls
    }
}

pub struct BookmarkService<C: Caching, F: Fetch> {
    config: ServiceConfig,
    client: F,
    cache: C,
    underlying_bookmarks: Rc<Mutex<Vec<TargetBookmark>>>,
    report: Rc<Mutex<ServiceReport>>,
}

impl<C, F> BookmarkService<C, F>
where
    C: Caching,
    F: Fetch,
{
    pub fn new(config: ServiceConfig, client: F, cache: C) -> Self {
        let underlying_bookmarks = vec![];
        let report = ServiceReport {
            dry_run: config.run_mode == RunMode::DryRun,
            ..Default::default()
        };

        Self {
            config,
            client,
            cache,
            underlying_bookmarks: Rc::new(Mutex::new(underlying_bookmarks)),
            report: Rc::new(Mutex::new(report)),
        }
    }

    pub async fn run(
        &self,
        bookmark_manager: &mut BookmarkManager,
        source_readers: &mut [SourceReader],
        target_reader: &mut impl ReadTarget,
        target_writer: &mut impl WriteTarget,
        now: DateTime<Utc>,
    ) -> Result<(), BogrepError> {
        let sources = source_readers
            .iter()
            .map(|source_reader| source_reader.source().clone())
            .collect::<Vec<_>>();

        bookmark_manager.import(source_readers, target_reader, now)?;

        debug!("Imported bookmarks: {bookmark_manager:?}");

        self.process(bookmark_manager, &sources, now).await?;

        bookmark_manager.export(target_writer)?;

        Ok(())
    }

    /// Process all imported bookmarks.
    pub async fn process(
        &self,
        bookmark_manager: &mut BookmarkManager,
        sources: &[Source],
        now: DateTime<Utc>,
    ) -> Result<(), BogrepError> {
        self.set_actions(bookmark_manager, now)?;

        match self.config.run_mode {
            RunMode::Import
            | RunMode::RemoveUrls(_)
            | RunMode::Fetch
            | RunMode::FetchAll
            | RunMode::FetchUrls(_)
            | RunMode::FetchDiff(_)
            | RunMode::Sync
            | RunMode::DryRun => {
                self.execute_actions(bookmark_manager).await?;
                self.add_underlyings(bookmark_manager);

                if !self.underlying_bookmarks.lock().is_empty() {
                    println!("Processing underlying bookmarks");
                    self.execute_actions(bookmark_manager).await?;
                }
            }
            _ => (),
        }

        bookmark_manager.print_report(sources, self.config.run_mode());
        bookmark_manager.finish();

        Ok(())
    }

    fn set_actions(
        &self,
        bookmark_manager: &mut BookmarkManager,
        now: DateTime<Utc>,
    ) -> Result<(), BogrepError> {
        if self.cache.is_empty() {
            debug!("Cache is empty");
            bookmark_manager.target_bookmarks_mut().reset_cache_status();
        }

        match self.config.run_mode() {
            RunMode::Import => {
                bookmark_manager
                    .target_bookmarks_mut()
                    .set_action(&Action::None);
            }
            RunMode::AddUrls(urls) => {
                bookmark_manager.add_urls(urls, self.cache.mode(), &Action::None, now);
            }
            RunMode::RemoveUrls(urls) => {
                bookmark_manager.remove_urls(urls);
            }
            RunMode::Remove => {
                bookmark_manager
                    .target_bookmarks_mut()
                    .set_action(&Action::Remove);
            }
            RunMode::RemoveAll => {
                bookmark_manager
                    .target_bookmarks_mut()
                    .set_action(&Action::RemoveAll);
            }
            RunMode::FetchUrls(urls) => {
                bookmark_manager.add_urls(urls, self.cache.mode(), &Action::FetchAndAdd, now);
            }
            RunMode::Fetch => {
                bookmark_manager
                    .target_bookmarks_mut()
                    .set_action(&Action::FetchAndAdd);
            }
            RunMode::FetchAll => {
                bookmark_manager
                    .target_bookmarks_mut()
                    .set_action(&Action::FetchAndReplace);
            }
            RunMode::FetchDiff(urls) => {
                bookmark_manager.add_urls(urls, self.cache.mode(), &Action::FetchAndDiff, now);
            }
            RunMode::Sync => {
                bookmark_manager
                    .target_bookmarks_mut()
                    .set_action(&Action::FetchAndReplace);
            }
            RunMode::DryRun => {
                bookmark_manager
                    .target_bookmarks_mut()
                    .set_action(&Action::DryRun);
            }
            RunMode::None => {
                bookmark_manager
                    .target_bookmarks_mut()
                    .set_action(&Action::None);
            }
        }

        for target_bookmark in bookmark_manager.target_bookmarks_mut().values_mut() {
            if self.config.run_mode != RunMode::DryRun {
                match target_bookmark.status() {
                    Status::Removed => target_bookmark.set_action(Action::Remove),
                    Status::Added | Status::None => (),
                }
            }
        }

        for ignored_url in self.config.ignored_urls() {
            let ignored_bookmarks = bookmark_manager
                .target_bookmarks_mut()
                .values_mut()
                .filter(|bookmark| bookmark.url.host() == ignored_url.host());

            for bookmark in ignored_bookmarks {
                bookmark.set_status(Status::Removed);

                if self.config.run_mode != RunMode::DryRun {
                    bookmark.set_action(Action::Remove);
                }
            }
        }

        Ok(())
    }

    /// Execute `Action`s for provided bookmarks.
    async fn execute_actions(
        &self,
        bookmark_manager: &mut BookmarkManager,
    ) -> Result<(), BogrepError> {
        let max_concurrent_requests = self.config.max_concurrent_requests;
        let bookmarks = bookmark_manager
            .target_bookmarks_mut()
            .values_mut()
            .filter(|bookmark| bookmark.action() != &Action::None)
            .collect::<Vec<_>>();

        if bookmarks.is_empty() {
            return Ok(());
        }

        {
            let mut report = self.report.lock();
            report.reset();
            report.set_total(bookmarks.len());
        }

        let mut stream = stream::iter(bookmarks)
            .map(|bookmark| self.execute_action(bookmark))
            .buffer_unordered(max_concurrent_requests);

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

    /// Fetch and add bookmark to cache.
    async fn execute_action<'a>(
        &self,
        bookmark: &'a mut TargetBookmark,
    ) -> Result<(), BogrepError> {
        let client = &self.client;
        let cache = &self.cache;

        match bookmark.action() {
            Action::FetchAndReplace => {
                let website = client.fetch(bookmark).await?;
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
            //  Fetch difference between cached and fetched website, and display
            //  changes.
            Action::FetchAndDiff => {
                if let Some(website_before) = cache.get(bookmark)? {
                    let fetched_website = client.fetch(bookmark).await?;
                    trace!("Fetched website: {fetched_website}");
                    let html = html::filter_html(&fetched_website)?;
                    let website_after = cache.replace(html, bookmark).await?;
                    Self::diff_websites(&website_before, &website_after);
                }
            }
            Action::Remove => {
                cache.remove(bookmark).await?;
            }
            Action::RemoveAll => {
                cache.remove_by_modes(bookmark).await?;
            }
            // We don't reset the action to `Action::None` in a dry run.
            Action::DryRun => return Ok(()),
            Action::None => (),
        }

        bookmark.set_action(Action::None);

        Ok(())
    }

    fn diff_websites(before: &str, after: &str) {
        let diff = TextDiff::from_lines(before, after);

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

    fn add_underlyings(&self, bookmark_manager: &mut BookmarkManager) {
        let target_bookmarks = bookmark_manager.target_bookmarks_mut();
        let underlying_bookmarks = self.underlying_bookmarks.lock();

        for underlying_bookmark in underlying_bookmarks.iter() {
            target_bookmarks.insert(underlying_bookmark.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CacheMode, MockCache, MockClient, Settings};
    use std::collections::HashMap;

    fn create_mock_client(urls: &[Url], content: &str) -> MockClient {
        let client = MockClient::new();

        for url in urls {
            client
                .add(
                    format!(
                        "<html><head></head><body><img></img><p>{}</p></body></html>",
                        content
                    ),
                    url,
                )
                .unwrap();
        }

        client
    }

    async fn create_mock_cache(
        cache_mode: CacheMode,
        content: Option<&str>,
        bookmark_manager: &mut BookmarkManager,
    ) -> MockCache {
        let cache = MockCache::new(cache_mode);

        if let Some(content) = content {
            cache
                .add(
                    format!("<html><head></head><body><p>{}</p></body></html>", content),
                    bookmark_manager
                        .target_bookmarks_mut()
                        .get_mut(&Url::parse("https://url1.com").unwrap())
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        cache
    }

    fn create_mock_manager(urls: &[Url], status: &[Status]) -> BookmarkManager {
        let now = Utc::now();
        let mut bookmark_manager = BookmarkManager::default();

        bookmark_manager.target_bookmarks_mut().insert(
            TargetBookmark::builder_with_id(
                "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                urls[0].to_owned(),
                now,
            )
            .with_status(status[0].clone())
            .with_action(Action::None)
            .build(),
        );
        bookmark_manager.target_bookmarks_mut().insert(
            TargetBookmark::builder_with_id(
                "25b6357e-6eda-4367-8212-84376c6efe05".to_owned(),
                urls[1].to_owned(),
                now,
            )
            .with_status(status[1].clone())
            .with_action(Action::None)
            .build(),
        );

        if urls.len() == 3 {
            bookmark_manager.target_bookmarks_mut().insert(
                TargetBookmark::builder_with_id(
                    "a4d8f19b-92c1-4e68-a6e9-7d60b54024bc".to_owned(),
                    urls[2].to_owned(),
                    now,
                )
                .with_status(status[2].clone())
                .with_action(Action::None)
                .build(),
            );
        }

        bookmark_manager
    }

    #[tokio::test]
    async fn test_set_actions_import() {
        let now = Utc::now();
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let url3 = Url::parse("https://url3.com").unwrap();
        let urls = vec![url1.clone(), url2.clone(), url3.clone()];
        let settings = Settings::default();
        let service_config = ServiceConfig::new(
            RunMode::Import,
            &settings.ignored_urls,
            settings.max_concurrent_requests,
        )
        .unwrap();
        let mut bookmark_manager =
            create_mock_manager(&urls, &[Status::Added, Status::Removed, Status::None]);
        let client = create_mock_client(&urls, "Test content");
        let cache = create_mock_cache(CacheMode::Html, None, &mut bookmark_manager).await;
        let service = BookmarkService::new(service_config, client, cache);

        let res = service.set_actions(&mut bookmark_manager, now);
        assert!(res.is_ok());

        let bookmarks = bookmark_manager.target_bookmarks();
        assert_eq!(bookmarks.get(&url1).unwrap().action, Action::None);
        assert_eq!(bookmarks.get(&url2).unwrap().action, Action::Remove);
        assert_eq!(bookmarks.get(&url3).unwrap().action, Action::None);
    }

    #[tokio::test]
    async fn test_set_actions_fetch() {
        let now = Utc::now();
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let url3 = Url::parse("https://url3.com").unwrap();
        let urls = vec![url1.clone(), url2.clone(), url3.clone()];
        let settings = Settings::default();
        let service_config = ServiceConfig::new(
            RunMode::Fetch,
            &settings.ignored_urls,
            settings.max_concurrent_requests,
        )
        .unwrap();
        let mut bookmark_manager =
            create_mock_manager(&urls, &[Status::Added, Status::Removed, Status::None]);
        let client = create_mock_client(&urls, "Test content");
        let cache = create_mock_cache(CacheMode::Html, None, &mut bookmark_manager).await;
        let service = BookmarkService::new(service_config, client, cache);

        let res = service.set_actions(&mut bookmark_manager, now);
        assert!(res.is_ok());

        let bookmarks = bookmark_manager.target_bookmarks();
        assert_eq!(bookmarks.get(&url1).unwrap().action, Action::FetchAndAdd);
        assert_eq!(bookmarks.get(&url2).unwrap().action, Action::Remove);
        assert_eq!(bookmarks.get(&url3).unwrap().action, Action::FetchAndAdd);
    }

    #[tokio::test]
    async fn test_set_actions_dry_run() {
        let now = Utc::now();
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let url3 = Url::parse("https://url3.com").unwrap();
        let urls = vec![url1.clone(), url2.clone(), url3.clone()];
        let settings = Settings::default();
        let service_config = ServiceConfig::new(
            RunMode::DryRun,
            &settings.ignored_urls,
            settings.max_concurrent_requests,
        )
        .unwrap();
        let mut bookmark_manager =
            create_mock_manager(&urls, &[Status::Added, Status::Removed, Status::None]);
        let client = create_mock_client(&urls, "Test content");
        let cache = create_mock_cache(CacheMode::Html, None, &mut bookmark_manager).await;
        let service = BookmarkService::new(service_config, client, cache);

        let res = service.set_actions(&mut bookmark_manager, now);
        assert!(res.is_ok());
        assert!(bookmark_manager
            .target_bookmarks()
            .values()
            .any(|bookmark| bookmark.action == Action::DryRun));
    }

    #[tokio::test]
    async fn test_process_fetch_ignored_urls() {
        let now = Utc::now();
        let url1 = Url::parse("https://url.com").unwrap();
        let url2 = Url::parse("https://url.com/endpoint").unwrap();
        let urls = vec![url1, url2];
        let settings = Settings::default();
        let service_config = ServiceConfig::new(
            RunMode::Fetch,
            &["https://url.com".to_owned()],
            settings.max_concurrent_requests,
        )
        .unwrap();
        let mut bookmark_manager = create_mock_manager(&urls, &[Status::None, Status::None]);
        let client = create_mock_client(&urls, "Test content");
        let cache = create_mock_cache(CacheMode::Html, None, &mut bookmark_manager).await;
        let service = BookmarkService::new(service_config, client, cache);

        let res = service.process(&mut bookmark_manager, &[], now).await;
        assert!(res.is_ok());
        assert!(bookmark_manager.target_bookmarks().is_empty());
    }

    #[tokio::test]
    async fn test_process_fetch_html() {
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let urls = vec![url1, url2];
        let now = Utc::now();
        let settings = Settings::default();
        let service_config = ServiceConfig::new(
            RunMode::Fetch,
            &settings.ignored_urls,
            settings.max_concurrent_requests,
        )
        .unwrap();
        let mut bookmark_manager = create_mock_manager(&urls, &[Status::None, Status::None]);
        let client = create_mock_client(&urls, "Test content");
        let cache = create_mock_cache(CacheMode::Html, None, &mut bookmark_manager).await;
        let service = BookmarkService::new(service_config, client, cache);

        let res = service.process(&mut bookmark_manager, &[], now).await;
        assert!(res.is_ok());
        assert_eq!(
            service.cache.cache_map(),
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
        );
        assert!(bookmark_manager
            .target_bookmarks()
            .values()
            .all(|bookmark| bookmark.last_cached.is_some()));
    }

    #[tokio::test]
    async fn test_process_fetch_text() {
        let now = Utc::now();
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let urls = vec![url1, url2];
        let settings = Settings::default();
        let service_config = ServiceConfig::new(
            RunMode::Fetch,
            &settings.ignored_urls,
            settings.max_concurrent_requests,
        )
        .unwrap();
        let mut bookmark_manager = create_mock_manager(&urls, &[Status::None, Status::None]);
        let client = create_mock_client(&urls, "Test content");
        let cache = create_mock_cache(CacheMode::Text, None, &mut bookmark_manager).await;
        let service = BookmarkService::new(service_config, client, cache);

        let res = service.process(&mut bookmark_manager, &[], now).await;
        assert!(res.is_ok());
        assert_eq!(
            service.cache.cache_map(),
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
        );
        assert!(bookmark_manager
            .target_bookmarks()
            .values()
            .all(|bookmark| bookmark.last_cached.is_some()));
    }

    #[tokio::test]
    async fn test_process_fetch_cached_html() {
        let now = Utc::now();
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let urls = vec![url1, url2];
        let settings = Settings::default();
        let service_config = ServiceConfig::new(
            RunMode::Fetch,
            &settings.ignored_urls,
            settings.max_concurrent_requests,
        )
        .unwrap();
        let mut bookmark_manager = create_mock_manager(&urls, &[Status::None, Status::None]);
        let client = create_mock_client(&urls, "Test content (fetched)");
        let cache = create_mock_cache(
            CacheMode::Html,
            Some("Test content (already cached)"),
            &mut bookmark_manager,
        )
        .await;
        let service = BookmarkService::new(service_config, client, cache);

        let res = service.process(&mut bookmark_manager, &[], now).await;
        assert!(res.is_ok());
        assert_eq!(
            service.cache.cache_map(),
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
        assert!(bookmark_manager
            .target_bookmarks()
            .values()
            .all(|bookmark| bookmark.last_cached.is_some()));
    }

    #[tokio::test]
    async fn test_process_fetch_cached_text() {
        let now = Utc::now();
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let urls = vec![url1, url2];
        let settings = Settings::default();
        let service_config = ServiceConfig::new(
            RunMode::Fetch,
            &settings.ignored_urls,
            settings.max_concurrent_requests,
        )
        .unwrap();
        let mut bookmark_manager = create_mock_manager(&urls, &[Status::None, Status::None]);
        let client = create_mock_client(&urls, "Test content (fetched)");
        let cache = create_mock_cache(
            CacheMode::Text,
            Some("Test content (already cached)"),
            &mut bookmark_manager,
        )
        .await;
        let service = BookmarkService::new(service_config, client, cache);

        let res = service.process(&mut bookmark_manager, &[], now).await;
        assert!(res.is_ok());
        assert_eq!(
            service.cache.cache_map(),
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
        assert!(bookmark_manager
            .target_bookmarks()
            .values()
            .all(|bookmark| bookmark.last_cached.is_some()));
    }
}
