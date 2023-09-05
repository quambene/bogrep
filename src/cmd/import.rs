use crate::{
    json, utils, BookmarkReader, ChromeBookmarkReader, Config, FirefoxBookmarkReader,
    SimpleBookmarkReader, SourceBookmarks, SourceFile, TargetBookmarks,
};
use anyhow::{anyhow, Context};
use log::{debug, info, trace};
use std::io::{Read, Write};

/// Import bookmarks from the configured source files and store unique bookmarks
/// in `bookmarks.json`.
pub fn import(config: &Config) -> Result<(), anyhow::Error> {
    let source_bookmarks_files = config
        .settings
        .source_bookmark_files
        .iter()
        .map(|source_file| utils::open_file(&source_file.source))
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    let target_bookmarks_files = utils::open_file(&config.target_bookmark_file)?;

    // TODO: impl BookmarkReader::find_bookmark_file() -> fs::File and use as writer.

    import_bookmarks(
        config.verbosity,
        &config.settings.source_bookmark_files,
        source_bookmarks_files,
        target_bookmarks_files,
    )?;

    Ok(())
}

fn import_bookmarks(
    verbosity: u8,
    source_bookmark_files: &[SourceFile],
    mut source_reader: Vec<impl Read>,
    mut target_reader_writer: impl Read + Write,
) -> Result<(), anyhow::Error> {
    let source_bookmarks = read_source_bookmarks(verbosity, source_bookmark_files, source_reader)?;
    let mut target_bookmarks = read_target_bookmarks(&mut target_reader_writer)?;

    target_bookmarks.update(source_bookmarks)?;

    write_target_bookmarks(&mut target_reader_writer, &target_bookmarks)?;

    log_import(source_bookmark_files, &target_bookmarks);

    Ok(())
}

fn read_source_bookmarks(
    verbosity: u8,
    source_bookmark_files: &[SourceFile],
    mut source_reader: Vec<impl Read>,
) -> Result<SourceBookmarks, anyhow::Error> {
    let mut bookmarks = SourceBookmarks::new();

    for bookmark_file in source_bookmark_files {
        debug!(
            "Read bookmarks from file '{}'",
            bookmark_file.source.display()
        );

        if verbosity >= 1 {
            info!(
                "Read bookmarks from file '{}'",
                bookmark_file.source.display()
            );
        }

        let path_str = bookmark_file.source.to_str().unwrap_or("");

        if path_str.contains("firefox") {
            let firefox_reader = FirefoxBookmarkReader;
            firefox_reader.read_and_parse(bookmark_file, &mut bookmarks)?;
        } else if path_str.contains("google-chrome") {
            let chrome_reader = ChromeBookmarkReader;
            chrome_reader.read_and_parse(bookmark_file, &mut bookmarks)?;
        } else if bookmark_file.source.extension().map(|path| path.to_str()) == Some(Some("txt")) {
            let simple_reader = SimpleBookmarkReader;
            simple_reader.read_and_parse(bookmark_file, &mut bookmarks)?;
        } else {
            return Err(anyhow!(
                "Format not supported for bookmark file '{}'",
                bookmark_file.source.display()
            ));
        }
    }

    Ok(bookmarks)
}

fn read_target_bookmarks(
    mut target_reader_writer: impl Read + Write,
) -> Result<TargetBookmarks, anyhow::Error> {
    let mut buf = String::new();
    target_reader_writer
        .read_to_string(&mut buf)
        .context("Can't read from `bookmarks.json` file:")?;
    let target_bookmarks = json::deserialize::<TargetBookmarks>(&buf)?;
    Ok(target_bookmarks)
}

fn write_target_bookmarks(
    mut target_reader_writer: impl Read + Write,
    target_bookmarks: &TargetBookmarks,
) -> Result<(), anyhow::Error> {
    let bookmarks_json = json::serialize(target_bookmarks)?;
    target_reader_writer
        .write_all(&bookmarks_json)
        .context("Can't write to `bookmarks.json` file")?;
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
