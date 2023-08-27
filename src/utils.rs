use anyhow::Context;
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

pub fn read_file(bookmark_path: &Path) -> Result<Vec<u8>, anyhow::Error> {
    let mut bookmarks = Vec::new();
    let mut file = File::open(bookmark_path)
        .context(format!("Can't open file at {}", bookmark_path.display()))?;
    file.read_to_end(&mut bookmarks)?;
    Ok(bookmarks)
}

pub fn write_file(bookmark_path: &Path, content: String) -> Result<(), anyhow::Error> {
    let mut file = File::create(bookmark_path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    Ok(())
}
