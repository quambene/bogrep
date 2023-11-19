mod source_bookmarks;
mod target_bookmarks;

use serde::{Deserialize, Serialize};
pub use source_bookmarks::{SourceBookmark, SourceBookmarks};
use std::path::{Path, PathBuf};
pub use target_bookmarks::{TargetBookmark, TargetBookmarks};

/// The type used to identify a source.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceType {
    Firefox,
    Chromium,
    Chrome,
    Edge,
    Simple,
    Internal,
    External,
    #[default]
    Others,
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
    pub name: SourceType,
    /// The path of the source file used for displaying.
    pub path: String,
    /// The folders to be imported.
    ///
    /// If no folders are selected, all bookmarks in the source file will be
    /// imported.
    pub folders: Vec<String>,
}

impl Source {
    pub fn new(name: SourceType, path: &Path, folders: Vec<String>) -> Self {
        Self {
            name,
            path: path.to_string_lossy().to_string(),
            folders,
        }
    }
}
