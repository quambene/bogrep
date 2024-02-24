use crate::{
    args::ImportArgs,
    bookmark_reader::{ReadTarget, SourceReader, TargetReaderWriter, WriteTarget},
    bookmarks::RawSource,
    json, utils, Config, SourceBookmarks, TargetBookmarks,
};
use anyhow::anyhow;
use log::debug;
use std::io::{self, Write};
use url::Url;

/// Import bookmarks from the configured source files and store unique bookmarks
/// in cache.
pub fn import(config: Config, args: ImportArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    let mut config = config;

    if config.settings.sources.is_empty() {
        let home_dir = dirs::home_dir().ok_or(anyhow!("Missing home dir"))?;
        let sources = SourceReader::select_sources(&home_dir)?;

        log_sources(&sources);

        println!("Select sources: yes (y) or no (n)");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input == "n" {
            println!("Aborting ...");
            return Ok(());
        } else if input == "y" {
            println!(
                "Selected sources: {}",
                (1..=sources.len())
                    .map(|num| num.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            );
        } else {
            println!("Aborting ...");
            return Ok(());
        }

        for source in sources {
            config.settings.sources.push(source);
        }

        let mut settings_file = utils::open_and_truncate_file(&config.settings_path)?;
        let settings_json = json::serialize(config.settings.clone())?;
        settings_file.write_all(&settings_json)?;
    }

    let mut source_readers = config
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
        &mut source_readers,
        &mut target_reader_writer.reader(),
        &mut target_reader_writer.writer(),
        &ignored_urls,
    )?;

    target_reader_writer.close()?;

    Ok(())
}

fn import_source(
    source_readers: &mut [SourceReader],
    target_reader: &mut impl ReadTarget,
    target_writer: &mut impl WriteTarget,
    ignored_urls: &[Url],
) -> Result<(), anyhow::Error> {
    let mut source_bookmarks = SourceBookmarks::default();

    for source_reader in source_readers.iter_mut() {
        source_reader.import(&mut source_bookmarks)?;
    }

    let mut target_bookmarks = TargetBookmarks::default();
    target_reader.read(&mut target_bookmarks)?;

    target_bookmarks.update(&source_bookmarks)?;
    target_bookmarks.ignore_urls(ignored_urls);
    target_bookmarks.clean_up();

    target_writer.write(&target_bookmarks)?;

    utils::log_import(source_readers, &target_bookmarks);

    Ok(())
}

fn log_sources(sources: &[RawSource]) {
    println!("Found sources:");

    for (index, source) in sources.iter().enumerate() {
        println!("{}: {}", index + 1, source.path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bookmark_reader::{ReadSource, TextReader},
        bookmarks::RawSource,
        json, test_utils, JsonBookmarks, Source, SourceType,
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
            &mut [source_reader],
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
        source_reader: Box<dyn ReadSource>,
        source_reader_writer: &mut Cursor<Vec<u8>>,
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
            source_reader,
        );
        let ignored_urls = vec![];

        let res = import_source(
            &mut [source_reader],
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
        test_utils::create_compressed_json_file(source_path).unwrap();

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
        let source = Source::new(SourceType::Unknown, source_path, source_folders);
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
            Box::new(TextReader),
            &mut source_reader_writer,
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
            Box::new(TextReader),
            &mut source_reader_writer,
            &mut target_reader,
            &mut target_writer,
        );
    }

    #[test]
    fn test_import_bookmarks_simple_delete_source_bookmarks() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        let source_folders = vec![];
        let source = Source::new(SourceType::Unknown, source_path, source_folders);
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
            Box::new(TextReader),
            &mut source_reader_writer,
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
            Box::new(TextReader),
            &mut source_reader_writer,
            &mut target_reader,
            &mut target_writer,
        );
    }
}
