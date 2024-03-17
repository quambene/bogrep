use super::{
    chrome::ChromeSelector,
    chromium::{ChromiumSelector, JsonBookmarkReader},
    edge::EdgeSelector,
    firefox::FirefoxSelector,
    safari::{PlistBookmarkReader, SafariSelector},
    simple::TextBookmarkReader,
    BookmarkReader, ChromiumReader, CompressedJsonReader, FirefoxReader, JsonReader,
    JsonReaderNoExtension, ParsedBookmarks, PlistReader, ReadSource, SafariReader, SeekRead,
    SimpleReader, SourceOs, SourceSelector, TextReader,
};
use crate::{bookmarks::RawSource, utils, Source, SourceBookmarks, SourceType};
use anyhow::anyhow;
use log::debug;
use std::path::Path;

pub struct SourceSelectors([SourceSelector; 5]);

impl SourceSelectors {
    pub fn new() -> Self {
        Self([
            FirefoxSelector::new(),
            ChromiumSelector::new(),
            ChromeSelector::new(),
            EdgeSelector::new(),
            SafariSelector::new(),
        ])
    }
}

/// A reader of source files to abstract the file system through the `Read`
/// trait.
#[derive(Debug)]
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

    pub fn select_sources(
        home_dir: &Path,
        source_os: &SourceOs,
    ) -> Result<Vec<RawSource>, anyhow::Error> {
        let mut source_dirs = vec![];
        let source_selectors = SourceSelectors::new();

        for source_selector in source_selectors.0 {
            let source_dirs_by_selector = source_selector.find_sources(home_dir, source_os)?;
            source_dirs.extend(source_dirs_by_selector);
        }

        let raw_sources = source_dirs
            .into_iter()
            .map(|source_dir| RawSource::new(source_dir, vec![]))
            .collect();

        Ok(raw_sources)
    }

    /// Select the source file if a source directory is given.
    pub fn init(raw_source: &RawSource) -> Result<Self, anyhow::Error> {
        debug!("Init source: {raw_source:?}");
        let source_path = &raw_source.path;
        let folders = &raw_source.folders;

        if source_path.is_dir() {
            let source_selectors = SourceSelectors::new();

            for source_selector in source_selectors.0 {
                if let Some(bookmarks_path) = source_selector.find_source_file(source_path)? {
                    let source_extension =
                        bookmarks_path.extension().and_then(|path| path.to_str());

                    if bookmarks_path.is_file() && source_selector.extension() == source_extension {
                        let source =
                            Source::new(source_selector.name(), &bookmarks_path, folders.clone());
                        let bookmark_file = utils::open_file(&bookmarks_path)?;
                        let reader = Box::new(bookmark_file);
                        let source_reader = Self::select(source_extension)?;
                        return Ok(Self::new(source, reader, source_reader));
                    }
                }
            }
        } else if source_path.is_file() {
            let source_extension = source_path.extension().and_then(|path| path.to_str());
            let source = Source::new(SourceType::Unknown, source_path, folders.clone());
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
        let folders = &raw_source.folders;
        let parsed_bookmarks = self.read_and_parse()?;

        match parsed_bookmarks {
            ParsedBookmarks::Text(parsed_bookmarks) => {
                let bookmark_readers: Vec<TextBookmarkReader> = vec![SimpleReader::new()];
                Self::import_by_source(
                    source_path,
                    folders,
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
                    folders,
                    source_bookmarks,
                    parsed_bookmarks,
                    bookmark_readers,
                )?;
            }
            ParsedBookmarks::Plist(parsed_bookmarks) => {
                let bookmark_readers: Vec<PlistBookmarkReader> = vec![SafariReader::new()];
                Self::import_by_source(
                    source_path,
                    folders,
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
        folders: &[String],
        source_bookmarks: &mut SourceBookmarks,
        parsed_bookmarks: P,
        bookmark_readers: Vec<BookmarkReader<P>>,
    ) -> Result<(), anyhow::Error> {
        for bookmark_reader in bookmark_readers {
            if let Some(source_type) =
                bookmark_reader.select_source(source_path, &parsed_bookmarks)?
            {
                let source = Source::new(source_type, source_path, folders.to_vec());
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
    use tempfile::tempdir;

    #[cfg(not(any(target_os = "windows")))]
    #[test]
    fn test_select_sources_linux() {
        let source_os = SourceOs::Linux;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        test_utils::tests::create_test_files(temp_path, &source_os);

        let sources = SourceReader::select_sources(temp_path, &source_os).unwrap();
        assert_eq!(sources.len(), 10);
    }

    #[cfg(not(any(target_os = "windows")))]
    #[test]
    fn test_select_sources_macos() {
        let source_os = SourceOs::Macos;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        test_utils::tests::create_test_files(temp_path, &source_os);

        let sources = SourceReader::select_sources(temp_path, &source_os).unwrap();
        assert_eq!(sources.len(), 5);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_select_sources_windows() {
        let source_os = SourceOs::Windows;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        test_utils::tests::create_test_files(temp_path, &source_os);

        let sources = SourceReader::select_sources(temp_path, &source_os).unwrap();
        assert_eq!(sources.len(), 4);
    }

    #[test]
    fn test_init_safari_binary() {
        let source_path = Path::new("test_data/bookmarks_safari_binary.plist");
        test_utils::create_binary_plist_file(source_path).unwrap();
        let folders = vec![];
        let raw_source = RawSource::new(source_path, folders);
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
        let folders = vec![];
        let raw_source = RawSource::new(source_path, folders);
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
        let folders = vec![];
        let raw_source = RawSource::new(source_path, folders);
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
        let folders = vec![];
        let raw_source = RawSource::new(source_path, folders);
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
        let folders = vec![];
        let raw_source = RawSource::new(source_path, folders);
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
        let folders = vec![];
        let raw_source = RawSource::new(source_path, folders);
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
        let folders = vec![];
        let raw_source = RawSource::new(source_path, folders);
        let source_reader = SourceReader::init(&raw_source).unwrap();
        let source = source_reader.source();
        assert_eq!(source.source_type, SourceType::Unknown);
        assert!(source.path.is_file());
        assert_eq!(source.path, source_path);
        assert_eq!(source_reader.source_reader.extension(), Some("txt"));
    }
}
