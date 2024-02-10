use super::{
    chromium::JsonBookmarkReader, firefox::FirefoxSelector, safari::PlistBookmarkReader,
    simple::TextBookmarkReader, BookmarkReader, ChromiumReader, FirefoxReader, ParsedBookmarks,
    ReadSource, SafariReader, SeekRead, SimpleReader, SourceSelector,
};
use crate::{bookmarks::RawSource, utils, Source, SourceBookmarks, SourceType};
use anyhow::anyhow;
use log::debug;
use lz4::block;
use std::{
    io::{BufRead, BufReader},
    path::Path,
};

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

        let mut bookmarks = Vec::new();
        reader.read_to_end(&mut bookmarks).unwrap();

        let parsed_bookmarks = plist::Value::from_reader(reader).unwrap();
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
    pub fn init(source: &RawSource) -> Result<Self, anyhow::Error> {
        let bookmark_file = utils::open_file(&source.path)?;
        let source_path = &source.path;
        let source_extension = source_path.extension().and_then(|path| path.to_str());

        if source.path.is_dir() {
            let source_selectors: Vec<SourceSelector> = vec![FirefoxSelector::new()];

            for source_selector in source_selectors {
                if let Some(bookmarks_path) = source_selector.find_file(&source.path)? {
                    if bookmarks_path.is_file() && source_selector.extension() == source_extension {
                        let source = Source::new(
                            source_selector.name(),
                            &source.path,
                            source.folders.clone(),
                        );
                        let reader = Box::new(bookmark_file);
                        let source_reader = Self::select(source_extension)?;
                        return Ok(Self::new(source, reader, source_reader));
                    }
                }
            }
        } else if source.path.is_file() {
            let source = Source::new(SourceType::Unknown, &source.path, source.folders.clone());
            let reader = Box::new(bookmark_file);
            let source_reader = Self::select(source_extension)?;
            return Ok(Self::new(source.clone(), reader, source_reader));
        }

        Err(anyhow!(
            "Format not supported for bookmark file '{}'",
            source.path.display()
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
                let source = Source::new(source_type, &source_path, source_folders.to_vec());
                bookmark_reader.import(&source, parsed_bookmarks, source_bookmarks)?;
                break;
            }
        }

        Ok(())
    }

    fn read_and_parse(&mut self) -> Result<ParsedBookmarks, anyhow::Error> {
        let parsed_bookmarks = self.source_reader.read_and_parse(&mut self.reader)?;
        Ok(parsed_bookmarks)
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
            None => Ok(Box::new(JsonReader)),
        }
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
