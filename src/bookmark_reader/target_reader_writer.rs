use super::{ReadTarget, WriteTarget};
use crate::{errors::BogrepError, utils, TargetBookmarks};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

/// A helper struct to read from and write to target files.
pub struct TargetReaderWriter {
    reader: File,
    reader_path: PathBuf,
    writer: File,
    writer_path: PathBuf,
}

impl TargetReaderWriter {
    pub fn new(reader_path: &Path, writer_path: &Path) -> Result<Self, BogrepError> {
        let reader = utils::open_file_in_read_mode(reader_path)?;
        let writer = utils::open_and_truncate_file(writer_path)?;

        Ok(Self {
            reader,
            reader_path: reader_path.to_owned(),
            writer,
            writer_path: writer_path.to_owned(),
        })
    }

    pub fn reader(&self) -> &File {
        &self.reader
    }

    pub fn writer(&self) -> &File {
        &self.writer
    }

    pub fn read(&mut self, target_bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError> {
        self.reader.read(target_bookmarks)
    }

    pub fn write(&mut self, target_bookmarks: &TargetBookmarks) -> Result<(), BogrepError> {
        self.writer.write(target_bookmarks)
    }

    pub fn close(self) -> Result<(), BogrepError> {
        let from = (self.writer, self.writer_path.as_path());
        let to = (self.reader, self.reader_path.as_path());

        utils::close_and_rename(from, to)?;

        Ok(())
    }
}
