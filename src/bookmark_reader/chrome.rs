use super::{chromium::ChromiumSelector, SelectSource, SourceOs};
use crate::SourceType;
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

    fn source_os(&self) -> SourceOs {
        SourceOs::Linux
    }

    fn extension(&self) -> Option<&str> {
        Some("json")
    }

    fn find_sources(&self, home_dir: &Path) -> Result<Vec<PathBuf>, anyhow::Error> {
        let browser_dirs = [
            // apt package
            home_dir.join(".config/google-chrome"),
        ];
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
    fn test_find_sources() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        tests::create_test_files(temp_path);

        let selector = ChromeSelector;
        let res: Result<Vec<PathBuf>, anyhow::Error> = selector.find_sources(temp_path);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_dirs = res.unwrap();
        assert_eq!(bookmark_dirs.len(), 2);
        assert!(bookmark_dirs.contains(&temp_path.join(".config/google-chrome/Default/Bookmarks")));
        assert!(
            bookmark_dirs.contains(&temp_path.join(".config/google-chrome/Profile 1/Bookmarks"))
        );
    }
}
