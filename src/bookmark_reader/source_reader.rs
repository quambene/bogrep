use super::{
    chrome::ChromeSelector,
    chromium::{ChromiumSelector, JsonBookmarkReader},
    edge::EdgeSelector,
    firefox::FirefoxSelector,
    safari::PlistBookmarkReader,
    simple::TextBookmarkReader,
    BookmarkReader, ChromiumReader, FirefoxReader, ParsedBookmarks, ReadSource, SafariReader,
    SeekRead, SimpleReader, SourceSelector,
};
use crate::{bookmarks::RawSource, utils, Source, SourceBookmarks, SourceType};
use anyhow::anyhow;
use log::debug;
use lz4::block;
use std::{
    io::{BufRead, BufReader, Cursor},
    path::Path,
};

pub struct SourceSelectors([SourceSelector; 4]);

impl SourceSelectors {
    pub fn new() -> Self {
        Self([
            FirefoxSelector::new(),
            ChromiumSelector::new(),
            ChromeSelector::new(),
            EdgeSelector::new(),
        ])
    }
}

/// Reader for txt files.
pub struct TextReader;

impl ReadSource for TextReader {
    fn extension(&self) -> Option<&str> {
        Some("txt")
    }

    fn read_and_parse<'a>(
        &self,
        reader: &'a mut dyn SeekRead,
    ) -> Result<ParsedBookmarks<'a>, anyhow::Error> {
        debug!("Read file with extension: {:?}", self.extension());

        let buf_reader = BufReader::new(reader);
        let lines = buf_reader.lines();
        Ok(ParsedBookmarks::Text(lines))
    }
}

/// Reader for json files.
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

/// Reader for plist files in binary format.
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

/// A reader of source files to abstract the file system through the `Read`
/// trait.
pub struct SourceReader {
    source: Source,
    reader: Box<dyn SeekRead>,
    source_reader: Box<dyn ReadSource>,
}

impl SourceReader {
    pub fn new(
        source: Source,
        reader: Box<dyn SeekRead>,
        source_reader: Box<dyn ReadSource>,
    ) -> Self {
        Self {
            source,
            reader,
            source_reader,
        }
    }

