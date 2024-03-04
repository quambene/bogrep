use crate::{
    args::ImportArgs,
    bookmark_reader::{ReadTarget, SourceOs, SourceReader, TargetReaderWriter, WriteTarget},
    bookmarks::{BookmarkManager, RunConfig, RunMode},
    errors::BogrepError,
    json, utils, BookmarkProcessor, Cache, CacheMode, Caching, Client, Config, Fetch,
    ProcessReport, Settings,
};
use anyhow::anyhow;
use chrono::Utc;
use log::debug;
use std::{
    collections::HashSet,
    io::{self, Write},
    path::Path,
};
use url::Url;

/// Import bookmarks from the configured source files and store unique bookmarks
/// in cache.
pub async fn import(config: Config, args: ImportArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    let source_os = match std::env::consts::OS {
        "linux" => Some(SourceOs::Linux),
        "macos" => Some(SourceOs::Macos),
        "windows" => Some(SourceOs::Windows),
        _ => None,
    };
    debug!("Source OS: {:?}", source_os);
    let mut config = config;
    let home_dir = dirs::home_dir().ok_or(anyhow!("Missing home dir"))?;

    if args.dry_run {
        println!("Running in dry mode ...")
    }

    if config.settings.sources.is_empty() {
        if let Some(source_os) = source_os {
            configure_sources(&mut config, &home_dir, &source_os)?;

            if !args.dry_run {
                let mut settings_file = utils::open_and_truncate_file(&config.settings_path)?;
                let settings_json = json::serialize(config.settings.clone())?;
                settings_file.write_all(&settings_json)?;
            }
        }
    }

    let cache_mode = CacheMode::new(&None, &config.settings.cache_mode);
    let cache = Cache::new(&config.cache_path, cache_mode);
    let client = Client::new(&config)?;
    let ignored_urls = config
        .settings
        .ignored_urls
        .iter()
        .map(|url| Url::parse(url))
        .collect::<Result<Vec<_>, _>>()?;
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
    let run_mode = if args.dry_run {
        RunMode::DryRun
    } else {
        RunMode::Import
    };
    let run_config = RunConfig::new(run_mode, cache.is_empty(), ignored_urls);

    import_and_process_bookmarks(
        &config.settings,
        run_config,
        client,
        cache,
        &mut source_readers,
        &mut target_reader_writer.reader(),
        &mut target_reader_writer.writer(),
    )
    .await?;

    target_reader_writer.close()?;

    Ok(())
}

pub async fn import_and_process_bookmarks(
    settings: &Settings,
    run_config: RunConfig,
    client: impl Fetch,
    cache: impl Caching,
    source_readers: &mut [SourceReader],
    target_reader: &mut impl ReadTarget,
    target_writer: &mut impl WriteTarget,
) -> Result<(), anyhow::Error> {
    let now = Utc::now();
    let mut bookmark_manager = BookmarkManager::new(run_config);
    let report = ProcessReport::init(bookmark_manager.is_dry_run());
    let bookmark_processor = BookmarkProcessor::new(client, cache, settings.to_owned(), report);

    target_reader.read(&mut bookmark_manager.target_bookmarks_mut())?;

    bookmark_manager.import(source_readers)?;
    bookmark_manager.add_bookmarks(now)?;
    bookmark_manager.remove_bookmarks();
    bookmark_manager.set_actions(now);

    bookmark_processor
        .process_bookmarks(
            bookmark_manager
                .target_bookmarks_mut()
                .values_mut()
                .collect(),
        )
        .await?;

    bookmark_manager.print_report(&source_readers);
    bookmark_manager.finish();

    target_writer.write(bookmark_manager.target_bookmarks())?;

    Ok(())
}

