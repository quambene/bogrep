use crate::{
    args::ImportArgs,
    bookmark_reader::{ReadTarget, SourceReader, TargetReaderWriter, WriteTarget},
    Action, Config, SourceBookmarks, TargetBookmarks,
};
use log::{debug, trace};
use url::Url;

/// Import bookmarks from the configured source files and store unique bookmarks
/// in cache.
pub fn import(config: &Config, args: ImportArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    let source_reader = config
        .settings
        .sources
        .iter()
        .map(SourceReader::init)
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    let target_reader_writer = TargetReaderWriter::new(
        &config.target_bookmark_file,
        &config.target_bookmark_lock_file,
    )?;
    let ignored_urls = config
        .settings
        .ignored_urls
        .iter()
        .map(|url| Url::parse(url))
        .collect::<Result<Vec<_>, _>>()?;

    import_source(
        source_reader,
        &mut target_reader_writer.reader(),
        &mut target_reader_writer.writer(),
        &ignored_urls,
    )?;

    target_reader_writer.close()?;

    Ok(())
}

fn import_source(
    mut source_reader: Vec<SourceReader>,
    target_reader: &mut impl ReadTarget,
    target_writer: &mut impl WriteTarget,
    ignored_urls: &[Url],
) -> Result<(), anyhow::Error> {
    let mut source_bookmarks = SourceBookmarks::default();

    for reader in &mut source_reader {
        reader.read_and_parse(&mut source_bookmarks)?;
    }

    let mut target_bookmarks = TargetBookmarks::default();
    target_reader.read(&mut target_bookmarks)?;

    target_bookmarks.update(&source_bookmarks)?;
    target_bookmarks.ignore_urls(ignored_urls);
    target_bookmarks.clean_up();

    target_writer.write(&target_bookmarks)?;

    log_import(&source_reader, &target_bookmarks);

    Ok(())
}

fn log_import(source_reader: &[SourceReader], target_bookmarks: &TargetBookmarks) {
    let source = if source_reader.len() == 1 {
        "source"
    } else {
        "sources"
    };

    println!(
        "Imported {} bookmarks from {} {source}: {}",
        target_bookmarks
            .values()
            .filter(|bookmark| bookmark.action == Action::FetchAndReplace
                || bookmark.action == Action::FetchAndAdd)
            .collect::<Vec<_>>()
            .len(),
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
        json, test_utils, JsonBookmarks, ReadBookmark, SimpleBookmarkReader, SourceType,
    };
    use std::{
        collections::HashSet,
        io::{Cursor, Write},
        path::Path,
    };

    fn test_import_source(source: &RawSource, expected_bookmarks: HashSet<String>) {
        let bookmarks_json = JsonBookmarks::default();
        let buf = json::serialize(&bookmarks_json).unwrap();

        let mut target_reader = Cursor::new(Vec::new());
        target_reader.write_all(&buf).unwrap();
        // Set cursor position to the start again to prepare cursor for reading.
        target_reader.set_position(0);
        let mut target_writer = Cursor::new(Vec::new());
        let ignored_urls = vec![];

        let source_reader = SourceReader::init(source).unwrap();
        let res = import_source(
            vec![source_reader],
            &mut target_reader,
            &mut target_writer,
            &ignored_urls,
        );
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let actual = target_writer.into_inner();
        let actual_bookmarks = json::deserialize::<JsonBookmarks>(&actual).unwrap();
        assert!(actual_bookmarks
            .iter()
            .all(|bookmark| bookmark.last_cached.is_none()));
        assert_eq!(
            actual_bookmarks
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
        let ignored_urls = vec![];

        let res = import_source(
            vec![source_reader],
            target_reader,
            target_writer,
            &ignored_urls,
        );

        let actual = target_writer.get_ref();
        let actual_bookmarks = json::deserialize::<JsonBookmarks>(actual);
        assert!(
            actual_bookmarks.is_ok(),
            "{}\n{}",
            actual_bookmarks.unwrap_err(),
            String::from_utf8(actual.to_owned()).unwrap()
        );

        let actual_bookmarks = actual_bookmarks.unwrap();
        assert!(actual_bookmarks
            .iter()
            .all(|bookmark| bookmark.last_cached.is_none()));
        assert_eq!(
            actual_bookmarks
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

        let bookmarks_json = JsonBookmarks::default();
        let buf = json::serialize(&bookmarks_json).unwrap();

        let mut target_reader = Cursor::new(Vec::new());
        target_reader.write_all(&buf).unwrap();
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

        let bookmarks_json = JsonBookmarks::default();
        let buf = json::serialize(&bookmarks_json).unwrap();

        let mut target_reader = Cursor::new(Vec::new());
        target_reader.write_all(&buf).unwrap();
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
}
