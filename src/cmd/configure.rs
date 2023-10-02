use crate::{
    args::{SetCacheMode, SetSource},
    json, utils, Config, ConfigArgs, Settings,
};
use log::info;
use std::io::Write;

/// Configure the source files to import the bookmarks.
pub fn configure(config: Config, args: ConfigArgs) -> Result<(), anyhow::Error> {
    if config.verbosity >= 1 {
        info!("{args:?}");
    }

    let settings_file = utils::open_file_in_read_write_mode(&config.settings_path)?;

    configure_settings(
        config.settings,
        args.set_source,
        args.set_cache_mode,
        settings_file,
    )?;

    Ok(())
}

fn configure_settings(
    settings: Settings,
    set_source: SetSource,
    set_cache_mode: SetCacheMode,
    mut writer: impl Write,
) -> Result<(), anyhow::Error> {
    let mut settings = settings;
    let settings_read = settings.clone();

    settings.configure(set_source, set_cache_mode);

    if settings_read != settings {
        let settings_json = json::serialize(settings)?;
        writer.write_all(&settings_json)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::{Cursor, Read},
    };

    #[test]
    fn test_configure_settings() {
        let mut cursor = Cursor::new(Vec::new());
        let settings = Settings::default();
        let set_source = SetSource {
            source: Some(String::from("path/to/bookmarks")),
            folders: vec![],
        };
        let set_cache_mode = SetCacheMode { cache_mode: None };
        let res = configure_settings(settings, set_source, set_cache_mode, &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let actual_settings = String::from_utf8(cursor.into_inner()).unwrap();

        let mut expected_settings = String::new();
        let mut expected_file = File::open("test_data/configure/settings.json").unwrap();
        expected_file
            .read_to_string(&mut expected_settings)
            .unwrap();
        assert_eq!(actual_settings, expected_settings);
    }
}
