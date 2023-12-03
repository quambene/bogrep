use crate::utils;
use lz4::block;
use std::{io::Write, path::Path};

pub fn create_compressed_bookmarks(compressed_bookmark_path: &Path) {
    if !compressed_bookmark_path.exists() {
        let decompressed_bookmark_path = Path::new("test_data/bookmarks_firefox.json");
        assert!(decompressed_bookmark_path.exists());

        let decompressed_bookmarks = utils::read_file(decompressed_bookmark_path).unwrap();
        let compressed_bookmarks = compress_bookmarks(&decompressed_bookmarks);

        let mut file = utils::create_file(compressed_bookmark_path).unwrap();
        file.write_all(&compressed_bookmarks).unwrap();
        file.flush().unwrap();

        assert!(compressed_bookmark_path.exists());
    }
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
