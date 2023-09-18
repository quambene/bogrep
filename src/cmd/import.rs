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
    let mut target_bookmark_file =
        utils::open_file_in_read_write_mode(&config.target_bookmark_file)?;

    import_bookmarks(source_reader, &mut target_bookmark_file)?;

    Ok(())
}

fn import_bookmarks(
    mut source_reader: Vec<SourceReader>,
    target_reader_writer: &mut (impl Read + Write),
) -> Result<(), anyhow::Error> {
    let source_bookmarks = SourceBookmarks::read(source_reader.as_mut())?;
    let mut target_bookmarks = TargetBookmarks::read(target_reader_writer)?;

    target_bookmarks.update(source_bookmarks)?;
    target_bookmarks.write(target_reader_writer)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{json, test_utils, Source};
    use std::{collections::HashSet, io::Cursor, path::Path};

    fn test_import_bookmarks(source: &Source, expected_bookmarks: HashSet<String>) {
        let target_bookmarks = TargetBookmarks::default();
        let target_bookmarks = json::serialize(&target_bookmarks).unwrap();

        let mut cursor = Cursor::new(Vec::new());
        cursor.write_all(&target_bookmarks).unwrap();
        let curser_after_read = cursor.position() as usize;
        // Set cursor position to the start again to prepare cursor for reading.
        cursor.set_position(0);

        let source_reader = SourceReader::new(&source).unwrap();
        let res = import_bookmarks(vec![source_reader], &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let cursor_after_write = cursor.position() as usize;

        let actual =
            String::from_utf8(cursor.get_ref()[curser_after_read..cursor_after_write].to_vec())
                .unwrap();
        println!("actual: {actual}");
        let actual_bookmarks = json::deserialize::<TargetBookmarks>(actual.as_bytes()).unwrap();
        assert!(actual_bookmarks
            .bookmarks
            .iter()
            .all(|bookmark| bookmark.last_cached == None));
        assert_eq!(
            actual_bookmarks
                .bookmarks
                .iter()
                .map(|bookmark| bookmark.url.clone())
                .collect::<HashSet<_>>(),
            expected_bookmarks,
        );
    }

    #[test]
    fn test_import_bookmarks_firefox() {
        let source_path = Path::new("test_data/source/bookmarks_firefox.jsonlz4");
        let source_folders = vec![];
        let source = Source::new(source_path, source_folders);
        let target_bookmarks = HashSet::from_iter([
            String::from("https://www.mozilla.org/en-US/firefox/central/"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html")
        ]);
        test_utils::create_compressed_bookmarks(source_path);

        test_import_bookmarks(&source, target_bookmarks);
    }

    #[test]
    fn test_import_bookmarks_google_chrome() {
        let source_path = Path::new("test_data/source/bookmarks_google-chrome.json");
        let source_folders = vec![];
        let source = Source::new(source_path, source_folders);
        let target_bookmarks = HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]);

        test_import_bookmarks(&source, target_bookmarks);
    }

    #[test]
    fn test_import_bookmarks_simple() {
        let source_path = Path::new("test_data/source/bookmarks_simple.txt");
        let source_folders = vec![];
        let source = Source::new(source_path, source_folders);
        let target_bookmarks = HashSet::from_iter([
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://www.quantamagazine.org/how-galois-groups-used-polynomial-symmetries-to-reshape-math-20210803/"),
            String::from("https://www.quantamagazine.org/computing-expert-says-programmers-need-more-math-20220517/"),
        ]);

        test_import_bookmarks(&source, target_bookmarks);
    }
}
