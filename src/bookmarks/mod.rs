mod source_bookmarks;
mod target_bookmarks;

use serde::{Deserialize, Serialize};
pub use source_bookmarks::SourceBookmarks;
use std::path::PathBuf;
pub use target_bookmarks::{TargetBookmark, TargetBookmarks};

pub enum SourceType {
    Firefox,
    Chromium,
    Chrome,
    Edge,
    Simple,
    Internal,
    Others,
}

/// The source of bookmarks.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Source {
    /// The path to the source file.
    #[serde(rename = "source")]
    pub path: PathBuf,
    /// The folders to be imported.
    ///
    /// If no folders are selected, all bookmarks in the source file will be
    /// imported.
    pub folders: Vec<String>,
}

impl Source {
    pub fn new(path: impl Into<PathBuf>, folders: Vec<String>) -> Self {
        Self {
            path: path.into(),
            folders,
        }
    }
}
