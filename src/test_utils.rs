#[cfg(test)]
use crate::utils;
#[cfg(test)]
use lz4::block;
#[cfg(test)]
use std::{io::Write, path::Path};

#[cfg(test)]
pub fn create_compressed_bookmarks(compressed_bookmark_path: &Path) {
    if !compressed_bookmark_path.exists() {
        let decompressed_bookmark_path = Path::new("test_data/source/bookmarks_firefox.json");
        assert!(decompressed_bookmark_path.exists());

        let decompressed_bookmarks = utils::read_file(decompressed_bookmark_path).unwrap();
        compress_bookmarks(&decompressed_bookmarks, compressed_bookmark_path);
        assert!(compressed_bookmark_path.exists());
    }
}

#[cfg(test)]
pub fn compress_bookmarks(decompressed_bookmarks: &[u8], compressed_bookmark_path: &Path) {
    let compressed_data = block::compress(decompressed_bookmarks, None, true).unwrap();

    // Add non-standard header to data
    let prefix: &[u8] = b"mozLz40\0";
    let mut compressed_data_with_header = Vec::with_capacity(prefix.len() + compressed_data.len());
    compressed_data_with_header.extend_from_slice(prefix);
    compressed_data_with_header.extend_from_slice(&compressed_data);

    let mut file = utils::create_file(compressed_bookmark_path).unwrap();
    file.write_all(&compressed_data_with_header).unwrap();
    file.flush().unwrap();
}
