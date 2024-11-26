use super::{RawSource, RunMode};
use crate::{
    bookmark_reader::{ReadTarget, SourceReader, WriteTarget},
    bookmarks::{target_bookmarks::TargetBookmarkBuilder, Status},
    errors::BogrepError,
    Action, CacheMode, SourceBookmark, SourceBookmarks, SourceType, TargetBookmark,
    TargetBookmarks,
};
use chrono::{DateTime, Utc};
use log::{trace, warn};
use url::Url;

#[derive(Debug, Default)]
pub struct BookmarkManager {
    target_bookmarks: TargetBookmarks,
    source_readers: Vec<SourceReader>,
}

impl BookmarkManager {
    pub fn new(target_bookmarks: TargetBookmarks, source_readers: Vec<SourceReader>) -> Self {
        Self {
            target_bookmarks,
            source_readers,
        }
    }

    pub fn from_sources(sources: &[RawSource]) -> Result<Self, anyhow::Error> {
        Ok(Self {
            target_bookmarks: TargetBookmarks::default(),
            source_readers: sources
                .iter()
                .map(SourceReader::init)
                .collect::<Result<Vec<_>, anyhow::Error>>()?,
        })
    }

    pub fn target_bookmarks(&self) -> &TargetBookmarks {
        &self.target_bookmarks
    }

    pub fn target_bookmarks_mut(&mut self) -> &mut TargetBookmarks {
        &mut self.target_bookmarks
    }

    /// Import bookmarks from source and target files.
    pub fn import(
        &mut self,
        target_reader: &mut impl ReadTarget,
        now: DateTime<Utc>,
    ) -> Result<(), BogrepError> {
        target_reader.read(&mut self.target_bookmarks)?;

        if !self.source_readers.is_empty() {
            let mut source_bookmarks = SourceBookmarks::default();

            for source_reader in self.source_readers.iter_mut() {
                source_reader.import(&mut source_bookmarks)?;
            }

            self.add_bookmarks(&source_bookmarks, now)?;
            self.remove_bookmarks(&source_bookmarks);
        }

        Ok(())
    }

    /// Export bookmarks to target file.
    pub fn export(&mut self, target_writer: &mut impl WriteTarget) -> Result<(), BogrepError> {
        target_writer.write(&self.target_bookmarks)?;
        Ok(())
    }

