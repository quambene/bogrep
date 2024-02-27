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
    use crate::bookmark_reader::SourceOs;
    use std::fs::{self, File};

    fn create_browser_dirs(browser_dir: &Path) {
        let default_profile_dir = browser_dir.join("Default");
        let profile_dir = browser_dir.join("Profile 1");

        fs::create_dir_all(&default_profile_dir).unwrap();
        fs::create_dir_all(&profile_dir).unwrap();

        let default_profile_file = default_profile_dir.join("Bookmarks");
        let profile_file = profile_dir.join("Bookmarks");
        File::create(&default_profile_file).unwrap();
        File::create(&profile_file).unwrap();
    }

    fn create_chromium_dirs_linux(home_dir: &Path) {
        let browser_dir = home_dir.join("snap/chromium/common/chromium");
        create_browser_dirs(&browser_dir);
    }

    fn create_chrome_dirs_linux(home_dir: &Path) {
        let browser_dir = home_dir.join(".config/google-chrome");
        create_browser_dirs(&browser_dir);
    }

    fn create_chrome_dirs_windows(home_dir: &Path) {
        let browser_dir = home_dir.join("AppData/Local/Google/Chrome/User Data");
        create_browser_dirs(&browser_dir);
    }

    fn create_edge_dirs_linux(home_dir: &Path) {
        let browser_dir = home_dir.join(".config/microsoft-edge");
        create_browser_dirs(&browser_dir);
    }

    fn create_edge_dirs_windows(home_dir: &Path) {
        let browser_dir = home_dir.join("AppData/Local/Microsoft/Edge/User Data");
        create_browser_dirs(&browser_dir);
    }

    fn create_safari_dirs_macos(home_dir: &Path) {
        let safari_dir = home_dir.join("Library/Safari");
        fs::create_dir_all(&safari_dir).unwrap();
        let safari_file = safari_dir.join("Bookmarks.plist");
        utils::create_file(&safari_file).unwrap();
    }

    fn create_firefox_dirs_linux(home_dir: &Path) {
        let browser_dir = home_dir.join("snap/firefox/common/.mozilla/firefox");
        let profile_dir1 = browser_dir.join("profile1.default/bookmarkbackups");
        let profile_dir2 = browser_dir.join("profile1.username/bookmarkbackups");
        fs::create_dir_all(&profile_dir1).unwrap();
        fs::create_dir_all(&profile_dir2).unwrap();
        utils::create_file(&profile_dir1.join("bookmarks.jsonlz4")).unwrap();
        utils::create_file(&profile_dir2.join("bookmarks.jsonlz4")).unwrap();
        let mut file = utils::create_file(&browser_dir.join("profiles.ini")).unwrap();
        let content = r#"
            [Profile2]
            Name=bene
            IsRelative=1
            Path=profile3.username

            [Profile1]
            Name=bene
            IsRelative=1
            Path=profile1.username
            Default=1
            
            [Profile0]
            Name=default
            IsRelative=1
            Path=profile1.default
        "#;
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();

        let browser_dir = home_dir.join(".mozilla/firefox");
        let profile_dir1 = browser_dir.join("profile2.default/bookmarkbackups");
        let profile_dir2 = browser_dir.join("profile2.username/bookmarkbackups");
        fs::create_dir_all(&profile_dir1).unwrap();
        fs::create_dir_all(&profile_dir2).unwrap();
        utils::create_file(&profile_dir1.join("bookmarks.jsonlz4")).unwrap();
        utils::create_file(&profile_dir2.join("bookmarks.jsonlz4")).unwrap();
        let mut file = File::create(browser_dir.join("profiles.ini")).unwrap();
        let content = r#"
            [Profile1]
            Name=bene
            IsRelative=1
            Path=profile2.username
            Default=1
            
            [Profile0]
            Name=default
            IsRelative=1
            Path=profile2.default
        "#;
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
    }

    pub fn create_test_files(home_dir: &Path, source_os: &SourceOs) {
        match source_os {
            SourceOs::Linux => {
                create_firefox_dirs_linux(home_dir);
                create_chromium_dirs_linux(home_dir);
                create_chrome_dirs_linux(home_dir);
                create_edge_dirs_linux(home_dir);
            }
            SourceOs::Macos => {
                create_safari_dirs_macos(home_dir);
            }
            SourceOs::Windows => {
                create_chrome_dirs_windows(home_dir);
                create_edge_dirs_windows(home_dir);
            }
        }
    }
}
