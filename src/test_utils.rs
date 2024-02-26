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

        plist::to_file_binary(binary_bookmark_path, &parsed_bookmarks)?;

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
    use std::fs::{self, File};

    pub fn create_test_files(home_dir: &Path) {
        let chromium_dir = home_dir.join("snap/chromium/common/chromium");
        let chrome_dir = home_dir.join(".config/google-chrome");
        let edge_dir = home_dir.join(".config/microsoft-edge");

        let browser_dirs = [chromium_dir, chrome_dir, edge_dir];

        for browser_dir in browser_dirs {
            let default_profile_dir = browser_dir.join("Default");
            let profile_dir = browser_dir.join("Profile 1");

            fs::create_dir_all(&default_profile_dir).unwrap();
            fs::create_dir_all(&profile_dir).unwrap();

            let default_profile_file = default_profile_dir.join("Bookmarks");
            let profile_file = profile_dir.join("Bookmarks");
            File::create(&default_profile_file).unwrap();
            File::create(&profile_file).unwrap();
        }

        let safari_dir = home_dir.join("Library/Safari");
        fs::create_dir_all(&safari_dir).unwrap();
        let safari_file = safari_dir.join("Bookmarks.plist");
        utils::create_file(&safari_file).unwrap();
    }
}
