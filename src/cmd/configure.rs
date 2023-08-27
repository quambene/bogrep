use crate::{Config, ConfigArgs, SourceFile};
use log::info;
use std::path::PathBuf;

/// Configure the source files to import the bookmarks.
pub fn configure(mut config: Config, args: ConfigArgs) -> Result<(), anyhow::Error> {
    if config.verbosity >= 1 {
        info!("{args:?}");
    }

    let settings_path = config.settings_path;

    if let Some(source) = args.set_source.source {
        let source = PathBuf::from(source);
        let source_file = SourceFile::new(source, args.set_source.folders);
        config.settings.source_bookmark_files.push(source_file);
        config.settings.write(&settings_path)?;
    }

    if let Some(set_cache_mode) = args.set_cache_mode {
        config.settings.cache_mode = set_cache_mode;
        config.settings.write(&settings_path)?;
    }

    Ok(())
}
