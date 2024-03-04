use crate::{
    bookmark_reader::SourceReader,
    bookmarks::{target_bookmarks::TargetBookmarkBuilder, Status},
    errors::BogrepError,
    Action, SourceBookmark, SourceBookmarks, SourceType, TargetBookmark, TargetBookmarks,
};
use chrono::{DateTime, Utc};
use log::{debug, trace, warn};
use url::Url;

use super::RunMode;

#[derive(Debug, Default)]
pub struct RunConfig {
    run_mode: RunMode,
    empty_cache: bool,
    ignored_urls: Vec<Url>,
    fetch_urls: Vec<Url>,
}

impl RunConfig {
    pub fn new(
        run_mode: RunMode,
        empty_cache: bool,
        ignored_urls: Vec<Url>,
        fetch_urls: Vec<Url>,
    ) -> Self {
        Self {
            run_mode,
            empty_cache,
            ignored_urls,
            fetch_urls,
        }
    }
}

#[derive(Debug)]
pub struct BookmarkManager {
    config: RunConfig,
    source_bookmarks: SourceBookmarks,
    target_bookmarks: TargetBookmarks,
}

impl BookmarkManager {
    pub fn new(config: RunConfig) -> Self {
        let source_bookmarks = SourceBookmarks::default();
        let target_bookmarks = TargetBookmarks::default();

        Self {
            config,
            source_bookmarks,
            target_bookmarks,
        }
    }

    pub fn is_dry_run(&self) -> bool {
        self.config.run_mode == RunMode::DryRun
    }

    /// Import bookmarks from sources.
    pub fn import(&mut self, source_readers: &mut [SourceReader]) -> Result<(), BogrepError> {
        for source_reader in source_readers.iter_mut() {
            source_reader.import(&mut self.source_bookmarks)?;
        }

        Ok(())
    }

    pub fn target_bookmarks(&self) -> &TargetBookmarks {
        &self.target_bookmarks
    }

    pub fn target_bookmarks_mut(&mut self) -> &mut TargetBookmarks {
        &mut self.target_bookmarks
    }

    /// Prepare bookmarks for processing in `BookmarkProcessor`.
    pub fn set_actions(&mut self, now: DateTime<Utc>) {
        for target_bookmark in self.target_bookmarks.values_mut() {
            match target_bookmark.status() {
                Status::Removed => target_bookmark.set_action(Action::Remove),
                Status::Added | Status::None => (),
            }
        }

        // TODO: fix ignored urls for same hosts
        for url in &self.config.ignored_urls {
            if let Some(target_bookmark) = self.target_bookmarks.get_mut(&url) {
                target_bookmark.set_status(Status::Removed);
                target_bookmark.set_action(Action::Remove);
            }
        }

        for url in &self.config.fetch_urls {
            if let Some(target_bookmark) = self.target_bookmarks.get_mut(url) {
                target_bookmark.set_action(Action::FetchAndReplace);
                target_bookmark.add_source(SourceType::Internal);
            } else {
                let target_bookmark = TargetBookmarkBuilder::new(url.to_owned(), now)
                    .add_source(SourceType::Internal)
                    .with_status(Status::Added)
                    .with_action(Action::FetchAndReplace)
                    .build();
                self.target_bookmarks.insert(target_bookmark);
            }
        }

        if self.config.empty_cache {
            debug!("Cache is empty");
            self.target_bookmarks.reset_cache_status();
        }

        match self.config.run_mode {
            RunMode::Import => {
                self.target_bookmarks.set_action(&Action::None);
            }
            RunMode::Fetch => {
                self.target_bookmarks.set_action(&Action::FetchAndAdd);
            }
            RunMode::FetchAll => {
                self.target_bookmarks.set_action(&Action::FetchAndReplace);
            }
            RunMode::DryRun => {
                self.target_bookmarks.set_action(&Action::DryRun);
            }
            RunMode::None => {
                self.target_bookmarks.set_action(&Action::None);
            }
        }
    }

