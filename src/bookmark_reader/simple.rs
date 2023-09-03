use crate::{utils, BookmarkReader, SourceBookmarks, SourceFile};
use anyhow::Context;
use std::io::{BufRead, BufReader};

pub struct SimpleBookmarkReader;

impl BookmarkReader for SimpleBookmarkReader {
    const NAME: &'static str = "text file";

    fn read_and_parse(
        &self,
        source_file: &SourceFile,
        bookmarks: &mut SourceBookmarks,
    ) -> Result<(), anyhow::Error> {
        let bookmark_file = utils::open_file(&source_file.source).context(format!(
            "Can't open source file at {}",
            source_file.source.display()
        ))?;
        // TODO: increase buffer size
        let reader = BufReader::new(bookmark_file);

        for line in reader.lines() {
            let url = line?;
            bookmarks.insert(&url);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, path::Path};

    #[test]
    fn test_read_txt() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        assert!(source_path.exists());

        let mut source_bookmarks = SourceBookmarks::new();
        let source_file = SourceFile::new(source_path, vec![]);
        let bookmark_reader = SimpleBookmarkReader;
        let res = bookmark_reader.read_and_parse(&source_file, &mut source_bookmarks);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        assert_eq!(source_bookmarks.bookmarks, HashSet::from_iter([
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://www.quantamagazine.org/how-galois-groups-used-polynomial-symmetries-to-reshape-math-20210803/"),
            String::from("https://www.quantamagazine.org/computing-expert-says-programmers-need-more-math-20220517/"),
        ]))
    }
}
