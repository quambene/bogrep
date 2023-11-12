use crate::{
    bookmark_reader::{SourceReader, TargetReaderWriter},
    utils, Config, SourceBookmarks, TargetBookmarks,
};
use log::{info, trace};
use std::io::{Read, Seek, Write};

/// Import bookmarks from the configured source files and store unique bookmarks
/// in cache.
pub fn import(config: &Config) -> Result<(), anyhow::Error> {
    let source_reader = config
        .settings
        .sources
        .iter()
        .map(|source| SourceReader::init(source))
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    let target_bookmark_file = utils::open_file_in_read_write_mode(&config.target_bookmark_file)?;

    import_bookmarks(source_reader, target_bookmark_file)?;

    Ok(())
}

fn import_bookmarks(
    mut source_reader: Vec<SourceReader>,
    target_reader_writer: impl Read + Write + Seek,
) -> Result<(), anyhow::Error> {
    let mut source_bookmarks = SourceBookmarks::new();

    for reader in &mut source_reader {
        reader.read_and_parse(&mut source_bookmarks)?;
    }

    let mut target_bookmarks = TargetBookmarks::default();
    let mut target_reader_writer = TargetReaderWriter::new(target_reader_writer);
    target_reader_writer.read(&mut target_bookmarks)?;
    target_bookmarks.update(source_bookmarks)?;
    target_reader_writer.write(&target_bookmarks)?;

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
    use crate::{json, test_utils, SimpleBookmarkReader, Source};
    use std::{collections::HashSet, io::Cursor, path::Path};

    fn test_import_bookmarks(source: &Source, expected_bookmarks: HashSet<String>) {
        let target_bookmarks = TargetBookmarks::default();
        let target_bookmarks = json::serialize(&target_bookmarks).unwrap();

        let mut cursor = Cursor::new(Vec::new());
        cursor.write_all(&target_bookmarks).unwrap();
        // Set cursor position to the start again to prepare cursor for reading.
        cursor.set_position(0);

        let source_reader = SourceReader::init(&source).unwrap();
        let res = import_bookmarks(vec![source_reader], &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let actual = cursor.into_inner();
        let actual_bookmarks = json::deserialize::<TargetBookmarks>(&actual).unwrap();
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
        let source_path = Path::new("test_data/bookmarks_firefox.json");
        let source_folders = vec![];
        let source = Source::new(source_path, source_folders);
        let expected_bookmarks = HashSet::from_iter([
            String::from("https://www.mozilla.org/en-US/firefox/central/"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html")
        ]);

        test_import_bookmarks(&source, expected_bookmarks);
    }

    #[test]
    fn test_import_bookmarks_firefox_compressed() {
        let source_path = Path::new("test_data/bookmarks_firefox.jsonlz4");
        let source_folders = vec![];
        let source = Source::new(source_path, source_folders);
        let expected_bookmarks = HashSet::from_iter([
            String::from("https://www.mozilla.org/en-US/firefox/central/"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html")
        ]);
        test_utils::create_compressed_bookmarks(source_path);

        test_import_bookmarks(&source, expected_bookmarks);
    }

    #[test]
    fn test_import_bookmarks_chrome() {
        let source_path = Path::new("test_data/bookmarks_chrome.json");
        let source_folders = vec![];
        let source = Source::new(source_path, source_folders);
        let expected_bookmarks = HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]);

        test_import_bookmarks(&source, expected_bookmarks);
    }

    #[test]
    fn test_import_bookmarks_chrome_no_extension() {
        let source_path = Path::new("test_data/bookmarks_chrome_no_extension");
        let source_folders = vec![];
        let source = Source::new(source_path, source_folders);
        let expected_bookmarks = HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]);

        test_import_bookmarks(&source, expected_bookmarks);
    }

    #[test]
    fn test_import_bookmarks_simple() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        let source_folders = vec![];
        let source = Source::new(source_path, source_folders);
        let expected_bookmarks = HashSet::from_iter([
            "https://www.deepl.com/translator".to_owned(),
            "https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"
                .to_owned(),
            "https://en.wikipedia.org/wiki/Design_Patterns".to_owned(),
            "https://doc.rust-lang.org/book/title-page.html".to_owned(),
        ]);
        test_import_bookmarks(&source, expected_bookmarks);
    }
}
