use super::{chromium::ChromiumSelector, SelectSource, SourceOs};
use crate::SourceType;
use anyhow::anyhow;
use std::path::{Path, PathBuf};

pub struct EdgeSelector;

impl EdgeSelector {
    pub fn new() -> Box<Self> {
        Box::new(EdgeSelector)
    }
}

impl SelectSource for EdgeSelector {
    fn name(&self) -> SourceType {
        SourceType::Chrome
    }

    fn source_os(&self) -> SourceOs {
        SourceOs::Linux
    }

    fn extension(&self) -> Option<&str> {
        Some("json")
    }

    fn find_dir(&self, home_dir: &Path) -> Result<Vec<PathBuf>, anyhow::Error> {
        let browser_dirs = [
            // apt package
            home_dir.join(".config/microsoft-edge"),
        ];

        let bookmark_dirs = ChromiumSelector::find_profile_dirs(&browser_dirs);

        Ok(bookmark_dirs)
    }

    fn find_file(&self, source_dir: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        let path_str = source_dir
            .to_str()
            .ok_or(anyhow!("Invalid path: source path contains invalid UTF-8"))?;

        if path_str.contains("microsoft-edge") {
            let bookmark_path = source_dir.join("Bookmarks");
            Ok(Some(bookmark_path))
        } else {
            Err(anyhow!(
                "Unexpected format for source directory: {}",
                source_dir.display()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_find_dir() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

        fs::create_dir_all(temp_path.join(".config/microsoft-edge/Default")).unwrap();
        fs::create_dir_all(temp_path.join(".config/microsoft-edge/Profile 1")).unwrap();

        let selector = EdgeSelector;
        let res = selector.find_dir(temp_path);
        assert!(res.is_ok(), "Can't find dir: {}", res.unwrap_err());

        let bookmark_dirs = res.unwrap();
        assert_eq!(bookmark_dirs.len(), 2);
        assert!(bookmark_dirs.contains(&temp_path.join(".config/microsoft-edge/Default")));
        assert!(bookmark_dirs.contains(&temp_path.join(".config/microsoft-edge/Profile 1")));
    }
}
