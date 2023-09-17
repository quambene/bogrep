use crate::{bookmark_reader::SourceReader, utils, Config, SourceBookmarks, TargetBookmarks};
use log::{info, trace};
use std::io::{Read, Write};

/// Import bookmarks from the configured source files and store unique bookmarks
/// in cache.
pub fn import(config: &Config) -> Result<(), anyhow::Error> {
    let source_reader = config
        .settings
        .sources
        .iter()
        .map(SourceReader::new)
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    let target_bookmark_file = utils::open_file_in_write_mode(&config.target_bookmark_file)?;

    import_bookmarks(source_reader, target_bookmark_file)?;

    Ok(())
}

fn import_bookmarks(
    mut source_reader: Vec<SourceReader>,
    mut target_reader_writer: impl Read + Write,
) -> Result<(), anyhow::Error> {
    let source_bookmarks = SourceBookmarks::read(source_reader.as_mut())?;
    let mut target_bookmarks = TargetBookmarks::read(&mut target_reader_writer)?;

    target_bookmarks.update(source_bookmarks)?;
    target_bookmarks.write(&mut target_reader_writer)?;

    log_import(&source_reader, &target_bookmarks);

    Ok(())
}

fn log_import(source_reader: &[SourceReader], target_bookmarks: &TargetBookmarks) {
    let source = if source_reader.len() == 1 {
        "source"
    } else {
        "sources"
    };

    info!(
        "Imported {} bookmarks from {} {source}: {}",
        target_bookmarks.bookmarks.len(),
        source_reader.len(),
        source_reader
            .iter()
            .map(|source_reader| source_reader.source().path.to_string_lossy())
            .collect::<Vec<_>>()
            .join(", ")
    );
    trace!("Imported bookmarks: {target_bookmarks:#?}");
}
