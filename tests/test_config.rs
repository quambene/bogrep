use bogrep::{Config, Settings};
use std::env;
use tempfile::tempdir;

#[test]
#[cfg_attr(not(feature = "integration-test"), ignore)]
fn test_config() {
    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    assert!(temp_path.exists(), "Missing path: {}", temp_path.display());

    env::set_var("BOGREP_HOME", temp_path);

    let res = Config::init();
    assert!(
        res.is_ok(),
        "Can't init config for BOGREP_HOME={}: {}",
        temp_path.display(),
        res.unwrap_err()
    );

    let config = res.unwrap();
    assert_eq!(
        config,
        Config {
            settings_path: temp_path.join("settings.json"),
            cache_path: temp_path.join("cache"),
            target_bookmark_file: temp_path.join("bookmarks.json"),
            target_bookmark_lock_file: temp_path.join("bookmarks-lock.json"),
            settings: Settings::default()
        }
    );
}
