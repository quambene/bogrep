use crate::errors::BogrepError;
use log::debug;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
use tokio::io::AsyncWriteExt;

/// Helper function to read a file that logs the path of the file in case of an
/// error.
pub fn read_file(path: &Path) -> Result<Vec<u8>, BogrepError> {
    debug!("Read file from {}", path.display());
    let mut buffer = Vec::new();
    let mut file = open_file(path)?;
    file.read_to_end(&mut buffer)
        .map_err(BogrepError::ReadFile)?;
    Ok(buffer)
}

/// Helper function to read a file to a string that logs the path of the file in
/// case of an error.
pub fn read_file_to_string(path: &Path) -> Result<String, BogrepError> {
    debug!("Read file from {}", path.display());
    let mut buffer = String::new();
    let mut file = open_file(path)?;
    file.read_to_string(&mut buffer)
        .map_err(BogrepError::ReadFile)?;
    Ok(buffer)
}

/// Helper function to write a file that logs the path of the file in case of an
/// error.
pub fn write_file(path: &Path, content: String) -> Result<(), BogrepError> {
    debug!("Write file to {}", path.display());
    let mut file = create_file(path)?;
    file.write_all(content.as_bytes())
        .map_err(|err| BogrepError::WriteFilePath {
            path: path.to_string_lossy().to_string(),
            err,
        })?;
    file.flush().map_err(BogrepError::FlushFile)?;
    Ok(())
}

/// Helper function to write a file that logs the path of the file in case of an
/// error.
pub async fn write_file_async(path: &Path, content: &[u8]) -> Result<(), BogrepError> {
    debug!("Write file to {}", path.display());
    let mut file = create_file_async(path).await?;
    file.write_all(content)
        .await
        .map_err(|err| BogrepError::WriteFilePath {
            path: path.to_string_lossy().to_string(),
            err,
        })?;
    file.flush().await.map_err(BogrepError::FlushFile)?;
    Ok(())
}

/// Helper function to open a file that logs the path of the file in case of an
/// error.
pub fn open_file(path: &Path) -> Result<File, BogrepError> {
    debug!("Open file at {}", path.display());
    let file = File::open(path).map_err(|err| BogrepError::OpenFile {
        path: path.to_string_lossy().to_string(),
        err,
    })?;
    Ok(file)
}

/// Helper function to open a file in read mode.
pub fn open_file_in_read_mode(path: &Path) -> Result<File, BogrepError> {
    debug!("Open file in read mode at {}", path.display());
    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|err| BogrepError::OpenFile {
            path: path.to_string_lossy().to_string(),
            err,
        })?;
    Ok(file)
}

/// Helper function to open a file in read-write mode.
pub fn open_file_in_read_write_mode(path: &Path) -> Result<File, BogrepError> {
    debug!("Open file in read and write mode at {}", path.display());
    let file = File::options()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|err| BogrepError::OpenFile {
            path: path.to_string_lossy().to_string(),
            err,
        })?;
    Ok(file)
}

/// Helper function to open a file and truncate it.
pub fn open_and_truncate_file(path: &Path) -> Result<File, BogrepError> {
    debug!("Open file in truncated write mode at {}", path.display());
    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)
        .map_err(|err| BogrepError::OpenFile {
            path: path.to_string_lossy().to_string(),
            err,
        })?;
    Ok(file)
}

/// Helper function to create a file that logs the path of the file in case of an error.
pub fn create_file(path: &Path) -> Result<File, BogrepError> {
    debug!("Create file at {}", path.display());
    let file = File::create(path).map_err(|err| BogrepError::CreateFile {
        path: path.to_string_lossy().to_string(),
        err,
    })?;
    Ok(file)
}

/// Helper function to create a file that logs the path of the file in case of an error.
pub async fn create_file_async(path: &Path) -> Result<tokio::fs::File, BogrepError> {
    debug!("Create file at {}", path.display());
    let file = tokio::fs::File::create(path)
        .await
        .map_err(|err| BogrepError::CreateFile {
            path: path.to_string_lossy().to_string(),
            err,
        })?;
    Ok(file)
}

/// Helper function to append a file that logs the path of the file in case of an error.
pub fn append_file(path: &Path) -> Result<File, BogrepError> {
    debug!("Append file at {}", path.display());
    let file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path)
        .map_err(|err| BogrepError::AppendFile {
            path: path.to_string_lossy().to_string(),
            err,
        })?;
    Ok(file)
}

/// Helper function to remove a file that logs the path of the file in case of an error.
pub fn remove_file(path: &Path) -> Result<(), BogrepError> {
    debug!("Remove file at {}", path.display());
    fs::remove_file(path).map_err(|err| BogrepError::RemoveFile {
        path: path.to_string_lossy().to_string(),
        err,
    })
}

/// Helper function to remove a file that logs the path of the file in case of an error.
pub async fn remove_file_async(path: &Path) -> Result<(), BogrepError> {
    debug!("Remove file at {}", path.display());
    tokio::fs::remove_file(path)
        .await
        .map_err(|err| BogrepError::RemoveFile {
            path: path.to_string_lossy().to_string(),
            err,
        })
}

/// Helper function to close and rename a file.
pub fn close_and_rename(from: (File, &Path), to: (File, &Path)) -> Result<(), BogrepError> {
    debug!("Close file at {}", from.1.display());
    drop(from.0);

    debug!("Close file at {}", to.1.display());
    drop(to.0);

    debug!(
        "Rename file from {} to {}",
        from.1.display(),
        to.1.display()
    );
    fs::rename(from.1, to.1).map_err(|err| BogrepError::RenameFile {
        from: from.1.to_string_lossy().to_string(),
        to: to.1.to_string_lossy().to_string(),
        err,
    })?;

    Ok(())
}
