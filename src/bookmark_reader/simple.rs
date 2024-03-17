use super::{ReadBookmark, SeekRead};
use crate::{
    bookmarks::{Source, SourceBookmarkBuilder},
    SourceBookmarks, SourceType,
};
use log::debug;
use std::{
    io::{BufReader, Lines},
    path::Path,
};

pub type LinesReader<'a> = Lines<BufReader<&'a mut dyn SeekRead>>;
pub type TextBookmarkReader<'a> = Box<dyn ReadBookmark<'a, ParsedValue = LinesReader<'a>>>;

/// A bookmark reader to read bookmarks from a simple text file with one url per
/// line.
#[derive(Debug)]
pub struct SimpleReader;

impl SimpleReader {
    pub fn new() -> Box<Self> {
        Box::new(Self)
    }
}

impl<'a> ReadBookmark<'a> for SimpleReader {
    type ParsedValue = LinesReader<'a>;

    fn name(&self) -> SourceType {
        SourceType::Simple
    }

    fn extension(&self) -> Option<&str> {
        Some("txt")
    }

    fn select_source(
        &self,
        _source_path: &Path,
        _parsed_bookmarks: &Self::ParsedValue,
    ) -> Result<Option<SourceType>, anyhow::Error> {
        Ok(Some(SourceType::Simple))
    }

    fn import(
        &self,
        source: &Source,
        parsed_bookmarks: Self::ParsedValue,
        source_bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        debug!("Import bookmarks from {:#?}", self.name());

        for line in parsed_bookmarks {
            let url = line?;

            if !url.is_empty() {
                let source_bookmark = SourceBookmarkBuilder::new(&url)
                    .add_source(source.source_type.to_owned())
                    .build();
                source_bookmarks.insert(source_bookmark);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bookmark_reader::{ParsedBookmarks, ReadSource, SourceReader, TextReader},
        bookmarks::SourceBookmarkBuilder,
        utils,
    };
    use assert_matches::assert_matches;
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
    };

    #[test]
    fn test_read_and_parse() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        let mut reader = utils::open_file(source_path).unwrap();
        let source_reader = TextReader;

        let res = source_reader.read_and_parse(&mut reader);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let parsed_bookmarks = res.unwrap();
        assert_matches!(parsed_bookmarks, ParsedBookmarks::Text(_));
    }

    #[test]
    fn test_import_txt() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        assert!(source_path.exists());

        let mut source_bookmarks = SourceBookmarks::default();
        let source = Source::new(SourceType::Unknown, &PathBuf::from("dummy_path"), vec![]);
        let bookmark_file = utils::open_file(source_path).unwrap();
        let source_reader = Box::new(TextReader);
        let mut source_reader = SourceReader::new(source, Box::new(bookmark_file), source_reader);

        let res = source_reader.import(&mut source_bookmarks);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let url1 = "https://www.deepl.com/translator";
        let url2 =
            "https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/";
        let url3 = "https://en.wikipedia.org/wiki/Design_Patterns";
        let url4 = "https://doc.rust-lang.org/book/title-page.html";

        assert_eq!(
            source_bookmarks.inner(),
            HashMap::from_iter([
                (
                    url1.to_owned(),
                    SourceBookmarkBuilder::new(url1)
                        .add_source(SourceType::Simple)
                        .build()
                ),
                (
                    url2.to_owned(),
                    SourceBookmarkBuilder::new(url2)
                        .add_source(SourceType::Simple)
                        .build()
                ),
                (
                    url3.to_owned(),
                    SourceBookmarkBuilder::new(url3)
                        .add_source(SourceType::Simple)
                        .build()
                ),
                (
                    url4.to_owned(),
                    SourceBookmarkBuilder::new(url4)
                        .add_source(SourceType::Simple)
                        .build()
                )
            ])
        );
    }
}
