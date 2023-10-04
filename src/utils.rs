use anyhow::Context;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

/// Helper function to read a file that logs the path of the file in case of an
/// error.
pub fn read_file(path: &Path) -> Result<Vec<u8>, anyhow::Error> {
    let mut buffer = Vec::new();
    let mut file = File::open(path).context(format!("Can't open file at {}", path.display()))?;
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

/// Helper function to read a file to a string that logs the path of the file in
/// case of an error.
pub fn read_file_to_string(path: &Path) -> Result<String, anyhow::Error> {
    let mut buffer = String::new();
    let mut file = File::open(path).context(format!("Can't open file at {}", path.display()))?;
    file.read_to_string(&mut buffer)?;
    Ok(buffer)
}

/// Helper function to write a file that logs the path of the file in case of an
/// error.
pub fn write_file(path: &Path, content: String) -> Result<(), anyhow::Error> {
    let mut file =
        File::create(path).context(format!("Can't create file at {}", path.display()))?;
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    Ok(())
}

/// Helper function to open a file that logs the path of the file in case of an
/// error.
pub fn open_file(path: &Path) -> Result<File, anyhow::Error> {
    let file = File::open(path).context(format!("Can't open file at {}", path.display()))?;
    Ok(file)
}

pub fn open_file_in_read_write_mode(path: &Path) -> Result<File, anyhow::Error> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        // .append(false)
        .open(path)
        .context(format!("Can't open file at {}", path.display()))?;
    Ok(file)
}

/// Helper function to create a file that logs the path of the file in case of an error.
pub fn create_file(path: &Path) -> Result<File, anyhow::Error> {
    let file = File::create(path).context(format!("Can't create file at {}", path.display()))?;
    Ok(file)
}

/// Helper function to create a file that logs the path of the file in case of an error.
pub async fn create_file_async(path: &Path) -> Result<tokio::fs::File, anyhow::Error> {
    let file = tokio::fs::File::create(path)
        .await
        .context(format!("Can't create file at {}", path.display()))?;
    Ok(file)
}

/// Helper function to append a file that logs the path of the file in case of an error.
pub fn append_file(path: &Path) -> Result<File, anyhow::Error> {
    let file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path)
        .context(format!("Can't append file at {}", path.display()))?;
    Ok(file)
}

/// Helper function to remove a file that logs the path of the file in case of an error.
pub fn remove_file(path: &Path) -> Result<(), anyhow::Error> {
    fs::remove_file(path).context(format!("Can't remove file at {}", path.display()))
}
