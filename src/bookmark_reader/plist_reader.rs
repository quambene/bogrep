use super::{ReadSource, SeekRead};
use crate::{bookmark_reader::ParsedBookmarks, Source, SourceBookmarks};
use anyhow::anyhow;
use log::debug;
use plist::{Dictionary, Value};
use std::io::Cursor;

/// Reader for plist files in binary format.
#[derive(Debug)]
pub struct PlistReader;

impl ReadSource for PlistReader {
    fn extension(&self) -> Option<&str> {
        Some("plist")
    }

    fn read_and_parse<'a>(
        &self,
        reader: &'a mut dyn SeekRead,
    ) -> Result<ParsedBookmarks<'a>, anyhow::Error> {
        debug!("Read file with extension: {:?}", self.extension());

        let mut raw_bookmarks = Vec::new();
        reader.read_to_end(&mut raw_bookmarks)?;

        let file_type = infer::get(&raw_bookmarks);
        let mime_type = file_type.map(|file_type| file_type.mime_type());

        debug!("Parse file with mime type {:?}", mime_type);

        let parsed_bookmarks = match mime_type {
            Some("text/xml") | Some("application/xml") => {
                let cursor = Cursor::new(raw_bookmarks);
                plist::Value::from_reader_xml(cursor)?
            }
            Some("application/octet-stream") | None => plist::from_bytes(&raw_bookmarks)?,
            Some(other) => return Err(anyhow!("File type {other} not supported")),
        };

        Ok(ParsedBookmarks::Plist(parsed_bookmarks))
    }
}

pub fn traverse_plist(
    value: &Value,
    source: &Source,
    bookmarks: &mut SourceBookmarks,
    select_bookmark: fn(&Dictionary, &Source, &mut SourceBookmarks, &mut Option<String>),
    select_folder: fn(&Dictionary) -> Option<&String>,
) {
    let mut parent_folder = None;

    match value {
        Value::Dictionary(obj) => {
            if source.folders.is_empty() {
                select_bookmark(obj, source, bookmarks, &mut parent_folder);

                for (_, val) in obj {
                    traverse_plist(val, source, bookmarks, select_bookmark, select_folder);
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
                    traverse_plist(val, source, bookmarks, select_bookmark, select_folder);
                }
            }
        }
        Value::Array(arr) => {
            for val in arr {
                traverse_plist(val, source, bookmarks, select_bookmark, select_folder);
            }
        }
        Value::Boolean(_) => (),
        Value::Data(_) => (),
        Value::Date(_) => (),
        Value::Real(_) => (),
        Value::Integer(_) => (),
        Value::String(_) => (),
        Value::Uid(_) => (),
        _ => (),
    }
}

fn traverse_children(
    value: &Value,
    source: &Source,
    bookmarks: &mut SourceBookmarks,
    parent_folder: &mut Option<String>,
    select_bookmark: fn(&Dictionary, &Source, &mut SourceBookmarks, &mut Option<String>),
    select_folder: fn(&Dictionary) -> Option<&String>,
) {
    let mut folder = None;

    match value {
        Value::Dictionary(obj) => {
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
        Value::Boolean(_) => (),
        Value::Data(_) => (),
        Value::Date(_) => (),
        Value::Real(_) => (),
        Value::Integer(_) => (),
        Value::String(_) => (),
        Value::Uid(_) => (),
        _ => (),
    }
}
