use super::{BookmarkManager, RunMode};
use crate::{
    bookmark_reader::{ReadTarget, SourceReader, WriteTarget},
    errors::BogrepError,
    Action, BookmarkProcessor, Caching, Fetch,
};
use chrono::{DateTime, Utc};

pub struct BookmarkService<C: Caching, F: Fetch> {
    run_mode: RunMode,
    manager: BookmarkManager,
    processor: BookmarkProcessor<C, F>,
}

impl<C, F> BookmarkService<C, F>
where
    C: Caching,
    F: Fetch,
{
    pub fn new(
        run_mode: RunMode,
        manager: BookmarkManager,
        processor: BookmarkProcessor<C, F>,
    ) -> Self {
        Self {
            run_mode,
            manager,
            processor,
        }
    }

    pub async fn run(
        &mut self,
        now: DateTime<Utc>,
        source_readers: &mut [SourceReader],
        target_reader: &mut impl ReadTarget,
        target_writer: &mut impl WriteTarget,
    ) -> Result<(), BogrepError> {
        let bookmark_manager = &mut self.manager;
        let bookmark_processor = &self.processor;

        target_reader.read(bookmark_manager.target_bookmarks_mut())?;

        match &self.run_mode {
            RunMode::Import => {
                bookmark_manager.import(source_readers)?;
                bookmark_manager.add_bookmarks(now)?;
                bookmark_manager.remove_bookmarks();
                bookmark_manager
                    .target_bookmarks_mut()
                    .set_action(&Action::None);
            }
            RunMode::AddUrls(urls) => {
                todo!()
            }
            RunMode::RemoveUrls(urls) => {
                todo!()
            }
            RunMode::FetchUrls(urls) => {
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

        bookmark_manager.set_actions(now);

        bookmark_processor
            .process_bookmarks(
                bookmark_manager
                    .target_bookmarks_mut()
                    .values_mut()
                    .collect(),
            )
            .await?;

        bookmark_manager.print_report(&source_readers);
        bookmark_manager.finish();

        target_writer.write(self.manager.target_bookmarks_mut())?;

        Ok(())
    }

    fn set_actions(&self, bookmark_manager: &mut BookmarkManager) {
        todo!()
    }
}
