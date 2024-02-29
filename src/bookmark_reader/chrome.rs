use super::{chromium::ChromiumSelector, SelectSource, SourceOs};
use crate::SourceType;
use log::debug;
use std::path::{Path, PathBuf};

pub struct ChromeSelector;

impl ChromeSelector {
    pub fn new() -> Box<Self> {
        Box::new(ChromeSelector)
    }
}

impl SelectSource for ChromeSelector {
    fn name(&self) -> SourceType {
        SourceType::Chrome
    }

    fn extension(&self) -> Option<&str> {
        Some("json")
    }

    fn find_sources(
        &self,
        home_dir: &Path,
        source_os: &SourceOs,
    ) -> Result<Vec<PathBuf>, anyhow::Error> {
        debug!("Find sources for {}", self.name());

        let browser_dirs = match source_os {
            SourceOs::Linux => vec![
                // apt package
                home_dir.join(".config/google-chrome"),
            ],
            SourceOs::Windows => vec![home_dir.join("AppData\\Local\\Google\\Chrome\\User Data")],
            SourceOs::Macos => vec![home_dir.join("Library/Application Support/Google/Chrome")],
        };
        let bookmark_dirs = ChromiumSelector::find_profile_dirs(&browser_dirs);
        let bookmark_files = bookmark_dirs
            .into_iter()
            .filter_map(|bookmark_dir| {
                let bookmark_file = bookmark_dir.join("Bookmarks");

                if bookmark_file.is_file() {
                    Some(bookmark_file)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(bookmark_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::tests;
    use tempfile::tempdir;

    #[test]
    fn test_selector_name() {
        let selector = ChromeSelector;
        assert_eq!(selector.name(), SourceType::Chrome);
    }

    #[test]
    fn test_find_sources_empty() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        let selector = ChromeSelector;

        let res = selector.find_sources(temp_path, &SourceOs::Linux);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let sources = res.unwrap();
        assert!(sources.is_empty());

        let res = selector.find_sources(temp_path, &SourceOs::Macos);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let sources = res.unwrap();
        assert!(sources.is_empty());

        let res = selector.find_sources(temp_path, &SourceOs::Windows);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let sources = res.unwrap();
        assert!(sources.is_empty());
    }

    #[cfg(not(any(target_os = "windows")))]
    #[test]
    fn test_find_sources_linux() {
        let source_os = SourceOs::Linux;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        tests::create_test_files(temp_path, &source_os);

        let selector = ChromeSelector;
        let res: Result<Vec<PathBuf>, anyhow::Error> = selector.find_sources(temp_path, &source_os);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_dirs = res.unwrap();
        assert_eq!(bookmark_dirs.len(), 2);
        assert!(bookmark_dirs.contains(&temp_path.join(".config/google-chrome/Default/Bookmarks")));
        assert!(
            bookmark_dirs.contains(&temp_path.join(".config/google-chrome/Profile 1/Bookmarks"))
        );
    }

    #[cfg(not(any(target_os = "windows")))]
    #[test]
    fn test_find_sources_macos() {
        let source_os = SourceOs::Macos;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        tests::create_test_files(temp_path, &source_os);

        let selector = ChromeSelector;
        let res: Result<Vec<PathBuf>, anyhow::Error> = selector.find_sources(temp_path, &source_os);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_dirs = res.unwrap();
        assert_eq!(bookmark_dirs.len(), 2);
        assert!(bookmark_dirs.contains(
            &temp_path.join("Library/Application Support/Google/Chrome/Default/Bookmarks")
        ));
        assert!(bookmark_dirs.contains(
            &temp_path.join("Library/Application Support/Google/Chrome/Profile 1/Bookmarks")
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_find_sources_windows() {
        let source_os = SourceOs::Windows;
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        tests::create_test_files(temp_path, &source_os);

        let selector = ChromeSelector;
        let res: Result<Vec<PathBuf>, anyhow::Error> = selector.find_sources(temp_path, &source_os);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_dirs = res.unwrap();
        assert_eq!(bookmark_dirs.len(), 2);
        assert!(bookmark_dirs.contains(
            &temp_path.join("AppData\\Local\\Google\\Chrome\\User Data\\Default\\Bookmarks")
        ));
        assert!(bookmark_dirs.contains(
            &temp_path.join("AppData\\Local\\Google\\Chrome\\User Data\\Profile 1\\Bookmarks")
        ));
    }
}
