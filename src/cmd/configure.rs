use crate::{
    bookmark_reader::{SourceOs, SourceReader},
    bookmarks::RawSource,
    cache::CacheMode,
    errors::BogrepError,
    json, utils, Config, ConfigArgs, Settings,
};
use anyhow::{anyhow, Context};
use log::{debug, warn};
use std::{
    collections::HashSet,
    fs,
    io::{self, Write},
    path::Path,
};

/// Configure the source files to import the bookmarks, the cache mode, or the
/// ignoure urls .
pub fn configure(config: Config, args: ConfigArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    if args.dry_run {
        println!("Running in dry mode ...")
    }

    let mut config = config;
    let home_dir = dirs::home_dir().ok_or(anyhow!("Missing home dir"))?;

    if config.settings.sources.is_empty() {
        if let Some(source_os) = utils::get_supported_os() {
            configure_sources(&mut config, &home_dir, &source_os)?;

            if !args.dry_run {
                let mut settings_file = utils::open_and_truncate_file(&config.settings_path)?;
                let settings_json = json::serialize(config.settings.clone())?;
                settings_file.write_all(&settings_json)?;
            }
        }
    }

    let cache_mode = args.set_cache_mode.cache_mode;
    let source_path = args
        .set_source
        .source
        .map(|source_path| fs::canonicalize(source_path).context("Invalid source path"))
        .transpose()?;
    let source_folders = args.set_source.folders;
    let source = source_path.map(|source_path| RawSource::new(source_path, source_folders));

    if let Some(ref source) = source {
        // Validate source file
        SourceReader::init(source)?;
    }

    let settings_file = utils::open_and_truncate_file(&config.settings_path)?;
    let mut settings = config.settings;

    configure_settings(
        &mut settings,
        source,
        cache_mode,
        &args.set_ignored_urls.ignore,
        &args.set_underlying_urls.underlying,
        settings_file,
    )?;

    Ok(())
}

fn configure_settings(
    settings: &mut Settings,
    source: Option<RawSource>,
    cache_mode: Option<CacheMode>,
    ignored_urls: &[String],
    underlying_urls: &[String],
    mut writer: impl Write,
) -> Result<(), anyhow::Error> {
    if let Some(source) = source {
        settings.set_source(source)?;
    }

    settings.set_cache_mode(cache_mode);

    for ignored_url in ignored_urls {
        if let Err(err) = settings.add_ignored_url(ignored_url) {
            warn!("{err}");
        }
    }

    for underlying_url in underlying_urls {
        if let Err(err) = settings.add_underlying_url(underlying_url) {
            warn!("{err}");
        };
    }

    let settings_json = json::serialize(settings)?;
    writer.write_all(&settings_json)?;

    Ok(())
}

/// Configure sources if no sources are configured.
pub fn configure_sources(
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
    use std::{io::Cursor, path::PathBuf};

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

    #[test]
    fn test_configure_source() {
        let mut cursor = Cursor::new(Vec::new());
        let mut settings = Settings::default();
        let source = RawSource {
            path: PathBuf::from("test_data/bookmarks_simple.txt"),
            folders: vec!["dev".to_string(), "articles".to_string()],
        };
        let ignored_urls = vec![];
        let underlying_urls = vec![];
        let cache_mode = None;

        let res = configure_settings(
            &mut settings,
            Some(source),
            cache_mode,
            &ignored_urls,
            &underlying_urls,
            &mut cursor,
        );
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let actual_settings = String::from_utf8(cursor.into_inner()).unwrap();
        let expected_settings = r#"{
    "sources": [
        {
            "source": "test_data/bookmarks_simple.txt",
            "folders": [
                "dev",
                "articles"
            ]
        }
    ],
    "ignored_urls": [],
    "underlying_urls": [],
    "cache_mode": "text",
    "max_concurrent_requests": 100,
    "request_timeout": 60000,
    "request_throttling": 3000,
    "max_idle_connections_per_host": 10,
    "idle_connections_timeout": 5000
}"#;
        assert_eq!(actual_settings, expected_settings);
    }

    #[test]
    fn test_configure_cache_mode() {
        let mut cursor = Cursor::new(Vec::new());
        let mut settings = Settings::default();
        let ignored_urls = vec![];
        let underlying_urls = vec![];
        let cache_mode = Some(CacheMode::Html);
        let res = configure_settings(
            &mut settings,
            None,
            cache_mode,
            &ignored_urls,
            &underlying_urls,
            &mut cursor,
        );
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let actual_settings = String::from_utf8(cursor.into_inner()).unwrap();
        let expected_settings = r#"{
    "sources": [],
    "ignored_urls": [],
    "underlying_urls": [],
    "cache_mode": "html",
    "max_concurrent_requests": 100,
    "request_timeout": 60000,
    "request_throttling": 3000,
    "max_idle_connections_per_host": 10,
    "idle_connections_timeout": 5000
}"#;
        assert_eq!(actual_settings, expected_settings);
    }

    #[test]
    fn test_configure_urls() {
        let mut cursor = Cursor::new(Vec::new());
        let mut settings = Settings::default();
        let ignored_urls = vec![
            "https://url1.com".to_string(),
            "https://url2.com".to_string(),
        ];
        let underlying_urls = vec!["https://news.ycombinator.com".to_string()];
        let res = configure_settings(
            &mut settings,
            None,
            None,
            &ignored_urls,
            &underlying_urls,
            &mut cursor,
        );
        assert!(res.is_ok(), "{}", res.unwrap_err());
        let actual_settings = String::from_utf8(cursor.into_inner()).unwrap();

        let expected_settings = r#"{
    "sources": [],
    "ignored_urls": [
        "https://url1.com/",
        "https://url2.com/"
    ],
    "underlying_urls": [
        "https://news.ycombinator.com/"
    ],
    "cache_mode": "text",
    "max_concurrent_requests": 100,
    "request_timeout": 60000,
    "request_throttling": 3000,
    "max_idle_connections_per_host": 10,
    "idle_connections_timeout": 5000
}"#;
        assert_eq!(actual_settings, expected_settings);
    }

    #[test]
    fn test_configure_urls_twice() {
        let mut cursor = Cursor::new(Vec::new());
        let mut settings = Settings::default();
        let ignored_urls = vec![
            "https://url1.com".to_string(),
            "https://url2.com".to_string(),
        ];
        let underlying_urls = vec!["https://news.ycombinator.com".to_string()];

        let res = configure_settings(
            &mut settings,
            None,
            None,
            &ignored_urls,
            &underlying_urls,
            &mut cursor,
        );
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let res = configure_settings(
            &mut settings,
            None,
            None,
            &ignored_urls,
            &underlying_urls,
            &mut cursor,
        );
        assert!(res.is_ok(), "{}", res.unwrap_err());
    }
}
