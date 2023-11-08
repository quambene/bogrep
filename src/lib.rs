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
/// Helper functions to work with HTML.
pub mod html;
/// Helper function to work with JSON.
pub mod json;
/// The settings used in Bogrep.
mod settings;
/// Utilities used in testing.
pub mod test_utils;
/// Utilities to work with files (create, open, read, write).
pub mod utils;

pub use args::{Args, ConfigArgs, FetchArgs, IgnoreArgs, InitArgs, Subcommands};
pub use bookmark_reader::{
    ChromeBookmarkReader, FirefoxBookmarkReader, ReadBookmark, SimpleBookmarkReader,
};
pub use bookmarks::{SourceBookmarks, TargetBookmark, TargetBookmarks};
pub use cache::{Cache, Caching, MockCache};
pub use client::{Client, Fetch, MockClient};
pub use config::Config;
pub use settings::{Settings, Source};
