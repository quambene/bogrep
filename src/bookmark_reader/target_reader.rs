use crate::{errors::BogrepError, json, JsonBookmarks, TargetBookmarks};
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
        self.read_to_end(&mut buf).map_err(BogrepError::ReadFile)?;

        // Rewind after reading.
        self.rewind().map_err(BogrepError::RewindFile)?;

        let bookmarks = json::deserialize::<JsonBookmarks>(&buf)?;

        for bookmark in bookmarks {
            target_bookmarks.insert(bookmark.try_into()?);
        }

        Ok(())
    }
}
