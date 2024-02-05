use super::BookmarkReaders;
use crate::{bookmarks::RawSource, utils};
use anyhow::anyhow;
use log::debug;
use lz4::block;
use std::io::{BufRead, BufReader, Lines, Read, Seek};

trait SeekRead: Seek + Read {}
impl<T> SeekRead for T where T: Seek + Read {}

pub trait ReadSource {
    type ParsedValue<'a>;

    fn extension(&self) -> Option<&str>;

    fn read_and_parse<'a>(
        &self,
        reader: &'a mut dyn SeekRead,
    ) -> Result<Self::ParsedValue<'a>, anyhow::Error>;
}

pub struct TextReader;

impl ReadSource for TextReader {
    type ParsedValue<'a> = Lines<BufReader<&'a mut dyn SeekRead>>;

    fn extension(&self) -> Option<&str> {
        Some("txt")
    }

    fn read_and_parse<'a>(
        &self,
        reader: &'a mut dyn SeekRead,
    ) -> Result<Self::ParsedValue<'a>, anyhow::Error> {
        debug!("Read file with extension: {:?}", self.extension());

        let buf_reader = BufReader::new(reader);
        let lines = buf_reader.lines();
        Ok(lines)
    }
}

pub struct JsonReader;

impl ReadSource for JsonReader {
    type ParsedValue<'a> = serde_json::Value;

    fn extension(&self) -> Option<&str> {
        Some("json")
    }

    fn read_and_parse<'a>(
        &'a self,
        reader: &mut dyn SeekRead,
    ) -> Result<Self::ParsedValue<'a>, anyhow::Error> {
        debug!("Read file with extension: {:?}", self.extension());

        let mut raw_bookmarks = Vec::new();
        reader.read_to_end(&mut raw_bookmarks)?;

        let parsed_bookmarks = serde_json::from_slice(&raw_bookmarks)?;
        Ok(parsed_bookmarks)
    }
}

pub struct CompressedJsonReader;

impl ReadSource for CompressedJsonReader {
    type ParsedValue<'a> = serde_json::Value;

    fn extension(&self) -> Option<&str> {
        Some("jsonlz4")
    }

    fn read_and_parse<'a>(
        &'a self,
        reader: &mut dyn SeekRead,
    ) -> Result<Self::ParsedValue<'a>, anyhow::Error> {
        debug!("Read file with extension: {:?}", self.extension());

        let mut compressed_data = Vec::new();
        reader.read_to_end(&mut compressed_data)?;

        // Skip the first 8 bytes: "mozLz40\0" (non-standard header)
        let decompressed_data = block::decompress(&compressed_data[8..], None)?;

        let parsed_bookmarks = serde_json::from_slice(&decompressed_data)?;
        Ok(parsed_bookmarks)
    }
}

pub struct PlistReader;

impl ReadSource for PlistReader {
    type ParsedValue<'a> = plist::Value;

    fn extension(&self) -> Option<&str> {
        Some("plist")
    }

    fn read_and_parse<'a>(
        &'a self,
        reader: &mut dyn SeekRead,
    ) -> Result<Self::ParsedValue<'a>, anyhow::Error> {
        debug!("Read file with extension: {:?}", self.extension());

        let mut bookmarks = Vec::new();
        reader.read_to_end(&mut bookmarks).unwrap();

        let parsed_bookmarks = plist::Value::from_reader(reader).unwrap();
        Ok(parsed_bookmarks)
    }
}

/// A reader of source files to abstract the file system through the `Read`
/// trait.
pub struct SourceReader {
    source: RawSource,
    reader: Box<dyn SeekRead>,
}

impl SourceReader {
    pub fn new(source: RawSource, reader: Box<dyn SeekRead>) -> Self {
        Self { source, reader }
    }

    /// Select the source file if a source directory is given.
    pub fn init(source: &RawSource) -> Result<Self, anyhow::Error> {
        let bookmark_file = utils::open_file(&source.path)?;
        let bookmark_readers = BookmarkReaders::new();

        for bookmark_reader in bookmark_readers.0 {
            if source.path.is_dir() {
                if let Some(bookmarks_path) = bookmark_reader.select_file(&source.path)? {
                    if bookmarks_path.is_file()
                        && bookmark_reader.extension()
                            == bookmarks_path.extension().and_then(|path| path.to_str())
                    {
                        return Ok(Self {
                            // Overwrite raw source
                            source: RawSource::new(&source.path, source.folders.clone()),
                            reader: Box::new(bookmark_file),
                        });
                    }
                }
            } else if source.path.is_file() {
                return Ok(Self {
                    source: source.to_owned(),
                    reader: Box::new(bookmark_file),
                });
            }
        }

        Err(anyhow!(
            "Format not supported for bookmark file '{}'",
            source.path.display()
        ))
    }

    pub fn source(&self) -> &RawSource {
        &self.source
    }

    pub fn reader_mut(&mut self) -> &mut dyn Read {
        &mut self.reader
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;
    use std::path::Path;

    #[test]
    fn test_init_firefox() {
        let source_path = Path::new("test_data/bookmarks_firefox.json");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert!(source_reader.source().path.is_file());
        assert_eq!(source_reader.source().path, source_path);
    }

    #[test]
    fn test_init_firefox_compressed() {
        let source_path = Path::new("test_data/bookmarks_firefox.jsonlz4");
        test_utils::create_compressed_bookmarks(source_path);
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert!(source_reader.source().path.is_file());
        assert_eq!(source_reader.source().path, source_path);
    }

    #[test]
    fn test_init_chrome() {
        let source_path = Path::new("test_data/bookmarks_chromium.json");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert!(source_reader.source().path.is_file());
        assert_eq!(source_reader.source().path, source_path);
    }

    #[test]
    fn test_init_chrome_no_extension() {
        let source_path = Path::new("test_data/bookmarks_chromium_no_extension");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert!(source_reader.source().path.is_file());
        assert_eq!(source_reader.source().path, source_path);
    }

    #[test]
    fn test_init_simple() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&source).unwrap();
        assert!(source_reader.source().path.is_file());
        assert_eq!(source_reader.source().path, source_path);
    }
}
