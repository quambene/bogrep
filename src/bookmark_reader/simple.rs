use super::{ReadBookmark, ReaderName};
use crate::{
    bookmarks::{Source, SourceBookmarkBuilder},
    SourceBookmarks, SourceType,
};
use std::{
    io::{BufRead, BufReader, Read},
    path::Path,
};

/// A bookmark reader to read bookmarks from a simple text file with one url per
/// line.
#[derive(Debug)]
pub struct SimpleBookmarkReader;

impl ReadBookmark for SimpleBookmarkReader {
    fn name(&self) -> ReaderName {
        ReaderName::Simple
    }

    fn extension(&self) -> Option<&str> {
        Some("txt")
    }

    fn select_source(&self, _source_path: &Path) -> Result<Option<SourceType>, anyhow::Error> {
        Ok(Some(SourceType::Simple))
    }

    fn read_and_parse(
        &self,
        reader: &mut dyn Read,
        source: &Source,
        source_bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        let buf_reader = BufReader::new(reader);

        for line in buf_reader.lines() {
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
    use crate::{bookmarks::SourceBookmarkBuilder, utils};
    use std::{collections::HashMap, path::Path};

    #[test]
    fn test_read_txt() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        assert!(source_path.exists());
        let mut source_bookmark_file = utils::open_file(source_path).unwrap();

        let mut source_bookmarks = SourceBookmarks::default();
        let source = Source::new(SourceType::Simple, source_path, vec![]);
        let bookmark_reader = SimpleBookmarkReader;

        let res = bookmark_reader.read_and_parse(
            &mut source_bookmark_file,
            &source,
            &mut source_bookmarks,
        );
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
