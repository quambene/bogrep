use crate::{json, TargetBookmarks};
use anyhow::Context;
use std::io::{Read, Seek, Write};

pub struct TargetReaderWriter<T: Read + Write + Seek>(T);

impl<T: Read + Write + Seek> TargetReaderWriter<T> {
    pub fn new(reader_writer: T) -> Self {
        Self(reader_writer)
    }

    pub fn read(&mut self, target_bookmarks: &mut TargetBookmarks) -> Result<(), anyhow::Error> {
        let mut buf = Vec::new();
        self.0
            .read_to_end(&mut buf)
            .context("Can't read from `bookmarks.json` file:")?;

        // Rewind after reading.
        self.0.rewind()?;

        *target_bookmarks = json::deserialize::<TargetBookmarks>(&buf)?;
        Ok(())
    }

    pub fn write(&mut self, target_bookmarks: &TargetBookmarks) -> Result<(), anyhow::Error> {
        let bookmarks_json = json::serialize(target_bookmarks)?;
        self.0
            .write_all(&bookmarks_json)
            .context("Can't write to `bookmarks.json` file")?;

        // Rewind after writing.
        self.0.rewind()?;

        Ok(())
    }

    #[cfg(test)]
    pub fn inner(self) -> T {
        self.0
    }
}
