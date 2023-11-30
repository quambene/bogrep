use crate::{errors::BogrepError, json, BookmarksJson, TargetBookmarks};
use std::io::{Read, Seek};

/// Extension trait for [`Read`] and [`Seek`] to read target bookmarks.
pub trait ReadTarget {
    fn read(&mut self, target_bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError>;
}

impl<T> ReadTarget for T
where
    T: Read + Seek,
{
    fn read(&mut self, target_bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError> {
        let mut buf = Vec::new();
        self.read_to_end(&mut buf)
            .map_err(|err| BogrepError::ReadFile(err))?;

        // Rewind after reading.
        self.rewind().map_err(|err| BogrepError::RewindFile(err))?;

        let bookmarks = json::deserialize::<BookmarksJson>(&buf)?;

        for bookmark in bookmarks {
            target_bookmarks.insert(bookmark);
        }

        Ok(())
    }
}
