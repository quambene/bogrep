use super::{target_reader::convert_underlyings, SeekReadWrite};
use crate::{errors::BogrepError, json, JsonBookmarks, TargetBookmarks};
use anyhow::Context;
use std::{
    fs::File,
    io::{Cursor, Read, Seek, Write},
};

pub type TargetReaderWriter = Box<dyn SeekReadWrite>;

/// Extension trait for [`Read`], [`Write`], and [`Seek`] to read and write bookmarks.
pub trait ReadWriteTarget: SeekReadWrite {
    fn read_target(&mut self, target_bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError>;

    fn write_target(&mut self, target_bookmarks: &TargetBookmarks) -> Result<(), BogrepError>;
}

impl ReadWriteTarget for File {
    fn read_target(&mut self, target_bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError> {
        let mut buf = Vec::new();
        self.read_to_end(&mut buf).map_err(BogrepError::ReadFile)?;

        // Rewind after reading.
        self.rewind().map_err(BogrepError::RewindFile)?;

        let bookmarks = json::deserialize::<JsonBookmarks>(&buf)?;

        for bookmark in bookmarks {
            target_bookmarks.insert(bookmark.try_into()?);
        }

        convert_underlyings(target_bookmarks)?;

        Ok(())
    }

    fn write_target(&mut self, target_bookmarks: &TargetBookmarks) -> Result<(), BogrepError> {
        let bookmarks = JsonBookmarks::from(target_bookmarks);
        let json = json::serialize(bookmarks)?;

        self.write_all(&json).map_err(BogrepError::WriteFile)?;

        // Truncate the file.
        self.set_len(json.len() as u64)
            .context("Can't set length for writer")?;

        // Rewind after writing.
        self.rewind().map_err(BogrepError::RewindFile)?;

        Ok(())
    }
}

impl ReadWriteTarget for Cursor<Vec<u8>> {
    fn read_target(&mut self, target_bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError> {
        let mut buf = Vec::new();
        self.read_to_end(&mut buf).map_err(BogrepError::ReadFile)?;

        // Rewind after reading.
        self.rewind().map_err(BogrepError::RewindFile)?;

        let bookmarks = json::deserialize::<JsonBookmarks>(&buf)?;

        for bookmark in bookmarks {
            target_bookmarks.insert(bookmark.try_into()?);
        }

        convert_underlyings(target_bookmarks)?;

        Ok(())
    }

    fn write_target(&mut self, target_bookmarks: &TargetBookmarks) -> Result<(), BogrepError> {
        let bookmarks = JsonBookmarks::from(target_bookmarks);
        let json = json::serialize(bookmarks)?;

        // Truncate the cursor.
        self.get_mut().clear();

        self.write_all(&json).map_err(BogrepError::WriteFile)?;

        // Rewind after writing.
        self.rewind().map_err(BogrepError::RewindFile)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bookmark_reader::ReadWriteTarget, Action, Status, TargetBookmark, UnderlyingType};
    use std::{
        collections::{HashMap, HashSet},
        io::Cursor,
    };
    use url::Url;

    const EXPECTED_BOOKMARKS: &str = r#"{
    "bookmarks": [
        {
            "id": "a87f7024-a7f5-4f9c-8a71-f64880b2f275",
            "url": "https://url1.com/",
            "last_imported": 1694989714351,
            "last_cached": null,
            "sources": [],
            "cache_modes": []
        },
        {
            "id": "511b1590-e6de-4989-bca4-96dc61730508",
            "url": "https://url2.com/",
            "last_imported": 1694989714351,
            "last_cached": null,
            "sources": [],
            "cache_modes": []
        }
    ]
}"#;

    const EXPECTED_BOOKMARKS_EMPTY: &str = r#"{
    "bookmarks": []
}"#;