/// Configure sources if no sources are configured.
fn configure_sources(
    config: &mut Config,
    home_dir: &Path,
    source_os: &SourceOs,
) -> Result<(), anyhow::Error> {
    let sources = SourceReader::select_sources(home_dir, source_os)?;
    let indexed_sources = sources
        .iter()
        .enumerate()
        .map(|(i, _)| i + 1)
        .collect::<Vec<_>>();

    println!("Found sources:");
    for (index, source) in sources.iter().enumerate() {
        println!("{}: {}", index + 1, source.path.display());
    }

    println!("Select sources: yes (y), no (n), or specify numbers separated by whitespaces");

    let selected_indices = loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        match select_sources_from_input(&input, &indexed_sources) {
            Ok(selected_sources) => {
                break selected_sources;
            }
            Err(_) => {
                println!("Invalid input. Please try again");
                continue;
            }
        }
    };

    if selected_indices.is_empty() {
        println!("No sources selected. Aborting ...");
    } else {
        println!(
            "Selected sources: {}",
            selected_indices
                .iter()
                .map(|num| num.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        );
    }

    let selected_sources = selected_indices
        .iter()
        .filter_map(|&i| sources.get(i - 1))
        .collect::<Vec<_>>();

    for source in selected_sources {
        config.settings.sources.push(source.to_owned());
    }

    Ok(())
}