    /// Select the source file if a source directory is given.
    pub fn init(raw_source: &RawSource) -> Result<Self, anyhow::Error> {
        let source_path = &raw_source.path;
        let source_folders = &raw_source.folders;

        if source_path.is_dir() {
            let source_selectors = SourceSelectors::new();

            for source_selector in source_selectors.0 {
                if let Some(bookmarks_path) = source_selector.find_file(source_path)? {
                    let source_extension =
                        bookmarks_path.extension().and_then(|path| path.to_str());

                    if bookmarks_path.is_file() && source_selector.extension() == source_extension {
                        let source = Source::new(
                            source_selector.name(),
                            &bookmarks_path,
                            source_folders.clone(),
                        );
                        let bookmark_file = utils::open_file(&bookmarks_path)?;
                        let reader = Box::new(bookmark_file);
                        let source_reader = Self::select(source_extension)?;
                        return Ok(Self::new(source, reader, source_reader));
                    }
                }
            }
        } else if source_path.is_file() {
            let source_extension = source_path.extension().and_then(|path| path.to_str());
            let source = Source::new(SourceType::Unknown, source_path, source_folders.clone());
            let bookmark_file = utils::open_file(&raw_source.path)?;
            let reader = Box::new(bookmark_file);
            let source_reader = Self::select(source_extension)?;
            return Ok(Self::new(source.clone(), reader, source_reader));
        }

        Err(anyhow!(
            "Format not supported for bookmark file '{}'",
            source_path.display()
        ))
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    pub fn import(&mut self, source_bookmarks: &mut SourceBookmarks) -> Result<(), anyhow::Error> {
        let raw_source = self.source().clone();
        let source_path = &raw_source.path;
        let source_folders = &raw_source.folders;
        let parsed_bookmarks = self.read_and_parse()?;

        match parsed_bookmarks {
            ParsedBookmarks::Text(parsed_bookmarks) => {
                let bookmark_readers: Vec<TextBookmarkReader> = vec![SimpleReader::new()];
                Self::import_by_source(
                    source_path,
                    source_folders,
                    source_bookmarks,
                    parsed_bookmarks,
                    bookmark_readers,
                )?;
            }
            ParsedBookmarks::Json(parsed_bookmarks) => {
                let bookmark_readers: Vec<JsonBookmarkReader> =
                    vec![FirefoxReader::new(), ChromiumReader::new()];
                Self::import_by_source(
                    source_path,
                    source_folders,
                    source_bookmarks,
                    parsed_bookmarks,
                    bookmark_readers,
                )?;
            }
            ParsedBookmarks::Plist(parsed_bookmarks) => {
                let bookmark_readers: Vec<PlistBookmarkReader> = vec![SafariReader::new()];
                Self::import_by_source(
                    source_path,
                    source_folders,
                    source_bookmarks,
                    parsed_bookmarks,
                    bookmark_readers,
                )?;
            }
            ParsedBookmarks::Html(_parsed_bookmarks) => {
                return Err(anyhow!("Bookmarks in HTML format not supported"));
            }
        }

        Ok(())
    }

    fn read_and_parse(&mut self) -> Result<ParsedBookmarks, anyhow::Error> {
        let parsed_bookmarks = self.source_reader.read_and_parse(&mut self.reader)?;
        Ok(parsed_bookmarks)
    }

    fn import_by_source<P>(
        source_path: &Path,
        source_folders: &[String],
        source_bookmarks: &mut SourceBookmarks,
        parsed_bookmarks: P,
        bookmark_readers: Vec<BookmarkReader<P>>,
    ) -> Result<(), anyhow::Error> {
        for bookmark_reader in bookmark_readers {
            if let Some(source_type) =
                bookmark_reader.select_source(source_path, &parsed_bookmarks)?
            {
                let source = Source::new(source_type, source_path, source_folders.to_vec());
                bookmark_reader.import(&source, parsed_bookmarks, source_bookmarks)?;
                break;
            }
        }

        Ok(())
    }

    fn select(source_extension: Option<&str>) -> Result<Box<dyn ReadSource>, anyhow::Error> {
        match source_extension {
            Some("txt") => Ok(Box::new(TextReader)),
            Some("json") => Ok(Box::new(JsonReader)),
            Some("jsonlz4") => Ok(Box::new(CompressedJsonReader)),
            Some("plist") => Ok(Box::new(PlistReader)),
            Some(others) => Err(anyhow!(format!("File type {others} not supported"))),
            // Chrome's bookmarks in json format are provided without file
            // extension.
            None => Ok(Box::new(JsonReaderNoExtension)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;
    use std::path::Path;

    #[test]
    fn test_init_safari_binary() {
        let source_path = Path::new("test_data/bookmarks_safari_binary.plist");
        test_utils::create_binary_plist_file(source_path).unwrap();
        let source_folders = vec![];
        let raw_source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&raw_source).unwrap();
        let source = source_reader.source();
        assert_eq!(source.source_type, SourceType::Unknown);
        assert!(source.path.is_file());
        assert_eq!(source.path, source_path);
        assert_eq!(source_reader.source_reader.extension(), Some("plist"));
    }

    #[test]
    fn test_init_safari_xml() {
        let source_path = Path::new("test_data/bookmarks_safari_xml.plist");
        let source_folders = vec![];
        let raw_source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&raw_source).unwrap();
        let source = source_reader.source();
        assert_eq!(source.source_type, SourceType::Unknown);
        assert!(source.path.is_file());
        assert_eq!(source.path, source_path);
        assert_eq!(source_reader.source_reader.extension(), Some("plist"));
    }

    #[test]
    fn test_init_firefox() {
        let source_path = Path::new("test_data/bookmarks_firefox.json");
        let source_folders = vec![];
        let raw_source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&raw_source).unwrap();
        let source = source_reader.source();
        assert_eq!(source.source_type, SourceType::Unknown);
        assert!(source.path.is_file());
        assert_eq!(source.path, source_path);
        assert_eq!(source_reader.source_reader.extension(), Some("json"));
    }

    #[test]
    fn test_init_firefox_compressed() {
        let source_path = Path::new("test_data/bookmarks_firefox.jsonlz4");
        test_utils::create_compressed_json_file(source_path).unwrap();
        let source_folders = vec![];
        let raw_source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&raw_source).unwrap();
        let source = source_reader.source();
        assert_eq!(source.source_type, SourceType::Unknown);
        assert!(source.path.is_file());
        assert_eq!(source.path, source_path);
        assert_eq!(source_reader.source_reader.extension(), Some("jsonlz4"));
    }

    #[test]
    fn test_init_chrome() {
        let source_path = Path::new("test_data/bookmarks_chromium.json");
        let source_folders = vec![];
        let raw_source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&raw_source).unwrap();
        let source = source_reader.source();
        assert_eq!(source.source_type, SourceType::Unknown);
        assert!(source.path.is_file());
        assert_eq!(source.path, source_path);
        assert_eq!(source_reader.source_reader.extension(), Some("json"));
    }

    #[test]
    fn test_init_chrome_no_extension() {
        let source_path = Path::new("test_data/bookmarks_chromium_no_extension");
        let source_folders = vec![];
        let raw_source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&raw_source).unwrap();
        let source = source_reader.source();
        assert_eq!(source.source_type, SourceType::Unknown);
        assert!(source.path.is_file());
        assert_eq!(source.path, source_path);
        assert_eq!(source_reader.source_reader.extension(), None);
    }

    #[test]
    fn test_init_simple() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        let source_folders = vec![];
        let raw_source = RawSource::new(source_path, source_folders);
        let source_reader = SourceReader::init(&raw_source).unwrap();
        let source = source_reader.source();
        assert_eq!(source.source_type, SourceType::Unknown);
        assert!(source.path.is_file());
        assert_eq!(source.path, source_path);
        assert_eq!(source_reader.source_reader.extension(), Some("txt"));
    }
}
