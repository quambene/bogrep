use super::{ReadBookmark, ReaderName, SeekRead};
use crate::{
    bookmarks::{Source, SourceBookmarkBuilder},
    SourceBookmarks, SourceType,
};
use std::{
    io::{BufReader, Lines},
    path::Path,
};

/// A bookmark reader to read bookmarks from a simple text file with one url per
/// line.
#[derive(Debug)]
pub struct SimpleBookmarkReader;

impl<'a> ReadBookmark<'a> for SimpleBookmarkReader {
    type ParsedValue = Lines<BufReader<&'a mut dyn SeekRead>>;

    fn name(&self) -> ReaderName {
        ReaderName::Simple
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
        for line in parsed_bookmarks {
            let url = line?;

            if !url.is_empty() {
                let source_bookmark = SourceBookmarkBuilder::new(&url)
                    .add_source(&source.name)
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
        bookmark_reader::{source_reader::TextReader, SourceReader},
        bookmarks::{RawSource, SourceBookmarkBuilder},
        utils,
    };
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
    };

    #[test]
    fn test_import_txt() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        assert!(source_path.exists());

        let mut source_bookmarks = SourceBookmarks::default();
        let source = RawSource::new(&PathBuf::from("dummy_path"), vec![]);
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
                        .add_source(&SourceType::Simple)
                        .build()
                ),
                (
                    url2.to_owned(),
                    SourceBookmarkBuilder::new(url2)
                        .add_source(&SourceType::Simple)
                        .build()
                ),
                (
                    url3.to_owned(),
                    SourceBookmarkBuilder::new(url3)
                        .add_source(&SourceType::Simple)
                        .build()
                ),
                (
                    url4.to_owned(),
                    SourceBookmarkBuilder::new(url4)
                        .add_source(&SourceType::Simple)
                        .build()
                )
            ])
        );
    }
}
