use crate::{
    bookmark_reader::{ReadTarget, SourceReader, WriteTarget},
    bookmarks::Status,
    errors::BogrepError,
    Action, SourceBookmark, SourceBookmarks, SourceType, TargetBookmark, TargetBookmarks,
};
use chrono::{DateTime, Utc};
use log::{trace, warn};
use std::collections::HashSet;
use url::Url;

#[derive(Debug)]
pub struct BookmarkManager {
    source_bookmarks: SourceBookmarks,
    target_bookmarks: TargetBookmarks,
    dry_run: bool,
}

impl BookmarkManager {
    pub fn new(dry_run: bool) -> Self {
        let source_bookmarks = SourceBookmarks::default();
        let target_bookmarks = TargetBookmarks::default();

        Self {
            source_bookmarks,
            target_bookmarks,
            dry_run,
        }
    }

    pub fn read(&mut self, target_reader: &mut impl ReadTarget) -> Result<(), BogrepError> {
        target_reader.read(&mut self.target_bookmarks)?;
        Ok(())
    }

    pub fn write(&self, target_writer: &mut impl WriteTarget) -> Result<(), BogrepError> {
        target_writer.write(&self.target_bookmarks)?;
        Ok(())
    }

    /// Import bookmarks from sources.
    pub fn import(&mut self, source_readers: &mut [SourceReader]) -> Result<(), BogrepError> {
        for source_reader in source_readers.iter_mut() {
            source_reader.import(&mut self.source_bookmarks)?;
        }

        Ok(())
    }

    pub fn source_bookmarks(&self) -> &SourceBookmarks {
        &self.source_bookmarks
    }

    pub fn source_bookmarks_mut(&mut self) -> &mut SourceBookmarks {
        &mut self.source_bookmarks
    }

    pub fn target_bookmarks(&self) -> &TargetBookmarks {
        &self.target_bookmarks
    }

    pub fn target_bookmarks_mut(&mut self) -> &mut TargetBookmarks {
        &mut self.target_bookmarks
    }

    /// Prepare bookmarks for processing in `BookmarkProcessor`.
    pub fn set_actions(&mut self) {
        if self.dry_run {
            self.target_bookmarks.set_action(&Action::DryRun);
        }
    }

    /// Remove bookmarks which are marked as `Status::Removed`.
    pub fn finish(&mut self) {
        self.target_bookmarks.clean_up();
    }

    pub fn add_urls(
        &mut self,
        urls: &[Url],
        source_type: &SourceType,
        action: &Action,
        now: DateTime<Utc>,
    ) {
        let cache_modes = HashSet::new();
        let mut sources = HashSet::new();
        sources.insert(source_type.to_owned());

        for url in urls {
            let bookmark = TargetBookmark::new(
                url.clone(),
                None,
                now,
                None,
                sources.clone(),
                cache_modes.clone(),
                Status::Added,
                action.clone(),
            );
            self.target_bookmarks.insert(bookmark);
        }
    }

    pub fn remove_urls(&mut self, urls: &[Url]) {
        for url in urls {
            if let Some(target_bookmark) = self.target_bookmarks.get_mut(url) {
                target_bookmark.status = Status::Added;
            }
        }
    }

    // TODO: fix ignored urls for same hosts
    pub fn ignore_urls(&mut self, ignored_urls: &[Url]) {
        for url in ignored_urls {
            if let Some(target_bookmark) = self.target_bookmarks.get_mut(url) {
                target_bookmark.set_status(Status::Removed);
                target_bookmark.set_action(Action::Remove);
            }
        }
    }

    pub fn add_bookmark(
        &mut self,
        source_bookmark: &SourceBookmark,
        now: DateTime<Utc>,
    ) -> Result<(), BogrepError> {
        let url = Url::parse(&source_bookmark.url)?;
        let target_bookmark = TargetBookmark::new(
            url,
            None,
            now,
            None,
            source_bookmark.sources.to_owned(),
            HashSet::new(),
            Status::Added,
            Action::FetchAndAdd,
        );
        self.target_bookmarks.insert(target_bookmark);
        Ok(())
    }

