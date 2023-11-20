use crate::{json, BookmarksJson, TargetBookmarks};
use anyhow::Context;
use std::io::{Read, Seek};

/// Extension trait for [`Read`] and [`Seek`] to read target bookmarks.
pub trait ReadTarget {
    fn read(&mut self, target_bookmarks: &mut TargetBookmarks) -> Result<(), anyhow::Error>;
}

impl<T> ReadTarget for T
where
    T: Read + Seek,
{
    fn read(&mut self, target_bookmarks: &mut TargetBookmarks) -> Result<(), anyhow::Error> {
        let mut buf = Vec::new();
        self.read_to_end(&mut buf)
            .context("Can't read from `bookmarks.json` file:")?;

        // Rewind after reading.
        self.rewind()?;

        let bookmarks = json::deserialize::<BookmarksJson>(&buf)?;

        for bookmark in bookmarks {
            target_bookmarks.insert(bookmark);
        }

        Ok(())
    }
}
