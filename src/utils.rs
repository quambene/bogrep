use anyhow::Context;
use log::debug;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

/// Helper function to read a file that logs the path of the file in case of an
/// error.
pub fn read_file(path: &Path) -> Result<Vec<u8>, anyhow::Error> {
    debug!("Read file from {}", path.display());
    let mut buffer = Vec::new();
    let mut file = File::open(path).context(format!("Can't open file at {}", path.display()))?;
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

/// Helper function to read a file to a string that logs the path of the file in
/// case of an error.
pub fn read_file_to_string(path: &Path) -> Result<String, anyhow::Error> {
    debug!("Read file from {}", path.display());
    let mut buffer = String::new();
    let mut file = File::open(path).context(format!("Can't open file at {}", path.display()))?;
    file.read_to_string(&mut buffer)?;
    Ok(buffer)
}

/// Helper function to write a file that logs the path of the file in case of an
/// error.
pub fn write_file(path: &Path, content: String) -> Result<(), anyhow::Error> {
    debug!("Write file to {}", path.display());
    let mut file =
        File::create(path).context(format!("Can't create file at {}", path.display()))?;
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    Ok(())
}

/// Helper function to open a file that logs the path of the file in case of an
/// error.
pub fn open_file(path: &Path) -> Result<File, anyhow::Error> {
    debug!("Open file at {}", path.display());
    let file = File::open(path).context(format!("Can't open file at {}", path.display()))?;
    Ok(file)
}

/// Helper function to open a file in read mode.
pub fn open_file_in_read_mode(path: &Path) -> Result<File, anyhow::Error> {
    debug!("Open file in read mode at {}", path.display());
    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .context(format!("Can't open file at {}", path.display()))?;
    Ok(file)
}

/// Helper function to open a file in read-write mode.
pub fn open_file_in_read_write_mode(path: &Path) -> Result<File, anyhow::Error> {
    debug!("Open file in read and write mode at {}", path.display());
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .context(format!("Can't open file at {}", path.display()))?;
    Ok(file)
}

/// Helper function to open a file and truncate it.
pub fn open_and_truncate_file(path: &Path) -> Result<File, anyhow::Error> {
    debug!("Open file in truncated write mode at {}", path.display());
    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)
        .context(format!("Can't open file at {}", path.display()))?;
    Ok(file)
}

/// Helper function to create a file that logs the path of the file in case of an error.
pub fn create_file(path: &Path) -> Result<File, anyhow::Error> {
    debug!("Create file at {}", path.display());
    let file = File::create(path).context(format!("Can't create file at {}", path.display()))?;
    Ok(file)
}

/// Helper function to create a file that logs the path of the file in case of an error.
pub async fn create_file_async(path: &Path) -> Result<tokio::fs::File, anyhow::Error> {
    debug!("Create file at {}", path.display());
    let file = tokio::fs::File::create(path)
        .await
        .context(format!("Can't create file at {}", path.display()))?;
    Ok(file)
}

/// Helper function to append a file that logs the path of the file in case of an error.
pub fn append_file(path: &Path) -> Result<File, anyhow::Error> {
    debug!("Append file at {}", path.display());
    let file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path)
        .context(format!("Can't append file at {}", path.display()))?;
    Ok(file)
}

/// Helper function to remove a file that logs the path of the file in case of an error.
pub fn remove_file(path: &Path) -> Result<(), anyhow::Error> {
    debug!("Remove file at {}", path.display());
    fs::remove_file(path).context(format!("Can't remove file at {}", path.display()))
}

/// Helper function to close and rename a file.
pub fn close_and_rename(from: (File, &Path), to: (File, &Path)) -> Result<(), anyhow::Error> {
    debug!("Close file at {}", from.1.display());
    drop(from.0);

    debug!("Close file at {}", to.1.display());
    drop(to.0);

    debug!(
        "Rename file from {} to {}",
        from.1.display(),
        to.1.display()
    );
    fs::rename(from.1, to.1)?;

    Ok(())
}
