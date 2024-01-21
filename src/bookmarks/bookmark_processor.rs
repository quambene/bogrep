use std::{collections::HashSet, error::Error, io::Write, rc::Rc};

use crate::{
    errors::BogrepError, html, Action, Caching, Fetch, SourceType, TargetBookmark, TargetBookmarks,
};
use chrono::Utc;
use futures::{stream, StreamExt};
use log::{debug, trace, warn};
use parking_lot::Mutex;

#[derive(Debug)]
pub struct BookmarkProcessor<C: Caching, F: Fetch> {
    client: F,
    cache: C,
    max_concurrent_requests: usize,
    underlying_bookmarks: Rc<Mutex<Vec<TargetBookmark>>>,
}

impl<C, F> BookmarkProcessor<C, F>
where
    F: Fetch,
    C: Caching,
{
    pub fn new(client: F, cache: C, max_concurrent_requests: usize) -> Self
    where
        F: Fetch,
        C: Caching,
    {
        Self {
            client,
            cache,
            max_concurrent_requests,
            underlying_bookmarks: Rc::new(Mutex::new(vec![])),
        }
    }

    pub fn add_underlyings(self, target_bookmarks: &mut TargetBookmarks) {
        let underlying_bookmarks = self.underlying_bookmarks.lock();

        for underlying_bookmark in underlying_bookmarks.iter() {
            target_bookmarks.insert(underlying_bookmark.clone());
        }
    }

    /// Process bookmarks for all actions except [`Action::None`].
    pub async fn process_bookmarks(
        &self,
        bookmarks: Vec<&mut TargetBookmark>,
    ) -> Result<(), BogrepError> {
        let bookmarks = bookmarks
            .into_iter()
            .filter(|bookmark| bookmark.action != Action::None)
            .collect::<Vec<_>>();
        let mut processed = 0;
        let mut cached = 0;
        let mut failed_response = 0;
        let mut binary_response = 0;
        let mut empty_response = 0;
        let total = bookmarks.len();

        let mut stream = stream::iter(bookmarks)
            .map(|bookmark| self.execute_actions(bookmark))
            .buffer_unordered(self.max_concurrent_requests);

        while let Some(item) = stream.next().await {
            processed += 1;

            print!("Processing bookmarks ({processed}/{total})\r");

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

    /// Fetch and add bookmark to cache.
    async fn execute_actions(&self, bookmark: &mut TargetBookmark) -> Result<(), BogrepError> {
        let client = &self.client;
        let cache = &self.cache;

        match bookmark.action {
            Action::FetchAndReplace => {
                let website = self.client.fetch(bookmark).await?;
                trace!("Fetched website: {website}");
                self.fetch_underlying(bookmark, &website).await?;
                let html = html::filter_html(&website)?;
                cache.replace(html, bookmark).await?;
            }
            Action::FetchAndAdd => {
                if !cache.exists(bookmark) {
                    let website = client.fetch(bookmark).await?;
                    trace!("Fetched website: {website}");
                    self.fetch_underlying(bookmark, &website).await?;
                    let html = html::filter_html(&website)?;
                    cache.add(html, bookmark).await?;
                }
            }
            Action::Remove => {
                cache.remove(bookmark).await?;
            }
            Action::None => (),
        }

        bookmark.action = Action::None;

        Ok(())
    }

    async fn fetch_underlying(
        &self,
        bookmark: &mut TargetBookmark,
        website: &str,
    ) -> Result<(), BogrepError> {
        let client = &self.client;
        let cache = &self.cache;

        if bookmark.underlying_url.is_none() {
            let underlying_url = html::select_underlying(&website, &bookmark.underlying_type)?;

            if let Some(underlying_url) = underlying_url {
                bookmark.underlying_url = Some(underlying_url.clone());

                let mut underlying_bookmark = TargetBookmark::new(
                    underlying_url.clone(),
                    None,
                    Utc::now(),
                    None,
                    HashSet::new(),
                    HashSet::new(),
                    Action::FetchAndAdd,
                );
                underlying_bookmark.set_source(SourceType::Underlying(bookmark.url.to_string()));

                if !cache.exists(&underlying_bookmark) {
                    let website = client.fetch(&underlying_bookmark).await?;
                    let html = html::filter_html(&website)?;
                    cache.add(html, &mut underlying_bookmark).await?;
                }

                let mut underlying_bookmarks = self.underlying_bookmarks.lock();
                underlying_bookmarks.push(underlying_bookmark);
            }
        }

        Ok(())
    }
}