    pub fn add_urls(
        &mut self,
        urls: &[Url],
        cache_mode: &CacheMode,
        action: &Action,
        now: DateTime<Utc>,
    ) {
        for url in urls {
            let target_bookmark = TargetBookmark::builder(url.clone(), now)
                .add_source(SourceType::Internal)
                .add_cache_mode(cache_mode.to_owned())
                .with_status(Status::Added)
                .with_action(action.to_owned())
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

    /// Print summary of the imported bookmarks.
    pub fn print_report(&self, run_mode: &RunMode) {
        let sources = self
            .source_readers
            .iter()
            .map(|source_reader| source_reader.source().clone())
            .collect::<Vec<_>>();
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
        let source_count = sources.len();
        let sources = sources
            .iter()
            .map(|source| source.path.to_string_lossy())
            .collect::<Vec<_>>()
            .join(", ");
        let source_str = if source_count == 1 {
            "source"
        } else {
            "sources"
        };
        let dry_run_str = match run_mode {
            RunMode::DryRun => " (dry run)",
            _ => "",
        };

        if !added_bookmarks.is_empty() {
            println!("Added {added_count} bookmarks{dry_run_str}");
        }

        if !removed_bookmarks.is_empty() {
            println!("Removed {removed_count} bookmarks{dry_run_str}");
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

    /// Add the difference between source and target bookmarks.
    fn add_bookmarks(
        &mut self,
        source_bookmarks: &SourceBookmarks,
        now: DateTime<Utc>,
    ) -> Result<(), BogrepError> {
        let bookmarks_to_add = Self::filter_to_add(source_bookmarks, &self.target_bookmarks);
        trace!(
            "Added bookmarks: {:#?}",
            bookmarks_to_add
                .iter()
                .map(|bookmark| bookmark.url().to_owned())
                .collect::<Vec<_>>()
        );

        for source_bookmark in bookmarks_to_add {
            let url = Url::parse(source_bookmark.url())?;
            let target_bookmark = TargetBookmarkBuilder::new(url, now)
                .with_sources(source_bookmark.sources().to_owned())
                .with_folders(source_bookmark.folders().to_owned())
                .build();
            self.target_bookmarks.upsert(target_bookmark);
        }

        Ok(())
    }

    /// Remove the difference between source and target bookmarks.
    fn remove_bookmarks(&mut self, source_bookmarks: &SourceBookmarks) {
        let bookmarks_to_remove =
            Self::filter_to_remove(source_bookmarks, &mut self.target_bookmarks);
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

    /// Filter the `SourceBookmarks` which should be added to the `TargetBookmarks`.
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

    /// Filter the `SourceBookmarks` which should be removed from the `TargetBookmarks`.
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
    use crate::{
        bookmarks::{RawSource, SourceBookmarkBuilder},
        json, JsonBookmarks, Settings, UnderlyingType,
    };
    use std::{
        collections::{HashMap, HashSet},
        io::{Cursor, Write},
        path::Path,
        str::FromStr,
    };

    fn create_target_reader(target_bookmarks: &TargetBookmarks) -> impl ReadTarget {
        let bookmarks_json = JsonBookmarks::from(target_bookmarks);
        let buf = json::serialize(bookmarks_json).unwrap();

        let mut target_reader: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        target_reader.write_all(&buf).unwrap();
        // Set cursor position to the start again to prepare cursor for reading.
        target_reader.set_position(0);
        target_reader
    }

    #[test]
    fn test_import_source() {
        let now = Utc::now();
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        let folders = vec![];
        let sources = vec![RawSource::new(source_path, folders)];
        let mut target_reader = create_target_reader(&TargetBookmarks::default());
        let mut bookmark_manager = BookmarkManager::from_sources(&sources).unwrap();

        let res = bookmark_manager.import(&mut target_reader, now);
        assert!(res.is_ok());

        assert!(bookmark_manager
            .target_bookmarks
            .contains_key(&Url::from_str("https://www.deepl.com/translator").unwrap()));
        assert!(bookmark_manager.target_bookmarks.contains_key(&Url::from_str("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/").unwrap()));
        assert!(bookmark_manager.target_bookmarks.contains_key(
            &Url::from_str("https://en.wikipedia.org/wiki/Design_Patterns").unwrap()
        ));
        assert!(bookmark_manager.target_bookmarks.contains_key(
            &Url::from_str("https://doc.rust-lang.org/book/title-page.html").unwrap()
        ));
    }

    #[test]
    fn test_import() {
        let now = Utc::now();
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let mut target_bookmarks = TargetBookmarks::default();
        target_bookmarks.insert(
            TargetBookmarkBuilder::new(url1.clone(), now)
                .add_source(SourceType::Internal)
                .build(),
        );
        target_bookmarks.insert(
            TargetBookmarkBuilder::new(url2.clone(), now)
                .add_source(SourceType::Internal)
                .build(),
        );
        let mut target_reader = create_target_reader(&target_bookmarks);

        let mut bookmark_manager = BookmarkManager::default();
        let res = bookmark_manager.import(&mut target_reader, now);
        assert!(res.is_ok());

        assert!(bookmark_manager.target_bookmarks.contains_key(&url1));
        assert!(bookmark_manager.target_bookmarks.contains_key(&url2));
    }

    #[test]
    fn test_export() {
        let now = Utc::now();
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let mut bookmark_manager = BookmarkManager::default();
        bookmark_manager.target_bookmarks.insert(
            TargetBookmarkBuilder::new(url1.clone(), now)
                .add_source(SourceType::Internal)
                .build(),
        );
        bookmark_manager.target_bookmarks.insert(
            TargetBookmarkBuilder::new(url2.clone(), now)
                .add_source(SourceType::Internal)
                .build(),
        );

        let mut target_writer = Cursor::new(Vec::new());
        let res = bookmark_manager.export(&mut target_writer);
        assert!(res.is_ok());

        let actual = target_writer.get_ref();
        let actual_bookmarks = json::deserialize::<JsonBookmarks>(actual);
        assert!(
            actual_bookmarks.is_ok(),
            "{}\n{}",
            actual_bookmarks.unwrap_err(),
            String::from_utf8(actual.to_owned()).unwrap()
        );

        let actual_bookmarks = actual_bookmarks.unwrap();
        assert!(actual_bookmarks
            .iter()
            .all(|bookmark| bookmark.last_cached.is_none()));
        assert_eq!(
            actual_bookmarks
                .iter()
                .map(|bookmark| bookmark.url.clone())
                .collect::<HashSet<_>>(),
            HashSet::from_iter([url1.to_string(), url2.to_string()]),
        );
    }

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
                    .add_source(SourceType::Simple)
                    .build(),
            ),
            (
                url3.to_string(),
                SourceBookmarkBuilder::new(url3.as_str())
                    .add_source(SourceType::Simple)
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
        let mut bookmark_manager = BookmarkManager::new(target_bookmarks, vec![]);

        bookmark_manager
            .add_bookmarks(&source_bookmarks, now)
            .unwrap();
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

        bookmark_manager.remove_bookmarks(&source_bookmarks);
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

    #[test]
    fn test_add_urls() {
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let now = Utc::now();
        let settings = Settings::default();
        let mut bookmark_manager = BookmarkManager::default();

        bookmark_manager.add_urls(
            &[url1.clone(), url2.clone()],
            &settings.cache_mode,
            &Action::None,
            now,
        );

        let bookmark = bookmark_manager.target_bookmarks().get(&url1).unwrap();
        assert_eq!(bookmark.url, url1);
        assert_eq!(bookmark.underlying_url, None);
        assert_eq!(bookmark.underlying_type, UnderlyingType::None);
        assert_eq!(bookmark.last_imported, now.timestamp_millis());
        assert_eq!(bookmark.last_cached, None);
        assert!(bookmark.sources.contains(&SourceType::Internal));
        assert!(bookmark.cache_modes.contains(&CacheMode::Text));
        assert_eq!(bookmark.status, Status::Added);
        assert_eq!(bookmark.action, Action::None);

        let bookmark = bookmark_manager.target_bookmarks().get(&url2).unwrap();
        assert_eq!(bookmark.url, url2);
        assert_eq!(bookmark.underlying_url, None);
        assert_eq!(bookmark.underlying_type, UnderlyingType::None);
        assert_eq!(bookmark.last_imported, now.timestamp_millis());
        assert_eq!(bookmark.last_cached, None);
        assert!(bookmark.sources.contains(&SourceType::Internal));
        assert!(bookmark.cache_modes.contains(&CacheMode::Text));
        assert_eq!(bookmark.status, Status::Added);
        assert_eq!(bookmark.action, Action::None);
    }

    #[test]
    fn test_add_urls_existing() {
        let now = Utc::now();
        let url = Url::parse("https://url1.com").unwrap();
        let target_bookmark = TargetBookmarkBuilder::new(url.clone(), now)
            .add_source(SourceType::Internal)
            .add_cache_mode(CacheMode::Text)
            .build();
        let settings = Settings::default();
        let mut bookmark_manager = BookmarkManager::default();
        bookmark_manager
            .target_bookmarks_mut()
            .insert(target_bookmark.clone());

        bookmark_manager.add_urls(&[url.clone()], &settings.cache_mode, &Action::None, now);

        let bookmark = bookmark_manager.target_bookmarks().get(&url).unwrap();
        assert_eq!(bookmark.id, target_bookmark.id);
        assert_eq!(bookmark.url, url);
        assert_eq!(bookmark.underlying_url, None);
        assert_eq!(bookmark.underlying_type, UnderlyingType::None);
        assert_eq!(bookmark.last_imported, now.timestamp_millis());
        assert_eq!(bookmark.last_cached, None);
        assert!(bookmark.sources.contains(&SourceType::Internal));
        assert!(bookmark.cache_modes.contains(&CacheMode::Text));
        assert_eq!(bookmark.status, Status::None);
        assert_eq!(bookmark.action, Action::None);
    }

    #[test]
    fn test_add_urls_empty() {
        let now = Utc::now();
        let settings = Settings::default();
        let mut bookmark_manager = BookmarkManager::default();
        assert!(bookmark_manager.target_bookmarks.is_empty());

        bookmark_manager.add_urls(&[], &settings.cache_mode, &Action::None, now);

        assert!(bookmark_manager.target_bookmarks.is_empty());
    }

    #[test]
    fn test_remove_urls() {
        let now = Utc::now();
        let url = Url::parse("https://url1.com").unwrap();
        let settings = Settings::default();

        let mut bookmark_manager = BookmarkManager::default();

        bookmark_manager.add_urls(&[url.clone()], &settings.cache_mode, &Action::None, now);
        assert_eq!(bookmark_manager.target_bookmarks.len(), 1);

        bookmark_manager.remove_urls(&[url.clone()]);

        let bookmark = bookmark_manager.target_bookmarks().get(&url).unwrap();
        assert_eq!(bookmark.url, url);
        assert_eq!(bookmark.underlying_url, None);
        assert_eq!(bookmark.underlying_type, UnderlyingType::None);
        assert_eq!(bookmark.last_imported, now.timestamp_millis());
        assert_eq!(bookmark.last_cached, None);
        assert!(bookmark.sources.contains(&SourceType::Internal));
        assert!(bookmark.cache_modes.contains(&CacheMode::Text));
        assert_eq!(bookmark.status, Status::Removed);
        assert_eq!(bookmark.action, Action::Remove);
    }
}
