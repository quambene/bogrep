use crate::{json, TargetBookmarks};
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
        let bookmarks_json = json::serialize(target_bookmarks)?;
        self.write_all(&bookmarks_json)
            .context("Can't write to `bookmarks.json` file")?;

        // Rewind after writing.
        self.rewind()?;

        Ok(())
    }
}
