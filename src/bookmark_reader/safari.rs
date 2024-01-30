use super::ReaderName;
use crate::{ReadBookmark, Source, SourceBookmarks, SourceType};
use log::debug;
use serde_json::{Map, Value};
use std::{io::Read, path::Path};

pub struct Safari;

impl Safari {
    fn select_bookmark(
        obj: &Map<String, Value>,
        source_bookmarks: &mut SourceBookmarks,
        source: &Source,
    ) {
        todo!()
    }

    fn traverse_json(value: &Value, source_bookmarks: &mut SourceBookmarks, source: &Source) {
        match value {
            Value::Object(obj) => {
                todo!()
            }
            Value::Array(arr) => {
                for val in arr {
                    Self::traverse_json(val, source_bookmarks, source);
                }
            }
            Value::String(_) => (),
            Value::Number(_) => (),
            Value::Bool(_) => (),
            Value::Null => (),
        }
    }

    fn traverse_children(value: &Value, source_bookmarks: &mut SourceBookmarks, source: &Source) {
        match value {
            Value::Object(obj) => {
                Self::select_bookmark(obj, source_bookmarks, source);

                for (_, val) in obj {
                    Self::traverse_children(val, source_bookmarks, source);
                }
            }
            Value::Array(arr) => {
                for val in arr {
                    Self::traverse_children(val, source_bookmarks, source);
                }
            }
            Value::String(_) => (),
            Value::Number(_) => (),
            Value::Bool(_) => (),
            Value::Null => (),
        }
    }
}

/// A bookmark reader to read bookmarks in plist format from Safari.
#[derive(Debug)]
pub struct SafariBookmarkReader;

impl ReadBookmark for SafariBookmarkReader {
    fn name(&self) -> ReaderName {
        ReaderName::Safari
    }

    fn extension(&self) -> Option<&str> {
        Some("plist")
    }

    fn select_source(&self, source_path: &Path) -> Result<Option<SourceType>, anyhow::Error> {
        todo!()
    }

    fn read(&self, reader: &mut dyn Read) -> Result<String, anyhow::Error> {
        debug!("Read bookmarks from {}", self.name());
        let mut bookmarks = Vec::new();
        reader.read_to_end(&mut bookmarks)?;
        Ok(String::from_utf8(bookmarks)?)
    }

    fn parse(
        &self,
        raw_bookmarks: &str,
        source: &Source,
        bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        debug!("Parse bookmarks from {}", self.name());
        let value: Value = serde_json::from_str(raw_bookmarks)?;
        Safari::traverse_json(&value, bookmarks, source);
        Ok(())
    }
}
