use crate::{errors::BogrepError, json, JsonBookmarks, TargetBookmarks};
use std::io::{Seek, Write};

/// Extension trait for [`Write`] and [`Seek`] to read target bookmarks.
pub trait WriteTarget {
    fn write(&mut self, target_bookmarks: &TargetBookmarks) -> Result<(), BogrepError>;
}

impl<T> WriteTarget for T
where
    T: Write + Seek,
{
    fn write(&mut self, target_bookmarks: &TargetBookmarks) -> Result<(), BogrepError> {
        let bookmarks = JsonBookmarks::from(target_bookmarks);
        let json = json::serialize(&bookmarks)?;

        self.write_all(&json).map_err(BogrepError::WriteFile)?;

        self.flush().map_err(BogrepError::FlushFile)?;

        // Rewind after writing.
        self.rewind().map_err(BogrepError::RewindFile)?;

        Ok(())
    }
}
