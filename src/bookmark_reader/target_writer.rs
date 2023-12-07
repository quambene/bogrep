use crate::{json, JsonBookmarks, TargetBookmarks};
use anyhow::Context;
use std::io::{Seek, Write};

/// Extension trait for [`Write`] and [`Seek`] to read target bookmarks.
pub trait WriteTarget {
    fn write(&mut self, target_bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error>;
}

impl<T> WriteTarget for T
where
    T: Write + Seek,
{
    fn write(&mut self, target_bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error> {
        let bookmarks = JsonBookmarks::from(target_bookmarks);
        let json = json::serialize(bookmarks)?;

        self.write_all(&json)
            .context("Can't write to `bookmarks.json` file")?;

        // Rewind after writing.
        self.rewind()?;

        Ok(())
    }
}
