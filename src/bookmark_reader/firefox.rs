use super::ReadBookmark;
use crate::{bookmark_reader::firefox_compressed::Firefox, Source, SourceBookmarks};
use anyhow::anyhow;
use log::debug;
use serde_json::Value;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Copy, Clone)]
pub struct FirefoxBookmarkReader;

impl ReadBookmark for FirefoxBookmarkReader {
    fn name(&self) -> &str {
        "Firefox"
    }

    fn extension(&self) -> Option<&str> {
        Some("json")
    }

    fn select(
        &self,
        reader: &mut dyn Read,
    ) -> Result<Option<Box<dyn ReadBookmark>>, anyhow::Error> {
        let raw_bookmarks = self.read(reader)?;
        let value: Value = serde_json::from_str(&raw_bookmarks)?;
        match value {
            Value::Object(obj) => {
                if let Some(Value::String(type_value)) = obj.get("type") {
                    if type_value == "text/x-moz-place-container" {
                        Ok(Some(Box::new(*self)))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn select_path(&self, source_path: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        let path_str = source_path
            .to_str()
            .ok_or(anyhow!("Invalid path: source path contains invalid UTF-8"))?;

        // On Linux, the path contains lowercase identifier; on macOS uppercase
        // identifier is required.
        if path_str.contains("firefox") || path_str.contains("Firefox") {
            // The Firefox bookmarks directory contains multiple bookmark file.
            // Check if a specific file or a directory of files is given.
            let bookmark_path = Firefox::find_most_recent_file(&source_path)?;
            Ok(Some(bookmark_path))
        } else {
            Err(anyhow!(
                "Unexpected format for source file: {}",
                source_path.display()
            ))
        }
    }

    fn open(&self, source_path: &Path) -> Result<File, anyhow::Error> {
        let bookmark_file = File::open(source_path)?;
        Ok(bookmark_file)
    }

    fn read(&self, reader: &mut dyn Read) -> Result<String, anyhow::Error> {
        debug!("Read bookmarks from {}", self.name());

        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;

        Ok(String::from_utf8(buf)?)
    }

    fn parse(
        &self,
        raw_bookmarks: &str,
        source: &Source,
        bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        debug!("Parse bookmarks from {}", self.name());
        let value: Value = serde_json::from_str(raw_bookmarks)?;
        Firefox::traverse_json(&value, bookmarks, source);
        Ok(())
    }
}
