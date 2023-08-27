use crate::{Config, SourceBookmarks, SourceFile, TargetBookmarks};
use log::{info, trace};

/// Import bookmarks from the configured source files and store unique bookmarks
/// in `bookmarks.json`.
pub fn import(config: &Config) -> Result<(), anyhow::Error> {
    let mut source_bookmarks = SourceBookmarks::new();
    let mut target_bookmarks;
    source_bookmarks.read(config)?;

    if config.target_bookmark_file.exists() {
        target_bookmarks = TargetBookmarks::read(config)?;
        target_bookmarks.diff(&source_bookmarks, config)?;
    } else {
        target_bookmarks = TargetBookmarks::from(source_bookmarks);
        target_bookmarks.write(config)?;
    };

    log_import(&config.settings.source_bookmark_files, &target_bookmarks);

    Ok(())
}

fn log_import(source_bookmark_files: &[SourceFile], target_bookmarks: &TargetBookmarks) {
    let source = if source_bookmark_files.len() == 1 {
        "source"
    } else {
        "sources"
    };

    info!(
        "Imported {} bookmarks from {} {source}: {}",
        target_bookmarks.bookmarks.len(),
        source_bookmark_files.len(),
        source_bookmark_files
            .iter()
            .map(|bookmark_file| bookmark_file.source.to_string_lossy())
            .collect::<Vec<_>>()
            .join(", ")
    );
    trace!("Imported bookmarks: {target_bookmarks:#?}");
}
