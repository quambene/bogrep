use crate::{
    args::ImportArgs,
    bookmark_reader::{ReadTarget, SourceReader, WriteTarget},
    utils, Config, SourceBookmarks, TargetBookmark, TargetBookmarks,
};
use chrono::Utc;
use log::{info, trace};
use std::collections::HashSet;

/// Import bookmarks from the configured source files and store unique bookmarks
/// in cache.
pub fn import(config: &Config, args: ImportArgs) -> Result<(), anyhow::Error> {
    let source_reader = config
        .settings
        .sources
        .iter()
        .map(SourceReader::init)
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    let mut target_reader = utils::open_file_in_read_mode(&config.target_bookmark_file)?;
    let mut target_writer = utils::open_and_truncate_file(&config.target_bookmark_lock_file)?;

    if args.urls.is_empty() {
        import_source(source_reader, &mut target_reader, &mut target_writer)?;
    } else {
        import_urls(&args.urls, &mut target_reader, &mut target_writer)?;
    }

    utils::close_and_rename(
        (target_writer, &config.target_bookmark_lock_file),
        (target_reader, &config.target_bookmark_file),
    )?;

    Ok(())
}

fn import_source(
    mut source_reader: Vec<SourceReader>,
    target_reader: &mut impl ReadTarget,
    target_writer: &mut impl WriteTarget,
) -> Result<(), anyhow::Error> {
    let mut source_bookmarks = SourceBookmarks::default();

    for reader in &mut source_reader {
        reader.read_and_parse(&mut source_bookmarks)?;
    }

    let mut target_bookmarks = TargetBookmarks::default();

    target_reader.read(&mut target_bookmarks)?;
    target_bookmarks.update(&source_bookmarks)?;
    target_writer.write(&target_bookmarks)?;

    log_import(&source_reader, &target_bookmarks);

    Ok(())
}

