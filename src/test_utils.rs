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

    const PROFILES_INI_LINUX: &str = r#"
        [Profile2]
        Name=bene
        IsRelative=1
        Path=profile3.username

        [Profile1]
        Name=bene
        IsRelative=1
        Path=profile2.username
        Default=1
            
        [Profile0]
        Name=default
        IsRelative=1
        Path=profile1.default
    "#;

    const PROFILES_INI_MACOS: &str = r#"
        [Profile2]
        Name=bene
        IsRelative=1
        Path=Profiles/profile3.username

        [Profile1]
        Name=bene
        IsRelative=1
        Path=Profiles/profile2.username
        Default=1
            
        [Profile0]
        Name=default
        IsRelative=1
        Path=Profiles/profile1.default
    "#;

    fn create_chromium_dirs(browser_dir: &Path) {
        let default_profile_dir = browser_dir.join("Default");
        let profile_dir = browser_dir.join("Profile 1");

        fs::create_dir_all(&default_profile_dir).unwrap();
        fs::create_dir_all(&profile_dir).unwrap();

        let default_profile_file = default_profile_dir.join("Bookmarks");
        let profile_file = profile_dir.join("Bookmarks");
        File::create(default_profile_file).unwrap();
        File::create(profile_file).unwrap();
    }

    fn create_safari_dirs(browser_dir: &Path) {
        fs::create_dir_all(browser_dir).unwrap();
        utils::create_file(&browser_dir.join("Bookmarks.plist")).unwrap();
    }

    fn create_firefox_dirs_linux(browser_dir: &Path) {
        let profile_dir1 = browser_dir.join("profile1.default/bookmarkbackups");
        let profile_dir2 = browser_dir.join("profile2.username/bookmarkbackups");
        fs::create_dir_all(&profile_dir1).unwrap();
        fs::create_dir_all(&profile_dir2).unwrap();
        utils::create_file(&profile_dir1.join("bookmarks.jsonlz4")).unwrap();
        utils::create_file(&profile_dir2.join("bookmarks.jsonlz4")).unwrap();
        let mut file = utils::create_file(&browser_dir.join("profiles.ini")).unwrap();
        file.write_all(PROFILES_INI_LINUX.as_bytes()).unwrap();
        file.flush().unwrap();
    }

    fn create_firefox_dirs_macos(browser_dir: &Path) {
        let profile_dir1 = browser_dir.join("Profiles/profile1.default/bookmarkbackups");
        let profile_dir2 = browser_dir.join("Profiles/profile2.username/bookmarkbackups");
        fs::create_dir_all(&profile_dir1).unwrap();
        fs::create_dir_all(&profile_dir2).unwrap();
        utils::create_file(&profile_dir1.join("bookmarks.jsonlz4")).unwrap();
        utils::create_file(&profile_dir2.join("bookmarks.jsonlz4")).unwrap();
        let mut file = utils::create_file(&browser_dir.join("profiles.ini")).unwrap();
        file.write_all(PROFILES_INI_MACOS.as_bytes()).unwrap();
        file.flush().unwrap();
    }

    pub fn create_test_files(home_dir: &Path, source_os: &SourceOs) {
        match source_os {
            SourceOs::Linux => {
                let browser_dir = home_dir.join("snap/firefox/common/.mozilla/firefox");
                create_firefox_dirs_linux(&browser_dir);

                let browser_dir = home_dir.join(".mozilla/firefox");
                create_firefox_dirs_linux(&browser_dir);

                let browser_dir = home_dir.join("snap/chromium/common/chromium");
                create_chromium_dirs(&browser_dir);

                let browser_dir = home_dir.join(".config/google-chrome");
                create_chromium_dirs(&browser_dir);

                let browser_dir = home_dir.join(".config/microsoft-edge");
                create_chromium_dirs(&browser_dir);
            }
            SourceOs::Macos => {
                let browser_dir = home_dir.join("Library/Safari");
                create_safari_dirs(&browser_dir);

                let browser_dir = home_dir.join("Library/Application Support/Firefox");
                create_firefox_dirs_macos(&browser_dir);

                let browser_dir = home_dir.join("Library/Application Support/Google/Chrome");
                create_chromium_dirs(&browser_dir);
            }
            SourceOs::Windows => {
                let browser_dir = home_dir.join("AppData\\Local\\Google\\Chrome\\User Data");
                create_chromium_dirs(&browser_dir);

                let browser_dir = home_dir.join("AppData\\Local\\Microsoft\\Edge\\User Data");
                create_chromium_dirs(&browser_dir);
            }
        }
    }
}