    /// Remove bookmarks which are marked as [`Status::Removed`].
    pub fn finish(&mut self) {
        let urls_to_remove = self
            .target_bookmarks
            .values()
            .filter_map(|bookmark| {
                if bookmark.status() == &Status::Removed {
                    Some(bookmark.url().to_owned())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for url in urls_to_remove {
            self.target_bookmarks.remove(&url);
        }
    }

    pub fn add_urls(&mut self, urls: &[Url], source_type: &SourceType, now: DateTime<Utc>) {
        for url in urls {
            let target_bookmark = TargetBookmark::builder(url.clone(), now)
                .add_source(source_type.to_owned())
                .build();
            self.target_bookmarks.upsert(target_bookmark);
        }
    }

    pub fn remove_urls(&mut self, urls: &[Url]) {
        for url in urls {
            if let Some(target_bookmark) = self.target_bookmarks.get_mut(url) {
                target_bookmark.set_status(Status::Removed);
                target_bookmark.set_action(Action::Remove);
            }
        }
    }

    /// Add the difference between source and target bookmarks.
    pub fn add_bookmarks(&mut self, now: DateTime<Utc>) -> Result<(), BogrepError> {
        let bookmarks_to_add = Self::filter_to_add(&self.source_bookmarks, &self.target_bookmarks);
        trace!(
            "Added new bookmarks: {:#?}",
            bookmarks_to_add
                .iter()
                .map(|bookmark| bookmark.url().to_owned())
                .collect::<Vec<_>>()
        );

        for source_bookmark in bookmarks_to_add {
            let url = Url::parse(&source_bookmark.url())?;
            let target_bookmark = TargetBookmarkBuilder::new(url, now)
                .with_sources(source_bookmark.sources().to_owned())
                .build();
            self.target_bookmarks.upsert(target_bookmark);
        }

        Ok(())
    }

    /// Remove the difference between source and target bookmarks.
    pub fn remove_bookmarks(&mut self) {
        let bookmarks_to_remove =
            Self::filter_to_remove(&self.source_bookmarks, &mut self.target_bookmarks);
        trace!(
            "Removed bookmarks: {:#?}",
            bookmarks_to_remove
                .iter()
                .map(|bookmark| bookmark.url())
                .collect::<Vec<_>>()
        );

        for bookmark in bookmarks_to_remove {
            bookmark.set_status(Status::Removed);
        }
    }

    pub fn print_report(&self, source_readers: &[SourceReader]) {
        let added_bookmarks = self
            .target_bookmarks
            .values()
            .filter(|target_bookmark| target_bookmark.status() == &Status::Added)
            .collect::<Vec<_>>();
        let removed_bookmarks = self
            .target_bookmarks
            .values()
            .filter(|target_bookmark| target_bookmark.status() == &Status::Removed)
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
        let dry_run_str = match self.config.run_mode {
            RunMode::DryRun => " (dry run)",
            _ => "",
        };

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmarks::SourceBookmarkBuilder;
    use std::{collections::HashMap, str::FromStr};

    #[test]
    fn test_add_and_remove_bookmarks() {
        let now = Utc::now();
        let url1 = Url::from_str("https://url1.com").unwrap();
        let url2 = Url::from_str("https://url2.com").unwrap();
        let url3 = Url::from_str("https://url3.com").unwrap();
        let source_bookmarks = SourceBookmarks::new(HashMap::from_iter([
            (
                url1.to_string(),
                SourceBookmarkBuilder::new(url1.as_str())
                    .add_source(&SourceType::Simple)
                    .build(),
            ),
            (
                url3.to_string(),
                SourceBookmarkBuilder::new(url3.as_str())
                    .add_source(&SourceType::Simple)
                    .build(),
            ),
        ]));
        let target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                url1.clone(),
                TargetBookmark::builder_with_id(
                    "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                    url1.clone(),
                    now,
                )
                .add_source(SourceType::Simple)
                .build(),
            ),
            (
                url2.clone(),
                TargetBookmark::builder_with_id(
                    "511b1590-e6de-4989-bca4-96dc61730508".to_owned(),
                    url2.clone(),
                    now,
                )
                .add_source(SourceType::Simple)
                .build(),
            ),
        ]));

        let mut bookmark_manager = BookmarkManager {
            config: RunConfig::default(),
            source_bookmarks,
            target_bookmarks,
        };

        bookmark_manager.add_bookmarks(now).unwrap();
        let actual_bookmarks = bookmark_manager.target_bookmarks();
        assert_eq!(
            actual_bookmarks,
            &TargetBookmarks::new(HashMap::from_iter([
                (
                    url1.clone(),
                    TargetBookmark::builder_with_id(
                        "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                        url1.clone(),
                        now
                    )
                    .add_source(SourceType::Simple)
                    .build()
                ),
                (
                    url2.clone(),
                    TargetBookmark::builder_with_id(
                        "511b1590-e6de-4989-bca4-96dc61730508".to_owned(),
                        url2.clone(),
                        now
                    )
                    .add_source(SourceType::Simple)
                    .build()
                ),
                (
                    url3.clone(),
                    TargetBookmark::builder_with_id(
                        actual_bookmarks.get(&url3).unwrap().id().to_owned(),
                        url3.clone(),
                        now
                    )
                    .add_source(SourceType::Simple)
                    .with_status(Status::Added)
                    .build()
                ),
            ]))
        );

        bookmark_manager.remove_bookmarks();
        let actual_bookmarks = bookmark_manager.target_bookmarks();
        assert_eq!(
            actual_bookmarks,
            &TargetBookmarks::new(HashMap::from_iter([
                (
                    url1.clone(),
                    TargetBookmark::builder_with_id(
                        "dd30381b-8e67-4e84-9379-0852f60a7cd7".to_owned(),
                        url1.clone(),
                        now
                    )
                    .add_source(SourceType::Simple)
                    .with_status(Status::None)
                    .build()
                ),
                (
                    url2.clone(),
                    TargetBookmark::builder_with_id(
                        "511b1590-e6de-4989-bca4-96dc61730508".to_owned(),
                        url2.clone(),
                        now
                    )
                    .add_source(SourceType::Simple)
                    .with_status(Status::Removed)
                    .build()
                ),
                (
                    url3.clone(),
                    TargetBookmark::builder_with_id(
                        actual_bookmarks.get(&url3).unwrap().id().to_owned(),
                        url3.clone(),
                        now
                    )
                    .add_source(SourceType::Simple)
                    .with_status(Status::Added)
                    .build()
                ),
            ]))
        );
    }
}