    pub fn add_bookmarks(&mut self, now: DateTime<Utc>) -> Result<(), BogrepError> {
        let bookmarks_to_add = Self::filter_to_add(&self.source_bookmarks, &self.target_bookmarks);
        trace!(
            "Added new bookmarks: {:#?}",
            bookmarks_to_add
                .iter()
                .map(|bookmark| bookmark.url.to_owned())
                .collect::<Vec<_>>()
        );

        for source_bookmark in bookmarks_to_add {
            let url = Url::parse(&source_bookmark.url)?;
            let target_bookmark = TargetBookmark::new(
                url,
                None,
                now,
                None,
                source_bookmark.sources.to_owned(),
                HashSet::new(),
                Status::Added,
                Action::FetchAndAdd,
            );
            self.target_bookmarks.insert(target_bookmark);
        }

        Ok(())
    }

    pub fn remove_bookmarks(&mut self) {
        let bookmarks_to_remove =
            Self::filter_to_remove(&self.source_bookmarks, &mut self.target_bookmarks);
        trace!(
            "Removed bookmarks: {:#?}",
            bookmarks_to_remove
                .iter()
                .map(|bookmark| bookmark.url.to_owned())
                .collect::<Vec<_>>()
        );

        for bookmark in bookmarks_to_remove {
            bookmark.status = Status::Removed;
            bookmark.action = Action::Remove;
        }
    }

    pub fn print_report(&self, source_readers: &[SourceReader]) {
        let added_bookmarks = self
            .target_bookmarks
            .values()
            .filter(|target_bookmark| target_bookmark.status == Status::Added)
            .collect::<Vec<_>>();
        let removed_bookmarks = self
            .target_bookmarks
            .values()
            .filter(|target_bookmark| target_bookmark.status == Status::Removed)
            .collect::<Vec<_>>();
        let added_count = added_bookmarks.len();
        let removed_count = removed_bookmarks.len();
        let source_count = source_readers.len();
        let sources = source_readers
            .iter()
            .map(|source_reader| source_reader.source().path.to_string_lossy())
            .collect::<Vec<_>>()
            .join(", ");
        let source_str = if source_count == 1 {
            "source"
        } else {
            "sources"
        };
        let dry_run_str = if self.dry_run { " (dry run)" } else { "" };

        if !added_bookmarks.is_empty() {
            println!("Added {added_count} new bookmarks");
        }

        if !removed_bookmarks.is_empty() {
            println!("Removed {removed_count} bookmarks");
        }

        if added_bookmarks.is_empty() && removed_bookmarks.is_empty() {
            println!("Bookmarks are already up to date");
        }

        if source_count == 0 {
            println!(
                "Imported {added_count} bookmarks from {source_count} {source_str}{dry_run_str}"
            );
        } else {
            println!(
                "Imported {added_count} bookmarks from {source_count} {source_str}{dry_run_str}: {sources}",
            );
        }
    }

    fn filter_to_add<'a>(
        source_bookmarks: &'a SourceBookmarks,
        target_bookmarks: &TargetBookmarks,
    ) -> Vec<&'a SourceBookmark> {
        source_bookmarks
            .iter()
            .filter_map(|(url, bookmark)| match Url::parse(url) {
                Ok(url) => {
                    if !target_bookmarks.contains_key(&url) {
                        Some(bookmark)
                    } else {
                        None
                    }
                }
                Err(err) => {
                    warn!("{}", BogrepError::ParseUrl(err));
                    None
                }
            })
            .collect()
    }

    fn filter_to_remove<'a>(
        source_bookmarks: &SourceBookmarks,
        target_bookmarks: &'a mut TargetBookmarks,
    ) -> Vec<&'a mut TargetBookmark> {
        target_bookmarks
            .iter_mut()
            .filter(|(url, _)| !source_bookmarks.contains_key(url.as_str()))
            .map(|(_, target_bookmark)| target_bookmark)
            .collect()
    }
}
