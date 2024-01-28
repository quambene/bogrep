use super::{ReadTarget, WriteTarget};
use crate::{errors::BogrepError, utils, TargetBookmarks};
use log::debug;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

/// A helper struct to read from and write to target files.
///
/// Cleans up the lock file when dropped.
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

    pub fn close(&self) -> Result<(), BogrepError> {
        let from = &self.writer_path;
        let to = &self.reader_path;

        debug!("Rename file from {} to {}", from.display(), to.display());
        fs::rename(from, to).map_err(|err| BogrepError::RenameFile {
            from: from.to_string_lossy().to_string(),
            to: to.to_string_lossy().to_string(),
            err,
        })?;
        Ok(())
    }
}

impl Drop for TargetReaderWriter {
    fn drop(&mut self) {
        if self.writer_path.exists() {
            // Clean up lock file.
            if let Err(err) = fs::remove_file(&self.writer_path) {
                eprintln!("Can't remove lock file: {err:?}")
            }
        }
    }
}
