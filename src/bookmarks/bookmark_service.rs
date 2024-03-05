use super::{BookmarkManager, RunMode};
use crate::{
    bookmark_reader::{ReadTarget, SourceReader, WriteTarget},
    errors::BogrepError,
    Action, BookmarkProcessor, Caching, Fetch, Status,
};
use chrono::{DateTime, Utc};
use log::debug;
use url::Url;

#[derive(Debug, Default)]
pub struct ServiceConfig {
    run_mode: RunMode,
    ignored_urls: Vec<Url>,
}

impl ServiceConfig {
    pub fn new(run_mode: RunMode, ignored_urls: Vec<Url>) -> Self {
        Self {
            run_mode,
            ignored_urls,
        }
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
    manager: BookmarkManager,
    processor: BookmarkProcessor<C, F>,
}

impl<C, F> BookmarkService<C, F>
where
    C: Caching,
    F: Fetch,
{
    pub fn new(
        config: ServiceConfig,
        manager: BookmarkManager,
        processor: BookmarkProcessor<C, F>,
    ) -> Self {
        Self {
            config,
            manager,
            processor,
        }
    }

    pub async fn run(
        &mut self,
        source_readers: &mut [SourceReader],
        target_reader: &mut impl ReadTarget,
        target_writer: &mut impl WriteTarget,
        now: DateTime<Utc>,
    ) -> Result<(), BogrepError> {
        let bookmark_manager = &mut self.manager;
        let bookmark_processor = &self.processor;

        target_reader.read(bookmark_manager.target_bookmarks_mut())?;

        Self::set_actions(
            &self.config,
            self.processor.cache(),
            bookmark_manager,
            source_readers,
            now,
        )?;

        bookmark_processor
            .process_bookmarks(
                bookmark_manager
                    .target_bookmarks_mut()
                    .values_mut()
                    .collect(),
            )
            .await?;

        bookmark_manager.print_report(&source_readers, self.config.run_mode());
        bookmark_manager.finish();

        target_writer.write(self.manager.target_bookmarks_mut())?;

        Ok(())
    }

    fn set_actions(
        config: &ServiceConfig,
        cache: &impl Caching,
        bookmark_manager: &mut BookmarkManager,
        source_readers: &mut [SourceReader],
        now: DateTime<Utc>,
    ) -> Result<(), BogrepError> {
        if cache.is_empty() {
            debug!("Cache is empty");
            bookmark_manager.target_bookmarks_mut().reset_cache_status();
        }

        match &config.run_mode {
            RunMode::Import => {
                bookmark_manager.import(source_readers)?;
                bookmark_manager.add_bookmarks(now)?;
                bookmark_manager.remove_bookmarks();
                bookmark_manager
                    .target_bookmarks_mut()
                    .set_action(&Action::None);
            }
            RunMode::AddUrls(urls) => {
                bookmark_manager.add_urls(urls, now)?;
            }
            RunMode::RemoveUrls(urls) => {
                bookmark_manager.remove_urls(urls)?;
            }
            RunMode::FetchUrls(_urls) => {
                todo!()
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
            RunMode::FetchDiff => {
                todo!()
            }
            RunMode::Update => {
                todo!()
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
            match target_bookmark.status() {
                Status::Removed => target_bookmark.set_action(Action::Remove),
                Status::Added | Status::None => (),
            }
        }

        // TODO: fix ignored urls for same hosts
        for url in &config.ignored_urls {
            if let Some(target_bookmark) = bookmark_manager.target_bookmarks_mut().get_mut(&url) {
                target_bookmark.set_status(Status::Removed);
                target_bookmark.set_action(Action::Remove);
            }
        }

        Ok(())
    }
}
