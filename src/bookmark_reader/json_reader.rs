use super::{ParsedBookmarks, ReadSource, SeekRead};
use crate::{Source, SourceBookmarks};
use anyhow::anyhow;
use log::debug;
use lz4::block;
use serde_json::{Map, Value};

/// Reader for json files.
#[derive(Debug)]
pub struct JsonReader;

impl ReadSource for JsonReader {
    fn extension(&self) -> Option<&str> {
        Some("json")
    }

    fn read_and_parse<'a>(
        &self,
        reader: &'a mut dyn SeekRead,
    ) -> Result<ParsedBookmarks<'a>, anyhow::Error> {
        debug!("Read file with extension: {:?}", self.extension());

        let mut raw_bookmarks = Vec::new();
        reader.read_to_end(&mut raw_bookmarks)?;

        let parsed_bookmarks = serde_json::from_slice(&raw_bookmarks)?;
        Ok(ParsedBookmarks::Json(parsed_bookmarks))
    }
}

/// Reader for json files.
#[derive(Debug)]
pub struct JsonReaderNoExtension;

impl ReadSource for JsonReaderNoExtension {
    fn extension(&self) -> Option<&str> {
        None
    }

    fn read_and_parse<'a>(
        &self,
        reader: &'a mut dyn SeekRead,
    ) -> Result<ParsedBookmarks<'a>, anyhow::Error> {
        let mut raw_bookmarks = Vec::new();
        reader.read_to_end(&mut raw_bookmarks)?;

        let file_type = infer::get(&raw_bookmarks);
        let mime_type = file_type.map(|file_type| file_type.mime_type());

        debug!("Parse file with mime type {:?}", mime_type);

        let parsed_bookmarks = match mime_type {
            Some("text/plain") | Some("application/json") => {
                serde_json::from_slice(&raw_bookmarks)?
            }
            Some(other) => return Err(anyhow!("File type {other} not supported")),
            None => {
                if let Ok(parsed_bookmarks) = serde_json::from_slice(&raw_bookmarks) {
                    parsed_bookmarks
                } else {
                    return Err(anyhow!("File type not supported: {file_type:?}"));
                }
            }
        };

        Ok(ParsedBookmarks::Json(parsed_bookmarks))
    }
}

/// Reader for compressed json files with a Firefox-specific, non-standard header.
#[derive(Debug)]
pub struct CompressedJsonReader;

impl ReadSource for CompressedJsonReader {
    fn extension(&self) -> Option<&str> {
        Some("jsonlz4")
    }

    fn read_and_parse<'a>(
        &self,
        reader: &'a mut dyn SeekRead,
    ) -> Result<ParsedBookmarks<'a>, anyhow::Error> {
        debug!("Read file with extension: {:?}", self.extension());

        let mut compressed_data = Vec::new();
        reader.read_to_end(&mut compressed_data)?;

        // Skip the first 8 bytes: "mozLz40\0" (non-standard header)
        let decompressed_data = block::decompress(&compressed_data[8..], None)?;

        let parsed_bookmarks = serde_json::from_slice(&decompressed_data)?;
        Ok(ParsedBookmarks::Json(parsed_bookmarks))
    }
}

/// Traverse json and import bookmarks.
pub fn traverse_json(
    value: &Value,
    source: &Source,
    bookmarks: &mut SourceBookmarks,
    select_bookmark: fn(&Map<String, Value>, &Source, &mut SourceBookmarks, &mut Option<String>),
    select_folder: fn(&Map<String, Value>) -> Option<&String>,
) {
    let mut parent_folder = None;

    match value {
        Value::Object(obj) => {
            if source.folders.is_empty() {
                select_bookmark(obj, source, bookmarks, &mut parent_folder);

                for (_, val) in obj {
                    traverse_json(val, source, bookmarks, select_bookmark, select_folder);
                }
            } else {
                if let Some(selected_folder) = select_folder(obj) {
                    parent_folder = Some(selected_folder.to_owned());

                    if source.folders.contains(selected_folder) {
                        for (_, val) in obj {
                            traverse_children(
                                val,
                                source,
                                bookmarks,
                                &mut parent_folder,
                                select_bookmark,
                                select_folder,
                            );
                        }
                    }
                }

                for (_, val) in obj {
                    traverse_json(val, source, bookmarks, select_bookmark, select_folder);
                }
            }
        }
        Value::Array(arr) => {
            for val in arr {
                traverse_json(val, source, bookmarks, select_bookmark, select_folder);
            }
        }
        Value::String(_) => (),
        Value::Number(_) => (),
        Value::Bool(_) => (),
        Value::Null => (),
    }
}

fn traverse_children(
    value: &Value,
    source: &Source,
    bookmarks: &mut SourceBookmarks,
    parent_folder: &mut Option<String>,
    select_bookmark: fn(&Map<String, Value>, &Source, &mut SourceBookmarks, &mut Option<String>),
    select_folder: fn(&Map<String, Value>) -> Option<&String>,
) {
    let mut folder = None;

    match value {
        Value::Object(obj) => {
            select_bookmark(obj, source, bookmarks, parent_folder);

            if let Some(selected_folder) = select_folder(obj) {
                folder = Some(selected_folder.to_owned());
            }

            for (_, val) in obj {
                traverse_children(
                    val,
                    source,
                    bookmarks,
                    &mut folder,
                    select_bookmark,
                    select_folder,
                );
            }
        }
        Value::Array(arr) => {
            for val in arr {
                traverse_children(
                    val,
                    source,
                    bookmarks,
                    parent_folder,
                    select_bookmark,
                    select_folder,
                );
            }
        }
        Value::String(_) => (),
        Value::Number(_) => (),
        Value::Bool(_) => (),
        Value::Null => (),
    }
}
