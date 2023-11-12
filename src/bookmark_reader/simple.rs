use super::ReadBookmark;
use crate::{Source, SourceBookmarks};
use std::io::{BufRead, BufReader, Read};

/// A bookmark reader to read bookmarks from a simple text file with one url per
/// line.
#[derive(Debug)]
pub struct SimpleBookmarkReader;

impl ReadBookmark for SimpleBookmarkReader {
    fn name(&self) -> &str {
        "simple"
    }

    fn extension(&self) -> Option<&str> {
        Some("txt")
    }

    fn is_selected(&self, _raw_bookmarks: &str) -> Result<bool, anyhow::Error> {
        Ok(true)
    }

    fn read_and_parse(
        &self,
        reader: &mut dyn Read,
        _source: &Source,
        bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        let buf_reader = BufReader::new(reader);

        for line in buf_reader.lines() {
            let url = line?;

            if !url.is_empty() {
                bookmarks.insert(&url);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils;
    use std::{collections::HashSet, path::Path};

    #[test]
    fn test_read_txt() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        assert!(source_path.exists());
        let mut source_bookmark_file = utils::open_file(source_path).unwrap();

        let mut source_bookmarks = SourceBookmarks::new();
        let source = Source::new(source_path, vec![]);
        let bookmark_reader = SimpleBookmarkReader;

        let res = bookmark_reader.read_and_parse(
            &mut source_bookmark_file,
            &source,
            &mut source_bookmarks,
        );
        assert!(res.is_ok(), "{}", res.unwrap_err());

        assert_eq!(source_bookmarks.bookmarks, HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]))
    }
}
