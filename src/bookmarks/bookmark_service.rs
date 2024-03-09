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

        self.process(bookmark_manager, &sources, now).await?;

        bookmark_manager.export(target_writer)?;

        Ok(())
    }

    /// Process all imported bookmarks.
    async fn process(
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
            | RunMode::Update => {
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

        for target_bookmark in bookmark_manager.target_bookmarks_mut().values_mut() {
            match target_bookmark.status() {
                Status::Removed => target_bookmark.set_action(Action::Remove),
                Status::Added | Status::None => (),
            }
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
            RunMode::Update => {
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

        // TODO: fix ignored urls for same hosts
        for url in self.config.ignored_urls() {
            if let Some(target_bookmark) = bookmark_manager.target_bookmarks_mut().get_mut(url) {
                target_bookmark.set_status(Status::Removed);
                target_bookmark.set_action(Action::Remove);
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

        {
            let mut report = self.report.lock();
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
    ) -> Result<&'a mut TargetBookmark, BogrepError> {
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
            // We don't reset the action to `Action::None` in a dry run.
            Action::DryRun => return Ok(bookmark),
            Action::None => (),
        }

        bookmark.set_action(Action::None);

        Ok(bookmark)
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

    fn create_mock_service(
        run_mode: &RunMode,
        settings: &Settings,
    ) -> BookmarkService<MockCache, MockClient> {
        let client = MockClient::new();
        let cache_mode = CacheMode::new(&None, &settings.cache_mode);
        let cache = MockCache::new(cache_mode);
        let service_config = ServiceConfig::new(
            run_mode.to_owned(),
            &settings.ignored_urls,
            settings.max_concurrent_requests,
        )
        .unwrap();
        let service = BookmarkService::new(service_config, client, cache);
        service
    }
}