fn import_urls(
    urls: &[String],
    target_reader: &mut impl ReadTarget,
    target_writer: &mut impl WriteTarget,
) -> Result<(), anyhow::Error> {
    let now = Utc::now();
    let mut target_bookmarks = TargetBookmarks::default();

    target_reader.read(&mut target_bookmarks)?;

    for url in urls {
        let bookmark = TargetBookmark::new(url, now, None, HashSet::new());
        target_bookmarks.add(&bookmark);
    }

    target_writer.write(&target_bookmarks)?;

    info!("Imported {} bookmarks", urls.len());

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
            .map(|source_reader| source_reader.source().path.clone())
            .collect::<Vec<_>>()
            .join(", ")
    );
    trace!("Imported bookmarks: {target_bookmarks:#?}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bookmarks::{RawSource, Source},
        json, test_utils, ReadBookmark, SimpleBookmarkReader, SourceType,
    };
    use std::{
        collections::HashSet,
        io::{Cursor, Write},
        path::Path,
    };

    fn test_import_source(source: &RawSource, expected_bookmarks: HashSet<String>) {
        let target_bookmarks = TargetBookmarks::default();
        let target_bookmarks = json::serialize(&target_bookmarks).unwrap();

        let mut target_reader = Cursor::new(Vec::new());
        target_reader.write_all(&target_bookmarks).unwrap();
        // Set cursor position to the start again to prepare cursor for reading.
        target_reader.set_position(0);
        let mut target_writer = Cursor::new(Vec::new());

        let source_reader = SourceReader::init(&source).unwrap();
        let res = import_source(vec![source_reader], &mut target_reader, &mut target_writer);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let actual = target_writer.into_inner();
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

    fn test_import_source_bookmarks(
        source_bookmarks: HashSet<String>,
        source: &Source,
        source_reader_writer: &mut Cursor<Vec<u8>>,
        bookmark_reader: impl ReadBookmark + 'static,
        target_reader: &mut Cursor<Vec<u8>>,
        target_writer: &mut Cursor<Vec<u8>>,
    ) {
        for bookmark in source_bookmarks.iter() {
            writeln!(source_reader_writer, "{}", bookmark).unwrap();
        }
        source_reader_writer.set_position(0);

        let source_reader = SourceReader::new(
            source.clone(),
            Box::new(source_reader_writer.clone()),
            Box::new(bookmark_reader),
        );

        let res = import_source(vec![source_reader], target_reader, target_writer);

        let actual = target_writer.get_ref();
        let actual_bookmarks = json::deserialize::<TargetBookmarks>(actual);
        assert!(
            actual_bookmarks.is_ok(),
            "{}\n{}",
            actual_bookmarks.unwrap_err(),
            String::from_utf8(actual.to_owned()).unwrap()
        );

        let actual_bookmarks = actual_bookmarks.unwrap();
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
            source_bookmarks,
        );

        assert!(res.is_ok(), "{}", res.unwrap_err());
    }

    #[test]
    fn test_import_source_firefox() {
        let source_path = Path::new("test_data/bookmarks_firefox.json");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let expected_bookmarks = HashSet::from_iter([
            String::from("https://www.mozilla.org/en-US/firefox/central/"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html")
        ]);

        test_import_source(&source, expected_bookmarks);
    }

    #[test]
    fn test_import_source_firefox_compressed() {
        let source_path = Path::new("test_data/bookmarks_firefox.jsonlz4");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let expected_bookmarks = HashSet::from_iter([
            String::from("https://www.mozilla.org/en-US/firefox/central/"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html")
        ]);
        test_utils::create_compressed_bookmarks(source_path);

        test_import_source(&source, expected_bookmarks);
    }

    #[test]
    fn test_import_source_chrome() {
        let source_path = Path::new("test_data/bookmarks_chromium.json");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let expected_bookmarks = HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]);

        test_import_source(&source, expected_bookmarks);
    }

    #[test]
    fn test_import_source_chromium_no_extension() {
        let source_path = Path::new("test_data/bookmarks_chromium_no_extension");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let expected_bookmarks = HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]);

        test_import_source(&source, expected_bookmarks);
    }

    #[test]
    fn test_import_source_simple() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let expected_bookmarks = HashSet::from_iter([
            "https://www.deepl.com/translator".to_owned(),
            "https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"
                .to_owned(),
            "https://en.wikipedia.org/wiki/Design_Patterns".to_owned(),
            "https://doc.rust-lang.org/book/title-page.html".to_owned(),
        ]);
        test_import_source(&source, expected_bookmarks);
    }

    #[test]
    fn test_import_bookmarks_simple_add_source_bookmarks() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        let source_folders = vec![];
        let source = Source::new(SourceType::Simple, source_path, source_folders);
        let mut source_reader_writer = Cursor::new(Vec::new());
        let source_bookmarks =
            HashSet::from_iter(["https://doc.rust-lang.org/book/title-page.html".to_owned()]);

        let target_bookmarks = TargetBookmarks::default();
        let target_bookmarks = json::serialize(&target_bookmarks).unwrap();

        let mut target_reader = Cursor::new(Vec::new());
        target_reader.write_all(&target_bookmarks).unwrap();
        // Set cursor position to the start again to prepare cursor for reading.
        target_reader.set_position(0);
        let mut target_writer = Cursor::new(Vec::new());

        test_import_source_bookmarks(
            source_bookmarks,
            &source,
            &mut source_reader_writer,
            SimpleBookmarkReader,
            &mut target_reader,
            &mut target_writer,
        );

        // Clean up source and simulate change of source bookmarks.
        source_reader_writer.get_mut().clear();
        let source_bookmarks = HashSet::from_iter([
            "https://www.deepl.com/translator".to_owned(),
            "https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"
                .to_owned(),
            "https://en.wikipedia.org/wiki/Design_Patterns".to_owned(),
            "https://doc.rust-lang.org/book/title-page.html".to_owned(),
        ]);

        test_import_source_bookmarks(
            source_bookmarks,
            &source,
            &mut source_reader_writer,
            SimpleBookmarkReader,
            &mut target_reader,
            &mut target_writer,
        );
    }

    #[test]
    fn test_import_bookmarks_simple_delete_source_bookmarks() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        let source_folders = vec![];
        let source = Source::new(SourceType::Simple, source_path, source_folders);
        let mut source_reader_writer = Cursor::new(Vec::new());
        let source_bookmarks = HashSet::from_iter([
            "https://www.deepl.com/translator".to_owned(),
            "https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"
                .to_owned(),
            "https://en.wikipedia.org/wiki/Design_Patterns".to_owned(),
            "https://doc.rust-lang.org/book/title-page.html".to_owned(),
        ]);

        let target_bookmarks = TargetBookmarks::default();
        let target_bookmarks = json::serialize(&target_bookmarks).unwrap();

        let mut target_reader = Cursor::new(Vec::new());
        target_reader.write_all(&target_bookmarks).unwrap();
        // Set cursor position to the start again to prepare cursor for reading.
        target_reader.set_position(0);
        let mut target_writer = Cursor::new(Vec::new());

        test_import_source_bookmarks(
            source_bookmarks,
            &source,
            &mut source_reader_writer,
            SimpleBookmarkReader,
            &mut target_reader,
            &mut target_writer,
        );

        // Clean up source and simulate change of source bookmarks.
        source_reader_writer.get_mut().clear();
        let source_bookmarks =
            HashSet::from_iter(["https://doc.rust-lang.org/book/title-page.html".to_owned()]);
        // Clean up target to prepare cursor for writing.
        target_writer.get_mut().clear();

        test_import_source_bookmarks(
            source_bookmarks,
            &source,
            &mut source_reader_writer,
            SimpleBookmarkReader,
            &mut target_reader,
            &mut target_writer,
        );
    }

    #[test]
    fn test_import_urls() {
        let mut expected_urls = HashSet::new();
        expected_urls.insert("https://test_url1.com".to_owned());
        expected_urls.insert("https://test_url2.com".to_owned());
        expected_urls.insert("https://test_url3.com".to_owned());

        let mut target_bookmarks = TargetBookmarks::default();
        target_bookmarks.add(&TargetBookmark::new(
            "https://test_url1.com".to_owned(),
            Utc::now(),
            None,
            HashSet::new(),
        ));
        let target_bookmarks = json::serialize(&target_bookmarks).unwrap();

        let mut target_reader = Cursor::new(Vec::new());
        target_reader.write_all(&target_bookmarks).unwrap();
        // Set cursor position to the start again to prepare cursor for reading.
        target_reader.set_position(0);
        let mut target_writer = Cursor::new(Vec::new());

        let urls = vec![
            "https://test_url2.com".to_owned(),
            "https://test_url3.com".to_owned(),
        ];

        let res = import_urls(&urls, &mut target_reader, &mut target_writer);
        assert!(res.is_ok());

        let actual = target_writer.get_ref();
        let actual_bookmarks = json::deserialize::<TargetBookmarks>(actual);
        assert!(
            actual_bookmarks.is_ok(),
            "{}\n{}",
            actual_bookmarks.unwrap_err(),
            String::from_utf8(actual.to_owned()).unwrap()
        );

        let actual_bookmarks = actual_bookmarks.unwrap();
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
            expected_urls,
        );
    }
}
