use crate::utils;
use lz4::block;
use std::{io::Write, path::Path};

pub fn create_compressed_json_file(compressed_bookmark_path: &Path) -> Result<(), anyhow::Error> {
    if !compressed_bookmark_path.exists() {
        let decompressed_bookmark_path = Path::new("test_data/bookmarks_firefox.json");
        assert!(decompressed_bookmark_path.exists());

        let decompressed_bookmarks = utils::read_file(decompressed_bookmark_path)?;
        let compressed_bookmarks = compress_bookmarks(&decompressed_bookmarks);

        let mut file = utils::create_file(compressed_bookmark_path)?;
        file.write_all(&compressed_bookmarks)?;
        file.flush()?;

        assert!(compressed_bookmark_path.exists());
    }

    Ok(())
}

pub fn create_binary_plist_file(binary_bookmark_path: &Path) -> Result<(), anyhow::Error> {
    if !binary_bookmark_path.exists() {
        let bookmark_path = Path::new("test_data/bookmarks_safari_xml.plist");
        let bookmark_file = utils::open_file(bookmark_path)?;
        let parsed_bookmarks = plist::Value::from_reader_xml(&bookmark_file)?;

        let mut file = utils::create_file(binary_bookmark_path)?;
        plist::to_file_binary(binary_bookmark_path, &parsed_bookmarks)?;
        file.flush().unwrap();

        assert!(binary_bookmark_path.exists());
    }

    Ok(())
}

pub fn compress_bookmarks(decompressed_bookmarks: &[u8]) -> Vec<u8> {
    let compressed_data = block::compress(decompressed_bookmarks, None, true).unwrap();

    // Add non-standard header to data
    let prefix: &[u8] = b"mozLz40\0";
    let mut compressed_data_with_header = Vec::with_capacity(prefix.len() + compressed_data.len());
    compressed_data_with_header.extend_from_slice(prefix);
    compressed_data_with_header.extend_from_slice(&compressed_data);

    compressed_data_with_header
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::fs;

    pub fn create_test_dirs(temp_path: &Path) {
        fs::create_dir_all(temp_path.join("snap/chromium/common/chromium/Default")).unwrap();
        fs::create_dir_all(temp_path.join("snap/chromium/common/chromium/Profile 1")).unwrap();
        fs::create_dir_all(temp_path.join(".config/google-chrome/Default")).unwrap();
        fs::create_dir_all(temp_path.join(".config/google-chrome/Profile 1")).unwrap();
        fs::create_dir_all(temp_path.join(".config/microsoft-edge/Default")).unwrap();
        fs::create_dir_all(temp_path.join(".config/microsoft-edge/Profile 1")).unwrap();
        fs::create_dir_all(temp_path.join("Library/Safari")).unwrap();
    }
}
