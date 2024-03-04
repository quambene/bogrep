//! Bogrep downloads and caches your bookmarks in plaintext without images or
//! videos. Use the Bogrep CLI to grep through your cached bookmarks in
//! full-text search.
//!
//! ## Examples
//!
//! ``` bash
//! # Configure the path to the bookmarks file (e.g. of your browser)
//! bogrep config --source "my/path/to/bookmarks_file.json"
//!
//! # Import bookmarks
//! bogrep import
//!
//! # Fetch and cache bookmarks
//! bogrep fetch
//!
//! # Search your bookmarks in full-text search
//! bogrep <pattern>
//! ````

/// Available arguments.
mod args;
/// Abstraction and implementations to read bookmarks.
mod bookmark_reader;
/// The source and target bookmarks.
mod bookmarks;
/// The cache to store websites.
mod cache;
/// The client for fetching websites.
mod client;
/// Available commands.
pub mod cmd;
/// The configuration used in Bogrep.
mod config;
/// The errors which can occur in Bogrep.
pub mod errors;
/// Helper functions to work with HTML.
pub mod html;
/// Helper function to work with JSON.
pub mod json;
/// Initialize a simple logger based on the verbosity level (or the `RUST_LOG`
/// environment variable).
mod logger;
/// The settings used in Bogrep.
mod settings;
/// Utilities used in testing.
pub mod test_utils;
/// Utilities to work with files (create, open, read, write).
pub mod utils;

pub use args::{Args, ConfigArgs, FetchArgs, InitArgs, Subcommands};
pub use bookmark_reader::{
    ChromiumReader, FirefoxReader, ReadBookmark, SafariReader, SimpleReader,
};
pub use bookmarks::{
    Action, BookmarkProcessor, JsonBookmark, JsonBookmarks, ProcessReport, Source, SourceBookmark,
    SourceBookmarks, SourceType, Status, TargetBookmark, TargetBookmarkBuilder, TargetBookmarks,
    UnderlyingType,
};
pub use cache::{Cache, CacheMode, Caching, MockCache};
pub use client::{Client, Fetch, MockClient};
pub use config::Config;
pub use logger::Logger;
pub use settings::Settings;
