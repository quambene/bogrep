use crate::{Config, ConfigArgs};
use log::info;

/// Configure the source files to import the bookmarks.
pub fn configure(config: Config, args: ConfigArgs) -> Result<(), anyhow::Error> {
    if config.verbosity >= 1 {
        info!("{args:?}");
    }

    let mut settings = config.settings;
    let settings_read = settings.clone();

    settings.configure(args.set_source, args.set_cache_mode);

    if settings_read != settings {
        settings.write(&config.settings_path)?;
    }

    Ok(())
}