    #[test]
    fn test_read_target_bookmarks() {
        let expected_bookmarks = EXPECTED_BOOKMARKS.as_bytes().to_vec();
        let mut target_bookmarks = TargetBookmarks::default();
        let mut target_reader_writer = Cursor::new(expected_bookmarks);

        let res = target_reader_writer.read_target(&mut target_bookmarks);
        assert!(res.is_ok());
        assert_eq!(
            target_bookmarks,
            TargetBookmarks::new(HashMap::from_iter([
                (
                    Url::parse("https://url1.com").unwrap(),
                    TargetBookmark {
                        id: String::from("a87f7024-a7f5-4f9c-8a71-f64880b2f275"),
                        url: Url::parse("https://url1.com").unwrap(),
                        underlying_url: None,
                        underlying_type: UnderlyingType::None,
                        last_imported: 1694989714351,
                        last_cached: None,
                        sources: HashSet::new(),
                        source_folders: HashSet::new(),
                        cache_modes: HashSet::new(),
                        status: Status::None,
                        action: Action::None,
                    }
                ),
                (
                    Url::parse("https://url2.com").unwrap(),
                    TargetBookmark {
                        id: String::from("511b1590-e6de-4989-bca4-96dc61730508"),
                        url: Url::parse("https://url2.com").unwrap(),
                        underlying_url: None,
                        underlying_type: UnderlyingType::None,
                        last_imported: 1694989714351,
                        last_cached: None,
                        sources: HashSet::new(),
                        source_folders: HashSet::new(),
                        cache_modes: HashSet::new(),
                        status: Status::None,
                        action: Action::None,
                    }
                )
            ]))
        );
    }

    #[test]
    fn test_read_target_bookmarks_empty() {
        let expected_bookmarks = EXPECTED_BOOKMARKS_EMPTY.as_bytes().to_vec();
        let mut target_bookmarks = TargetBookmarks::default();
        let mut target_reader_writer = Cursor::new(expected_bookmarks);

        let res = target_reader_writer.read_target(&mut target_bookmarks);
        assert!(res.is_ok());
        assert!(target_bookmarks.is_empty());
    }

    #[test]
    fn test_write_target_bookmarks() {
        let target_bookmarks = TargetBookmarks::new(HashMap::from_iter([
            (
                Url::parse("https://url1.com").unwrap(),
                TargetBookmark {
                    id: String::from("a87f7024-a7f5-4f9c-8a71-f64880b2f275"),
                    url: Url::parse("https://url1.com").unwrap(),
                    underlying_url: None,
                    underlying_type: UnderlyingType::None,
                    last_imported: 1694989714351,
                    last_cached: None,
                    sources: HashSet::new(),
                    source_folders: HashSet::new(),
                    cache_modes: HashSet::new(),
                    status: Status::None,
                    action: Action::None,
                },
            ),
            (
                Url::parse("https://url2.com").unwrap(),
                TargetBookmark {
                    id: String::from("511b1590-e6de-4989-bca4-96dc61730508"),
                    url: Url::parse("https://url2.com").unwrap(),
                    underlying_url: None,
                    underlying_type: UnderlyingType::None,
                    last_imported: 1694989714351,
                    last_cached: None,
                    sources: HashSet::new(),
                    source_folders: HashSet::new(),
                    cache_modes: HashSet::new(),
                    status: Status::None,
                    action: Action::None,
                },
            ),
        ]));
        let mut target_reader_writer = Cursor::new(Vec::new());
        let res = target_reader_writer.write_target(&target_bookmarks);
        assert!(res.is_ok());

        let actual = target_reader_writer.into_inner();
        assert_eq!(String::from_utf8(actual).unwrap(), EXPECTED_BOOKMARKS);
    }

    #[test]
    fn test_write_target_bookmarks_empty() {
        let bookmarks = TargetBookmarks::default();
        let mut target_reader_writer = Cursor::new(Vec::new());
        let res = target_reader_writer.write_target(&bookmarks);
        assert!(res.is_ok());

        let actual = target_reader_writer.into_inner();
        assert_eq!(String::from_utf8(actual).unwrap(), EXPECTED_BOOKMARKS_EMPTY);
    }
}
