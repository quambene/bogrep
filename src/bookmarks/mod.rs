mod bookmark_manager;
mod bookmark_service;
mod source_bookmarks;
mod target_bookmarks;

use crate::CacheMode;
pub use bookmark_manager::BookmarkManager;
pub use bookmark_service::{BookmarkService, ServiceConfig};
use serde::{Deserialize, Serialize};
pub use source_bookmarks::{SourceBookmark, SourceBookmarkBuilder, SourceBookmarks};
use std::{
    cmp::Ordering,
    collections::HashSet,
    fmt,
    path::{Path, PathBuf},
    slice::Iter,
};
pub use target_bookmarks::{TargetBookmark, TargetBookmarkBuilder, TargetBookmarks};
use url::Url;
use uuid::Uuid;

pub const HACKER_NEWS_DOMAINS: &[&str] = &["news.ycombinator.com", "www.news.ycombinator.com"];
pub const REDDIT_DOMAINS: &[&str] = &["reddit.com", "www.reddit.com"];

/// The supported domains to fetch the underlying.
pub const SUPPORTED_UNDERLYING_DOMAINS: &[&str] = &[
    "news.ycombinator.com",
    "www.news.ycombinator.com",
    "reddit.com",
    "www.reddit.com",
];

/// The type used to identify a source.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceType {
    Firefox,
    ChromiumDerivative,
    Chromium,
    Chrome,
    Edge,
    Safari,
    Simple,
    Underlying(String),
    Internal,
    External,
    #[default]
    Unknown,
}

impl fmt::Display for SourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reader_name = match &self {
            SourceType::Firefox => "Firefox",
            SourceType::ChromiumDerivative => "Chromium (derivative)",
            SourceType::Chromium => "Chromium",
            SourceType::Chrome => "Chrome",
            SourceType::Edge => "Edge",
            SourceType::Safari => "Safari",
            SourceType::Simple => "Simple",
            SourceType::Underlying(_) => "Underlying",
            SourceType::Internal => "Internal",
            SourceType::External => "External",
            SourceType::Unknown => "Unknown",
        };
        write!(f, "{}", reader_name)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Status {
    /// Bookmark added to state.
    Added,
    /// Bookmark removed from state.
    Removed,
    None,
}

/// The action to be performed on the bookmark.
///
/// `Actions`s includes external resources, like cache and fetching bookmarks
/// from the web.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum Action {
    /// Fetch and cache the bookmark, even if it is cached already. The cached
    /// content will be replaced with the most recent version of the website.
    FetchAndReplace,
    /// Fetch and cache bookmark if it is not cached yet.
    FetchAndAdd,
    /// Fetch a bookmark and diff the fetched content with the cached content.#
    FetchAndDiff,
    /// Remove a bookmark from the cache.
    Remove,
    /// No actions to be performed.
    None,
    /// Skip fetching, caching, and writing to file.
    DryRun,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum UnderlyingType {
    HackerNews,
    Reddit,
    None,
}