fn select_sources_from_input(
    input: &str,
    indexed_sources: &[usize],
) -> Result<Vec<usize>, BogrepError> {
    let choices: Vec<&str> = input.split_whitespace().collect();

    if choices.len() == 1 {
        match choices[0] {
            "y" | "yes" => Ok(indexed_sources.to_vec()),
            "n" | "no" => Ok(vec![]),
            num => {
                let num = num
                    .parse::<usize>()
                    .map_err(|_| BogrepError::InvalidInput)?;

                if indexed_sources.contains(&num) {
                    Ok(vec![num])
                } else {
                    Err(BogrepError::InvalidInput)
                }
            }
        }
    } else {
        let nums: Result<Vec<usize>, _> = choices.iter().map(|s| s.parse::<usize>()).collect();
        if let Ok(nums) = nums {
            // Remove duplicates
            let mut nums = nums
                .into_iter()
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            nums.sort();

            if nums.iter().all(|num| indexed_sources.contains(num)) {
                Ok(nums)
            } else {
                Err(BogrepError::InvalidInput)
            }
        } else {
            Err(BogrepError::InvalidInput)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bookmark_reader::{ReadSource, TextReader},
        bookmarks::{RawSource, RunMode},
        json, test_utils, JsonBookmarks, MockCache, MockClient, Source, SourceType,
    };
    use std::{
        collections::HashSet,
        io::{Cursor, Write},
        path::Path,
    };

    async fn test_import_source(
        sources: &[RawSource],
        expected_bookmarks: HashSet<String>,
        dry_run: bool,
    ) {
        let settings = Settings::default();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Html);
        let bookmarks_json = JsonBookmarks::default();
        let buf = json::serialize(&bookmarks_json).unwrap();

        let mut target_reader = Cursor::new(Vec::new());
        target_reader.write_all(&buf).unwrap();
        // Set cursor position to the start again to prepare cursor for reading.
        target_reader.set_position(0);
        let mut target_writer = Cursor::new(Vec::new());
        let ignored_urls = vec![];
        let mut source_readers = sources
            .iter()
            .map(|source| SourceReader::init(source).unwrap())
            .collect::<Vec<_>>();
        let run_mode = if dry_run {
            RunMode::DryRun
        } else {
            RunMode::Import
        };
        let config = RunConfig::new(run_mode, cache.is_empty(), ignored_urls);

        let res = import_and_process_bookmarks(
            &settings,
            config,
            client,
            cache,
            &mut source_readers,
            &mut target_reader,
            &mut target_writer,
        )
        .await;
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

    async fn test_import_source_bookmarks(
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

        let settings = Settings::default();
        let client = MockClient::new();
        let cache = MockCache::new(CacheMode::Html);
        let source_reader = SourceReader::new(
            source.clone(),
            Box::new(source_reader_writer.clone()),
            source_reader,
        );
        let config = RunConfig::new(RunMode::Import, cache.is_empty(), vec![]);

        let res = import_and_process_bookmarks(
            &settings,
            config,
            client,
            cache,
            &mut [source_reader],
            target_reader,
            target_writer,
        )
        .await;

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

    #[tokio::test]
    async fn test_import_source_empty() {
        let source_path = Path::new("test_data/bookmarks_firefox.json");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let expected_bookmarks = HashSet::from_iter([
            String::from("https://www.mozilla.org/en-US/firefox/central/"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html")
        ]);

        test_import_source(&[source], expected_bookmarks, false).await;
    }

    #[tokio::test]
    async fn test_import_source_firefox() {
        let source_path = Path::new("test_data/bookmarks_firefox.json");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let expected_bookmarks = HashSet::from_iter([
            String::from("https://www.mozilla.org/en-US/firefox/central/"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html")
        ]);

        test_import_source(&[source], expected_bookmarks, false).await;
    }

    #[tokio::test]
    async fn test_import_source_firefox_compressed() {
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

        test_import_source(&[source], expected_bookmarks, false).await;
    }

    #[tokio::test]
    async fn test_import_source_chrome() {
        let source_path = Path::new("test_data/bookmarks_chromium.json");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let expected_bookmarks = HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]);

        test_import_source(&[source], expected_bookmarks, false).await;
    }

    #[tokio::test]
    async fn test_import_source_chromium_no_extension() {
        let source_path = Path::new("test_data/bookmarks_chromium_no_extension");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        let expected_bookmarks = HashSet::from_iter([
            String::from("https://www.deepl.com/translator"),
            String::from("https://www.quantamagazine.org/how-mathematical-curves-power-cryptography-20220919/"),
            String::from("https://en.wikipedia.org/wiki/Design_Patterns"),
            String::from("https://doc.rust-lang.org/book/title-page.html"),
        ]);

        test_import_source(&[source], expected_bookmarks, false).await;
    }

    #[tokio::test]
    async fn test_import_source_simple() {
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
        test_import_source(&[source], expected_bookmarks, false).await;
    }

    #[tokio::test]
    async fn test_import_source_simple_dry_run() {
        let source_path = Path::new("test_data/bookmarks_simple.txt");
        let source_folders = vec![];
        let source = RawSource::new(source_path, source_folders);
        // We are expecting no bookmarks in a dry run.
        let expected_bookmarks = HashSet::new();
        test_import_source(&[source], expected_bookmarks, true).await;
    }

    #[tokio::test]
    async fn test_import_bookmarks_simple_add_source_bookmarks() {
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
        )
        .await;

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
        )
        .await;
    }

    #[tokio::test]
    async fn test_import_bookmarks_simple_delete_source_bookmarks() {
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
        )
        .await;

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
        )
        .await;
    }

    #[test]
    fn test_select_sources_from_input() {
        let indexed_sources = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let selected_sources = select_sources_from_input("y", &indexed_sources).unwrap();
        assert_eq!(selected_sources, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        let selected_sources = select_sources_from_input("yes", &indexed_sources).unwrap();
        assert_eq!(selected_sources, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        let selected_sources = select_sources_from_input("n", &indexed_sources).unwrap();
        assert_eq!(selected_sources, vec![] as Vec<usize>);

        let selected_sources = select_sources_from_input("no", &indexed_sources).unwrap();
        assert_eq!(selected_sources, vec![] as Vec<usize>);

        let selected_sources = select_sources_from_input("1", &indexed_sources).unwrap();
        assert_eq!(selected_sources, vec![1]);

        let selected_sources = select_sources_from_input("1 5 10", &indexed_sources).unwrap();
        assert_eq!(selected_sources, vec![1, 5, 10]);

        let selected_sources = select_sources_from_input("1 1", &indexed_sources).unwrap();
        assert_eq!(selected_sources, vec![1]);

        let selected_sources = select_sources_from_input("1 5 1 10", &indexed_sources).unwrap();
        assert_eq!(selected_sources, vec![1, 5, 10]);

        let selected_sources = select_sources_from_input("1 5 1 10 0", &indexed_sources);
        assert!(selected_sources.is_err());

        let selected_sources = select_sources_from_input("x", &indexed_sources);
        assert!(selected_sources.is_err());

        let selected_sources = select_sources_from_input("x ", &indexed_sources);
        assert!(selected_sources.is_err());

        let selected_sources = select_sources_from_input(" x", &indexed_sources);
        assert!(selected_sources.is_err());

        let selected_sources = select_sources_from_input("xx", &indexed_sources);
        assert!(selected_sources.is_err());

        let selected_sources = select_sources_from_input("x x", &indexed_sources);
        assert!(selected_sources.is_err());

        let selected_sources = select_sources_from_input("0", &indexed_sources);
        assert!(selected_sources.is_err());

        let selected_sources = select_sources_from_input("11", &indexed_sources);
        assert!(selected_sources.is_err());

        let selected_sources = select_sources_from_input("1 5 11", &indexed_sources);
        assert!(selected_sources.is_err());
    }
}
