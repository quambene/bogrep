use super::{chromium::ChromiumSelector, SelectSource, SourceOs};
use crate::SourceType;
use log::debug;
use std::path::{Path, PathBuf};

pub struct EdgeSelector;

impl EdgeSelector {
    pub fn new() -> Box<Self> {
        Box::new(EdgeSelector)
    }
}

impl SelectSource for EdgeSelector {
    fn name(&self) -> SourceType {
        SourceType::Edge
    }

    fn source_os(&self) -> SourceOs {
        SourceOs::Linux
    }

    fn extension(&self) -> Option<&str> {
        Some("json")
    }

    fn find_sources(&self, home_dir: &Path) -> Result<Vec<PathBuf>, anyhow::Error> {
        debug!("Find sources for {}", self.name());
        let browser_dirs = [
            // apt package
            home_dir.join(".config/microsoft-edge"),
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
        let selector = EdgeSelector;
        assert_eq!(selector.name(), SourceType::Edge);
    }

    #[test]
    fn test_find_sources_empty() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        let selector = EdgeSelector;
        let res = selector.find_sources(temp_path);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let sources = res.unwrap();
        assert!(sources.is_empty());
    }

    #[test]
    fn test_find_sources() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        tests::create_test_files(temp_path);

        let selector = EdgeSelector;
        assert_eq!(selector.name(), SourceType::Edge);

        let res = selector.find_sources(temp_path);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_dirs = res.unwrap();
        assert_eq!(bookmark_dirs.len(), 2);
        assert!(bookmark_dirs.contains(&temp_path.join(".config/microsoft-edge/Default/Bookmarks")));
        assert!(
            bookmark_dirs.contains(&temp_path.join(".config/microsoft-edge/Profile 1/Bookmarks"))
        );
    }
}