impl From<&Url> for UnderlyingType {
    fn from(url: &Url) -> Self {
        if url
            .domain()
            .is_some_and(|domain| HACKER_NEWS_DOMAINS.contains(&domain))
        {
            UnderlyingType::HackerNews
        } else if url
            .domain()
            .is_some_and(|domain| REDDIT_DOMAINS.contains(&domain))
        {
            UnderlyingType::Reddit
        } else {
            UnderlyingType::None
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub enum RunMode {
    /// Import bookmarks, but don't fetch them.
    Import,
    /// Add provided bookmark urls.
    AddUrls(Vec<Url>),
    /// Remove provided bookmark urls.
    RemoveUrls(Vec<Url>),
    /// Import and fetch provided bookmark urls.
    FetchUrls(Vec<Url>),
    /// Fetch bookmarks which were not fetched yet.
    Fetch,
    /// Fetch and replace all bookmarks.
    FetchAll,
    /// Fetch diff for provided bookmark urls.
    FetchDiff(Vec<Url>),
    /// Sync bookmarks.
    Sync,
    /// Run in dry mode.
    DryRun,
    #[default]
    None,
}

/// The source of bookmarks.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct RawSource {
    /// The path to the source file or directory.
    #[serde(rename = "source")]
    pub path: PathBuf,
    /// The folders to be imported.
    ///
    /// If no folders are selected, all bookmarks in the source file will be
    /// imported.
    pub folders: Vec<String>,
}

impl RawSource {
    pub fn new(path: impl Into<PathBuf>, folders: Vec<String>) -> Self {
        Self {
            path: path.into(),
            folders,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Source {
    /// The name of the source.
    pub source_type: SourceType,
    /// The path of the source file used for logging.
    pub path: PathBuf,
    /// The folders to be imported.
    ///
    /// If no folders are selected, all bookmarks in the source file will be
    /// imported.
    pub folders: Vec<String>,
}

impl Source {
    pub fn new(source_type: SourceType, path: &Path, folders: Vec<String>) -> Self {
        Self {
            source_type,
            path: path.to_owned(),
            folders,
        }
    }
}

/// The source folder of a bookmark.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceFolder {
    source: SourceType,
    name: String,
}

impl SourceFolder {
    pub fn new(source: SourceType, name: String) -> Self {
        Self { source, name }
    }

    pub fn source(&self) -> &SourceType {
        &self.source
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Serialize, PartialEq, Deserialize)]
pub struct JsonBookmark {
    pub id: String,
    pub url: String,
    pub last_imported: i64,
    pub last_cached: Option<i64>,
    pub sources: HashSet<SourceType>,
    pub cache_modes: HashSet<CacheMode>,
}

impl JsonBookmark {
    pub fn new(
        url: String,
        last_imported: i64,
        last_cached: Option<i64>,
        sources: HashSet<SourceType>,
        cache_modes: HashSet<CacheMode>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            url,
            last_imported,
            last_cached,
            sources,
            cache_modes,
        }
    }
}

impl From<TargetBookmark> for JsonBookmark {
    fn from(value: TargetBookmark) -> Self {
        Self {
            id: value.id().to_owned(),
            url: value.url().to_string(),
            last_imported: value.last_imported(),
            last_cached: value.last_cached(),
            sources: value.sources().to_owned(),
            cache_modes: value.cache_modes().to_owned(),
        }
    }
}

impl From<&TargetBookmark> for JsonBookmark {
    fn from(value: &TargetBookmark) -> Self {
        Self {
            id: value.id().to_owned(),
            url: value.url().to_string(),
            last_imported: value.last_imported(),
            last_cached: value.last_cached(),
            sources: value.sources().clone(),
            cache_modes: value.cache_modes().clone(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct JsonBookmarks {
    pub bookmarks: Vec<JsonBookmark>,
}

impl JsonBookmarks {
    pub fn new(bookmarks: Vec<TargetBookmark>) -> Self {
        let mut bookmarks = bookmarks
            .into_iter()
            .map(JsonBookmark::from)
            .collect::<Vec<_>>();
        bookmarks.sort_by(Self::compare);

        Self { bookmarks }
    }

    pub fn iter(&self) -> Iter<JsonBookmark> {
        self.bookmarks.iter()
    }

    pub fn get(&self, index: usize) -> Option<&JsonBookmark> {
        self.bookmarks.get(index)
    }

    pub fn is_empty(&self) -> bool {
        self.bookmarks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.bookmarks.len()
    }

    // Sort by `last_cached` and then by `url`.
    fn compare(a: &JsonBookmark, b: &JsonBookmark) -> Ordering {
        match (a.last_cached, b.last_cached) {
            (Some(a_cached), Some(b_cached)) => {
                a_cached.cmp(&b_cached).then_with(|| a.url.cmp(&b.url))
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => a.url.cmp(&b.url),
        }
    }
}

impl From<&TargetBookmarks> for JsonBookmarks {
    fn from(target_bookmarks: &TargetBookmarks) -> Self {
        let mut bookmarks = target_bookmarks
            .values()
            .filter(|bookmark| {
                if bookmark.action() == &Action::DryRun {
                    bookmark.status() == &Status::None
                } else {
                    true
                }
            })
            .map(JsonBookmark::from)
            .collect::<Vec<_>>();
        bookmarks.sort_by(Self::compare);
        JsonBookmarks { bookmarks }
    }
}

impl IntoIterator for JsonBookmarks {
    type Item = JsonBookmark;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.bookmarks.into_iter()
    }
}

pub struct JsonBookmarksIterator<'a> {
    bookmarks_iter: Iter<'a, JsonBookmark>,
}

impl<'a> IntoIterator for &'a JsonBookmarks {
    type Item = &'a JsonBookmark;
    type IntoIter = JsonBookmarksIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        JsonBookmarksIterator {
            bookmarks_iter: self.bookmarks.iter(),
        }
    }
}

impl<'a> Iterator for JsonBookmarksIterator<'a> {
    type Item = &'a JsonBookmark;

    fn next(&mut self) -> Option<Self::Item> {
        self.bookmarks_iter.next()
    }
}

#[derive(Debug, Default)]
pub struct ServiceReport {
    total: usize,
    processed: i32,
    cached: i32,
    failed_response: i32,
    binary_response: i32,
    empty_response: i32,
    dry_run: bool,
}

impl ServiceReport {
    pub fn new(
        total: usize,
        processed: i32,
        cached: i32,
        failed_response: i32,
        binary_response: i32,
        empty_response: i32,
        dry_run: bool,
    ) -> Self {
        Self {
            total,
            processed,
            cached,
            failed_response,
            binary_response,
            empty_response,
            dry_run,
        }
    }

    pub fn init(dry_run: bool) -> Self {
        Self::new(0, 0, 0, 0, 0, 0, dry_run)
    }

    pub fn print(&self) {
        print!("Processing bookmarks ({}/{})\r", self.processed, self.total);
    }

    pub fn print_summary(&self) {
        if self.total == 0 {
            println!("Processing bookmarks (0/0)");
        } else {
            println!();
        }

        if self.dry_run {
            println!(
                "Processed {} bookmarks, {} cached, {} ignored, {} failed (dry run)",
                self.total, 0, 0, 0
            );
        } else {
            println!(
                "Processed {} bookmarks, {} cached, {} ignored, {} failed",
                self.total,
                self.cached,
                self.failed_response,
                self.binary_response + self.empty_response
            );
        }
    }

    pub fn reset(&mut self) {
        self.total = 0;
        self.processed = 0;
        self.cached = 0;
        self.failed_response = 0;
        self.binary_response = 0;
        self.empty_response = 0;
    }

    pub fn set_total(&mut self, total: usize) {
        self.total = total;
    }

    pub fn increment_processed(&mut self) {
        self.processed += 1;
    }

    pub fn increment_cached(&mut self) {
        self.cached += 1;
    }

    pub fn increment_failed_response(&mut self) {
        self.failed_response += 1;
    }

    pub fn increment_binary_response(&mut self) {
        self.binary_response += 1;
    }

    pub fn increment_empty_response(&mut self) {
        self.empty_response += 1;
    }
}
