use crate::{
    bookmark_reader::{SourceOs, SourceReader},
    bookmarks::RawSource,
    errors::BogrepError,
    json,
    settings::SettingsArgs,
    utils, Config, ConfigArgs, Settings,
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

    let source_path = args
        .set_source
        .source
        .as_ref()
        .map(|source_path| fs::canonicalize(source_path).context("Invalid source path"))
        .transpose()?;
    let source_folders = &args.set_source.folders;
    let source = source_path.map(|source_path| RawSource::new(source_path, source_folders.clone()));

    if let Some(ref source) = source {
        // Validate source file
        SourceReader::init(source)?;
    }

    let settings_file = utils::open_and_truncate_file(&config.settings_path)?;

    let settings_args = SettingsArgs::new(
        source,
        args.set_ignored_urls.ignore,
        args.set_underlying_urls.underlying,
        args.set_cache_mode.cache_mode,
        args.set_max_concurrent_requests.max_concurrent_requests,
        args.set_request_timeout.request_timeout,
        args.set_request_throttling.request_throttling,
        args.set_max_idle_connections_per_host
            .max_idle_connections_per_host,
        args.set_idle_connections_timeout.idle_connections_timeout,
    );

    configure_settings(&mut config.settings, &settings_args, settings_file)?;

    Ok(())
}

fn configure_settings(
    settings: &mut Settings,
    settings_args: &SettingsArgs,
    mut writer: impl Write,
) -> Result<(), anyhow::Error> {
    if let Some(source) = settings_args.source.as_ref() {
        settings.set_source(source.clone())?;
    }

    if let Some(request_timeout) = settings_args.request_timeout {
        settings.set_request_timeout(request_timeout);
    }

    if let Some(request_throttling) = settings_args.request_throttling {
        settings.set_request_throttling(request_throttling);
    }

    if let Some(request_timeout) = settings_args.max_concurrent_requests {
        settings.set_max_concurrent_requests(request_timeout);
    }

    if let Some(request_timeout) = settings_args.max_idle_connections_per_host {
        settings.set_max_idle_connections_per_host(request_timeout);
    }

    if let Some(request_timeout) = settings_args.idle_connections_timeout {
        settings.set_idle_connections_timeout(request_timeout);
    }

    settings.set_cache_mode(settings_args.cache_mode.clone());

    for ignored_url in settings_args.ignored_urls.as_slice() {
        if let Err(err) = settings.add_ignored_url(&ignored_url) {
            warn!("{err}");
        }
    }

    for underlying_url in settings_args.underlying_urls.as_slice() {
        if let Err(err) = settings.add_underlying_url(&underlying_url) {
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

    println!("Found sources:");
    for (index, source) in sources.iter().enumerate() {
        println!("{}: {}", index + 1, source.path.display());
    }

    println!("Select sources: yes (y), no (n), or specify numbers separated by whitespaces");

    let mut selected_sources = configure_source_path(&sources)?;

    if selected_sources.is_empty() {
        return Ok(());
    }

    println!("Select bookmark folders: yes (y), no (n), or specify folder names separated by whitespaces");

    for source in selected_sources.iter_mut() {
        println!("Select folders for source: {}", source.path.display());

        let source_folders = configure_source_folders()?;

        if let Some(folders) = source_folders {
            println!("Selected folders: {folders:?}");
            source.folders = folders;
            config.settings.sources.push(source.to_owned());
        } else {
            config.settings.sources.clear();
            println!("No folders selected. Aborting ...");
            break;
        }
    }

    Ok(())
}

fn configure_source_path(sources: &[RawSource]) -> Result<Vec<RawSource>, anyhow::Error> {
    let indexed_sources = sources
        .iter()
        .enumerate()
        .map(|(i, _)| i + 1)
        .collect::<Vec<_>>();

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
        .into_iter()
        .filter_map(|i| sources.get(i - 1).cloned())
        .collect::<Vec<_>>();

    Ok(selected_sources)
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

fn configure_source_folders() -> Result<Option<Vec<String>>, anyhow::Error> {
    let selected_folders = loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        match select_source_folders_from_input(&input) {
            Ok(selected_fodlers) => {
                break selected_fodlers;
            }
            Err(_) => {
                println!("Invalid input. Please try again");
                continue;
            }
        }
    };

    Ok(selected_folders)
}

fn select_source_folders_from_input(input: &str) -> Result<Option<Vec<String>>, BogrepError> {
    let choices: Vec<&str> = input.split_whitespace().collect();

    if choices.is_empty() {
        Err(BogrepError::InvalidInput)
    } else if choices.len() == 1 {
        match choices[0] {
            "y" | "yes" => Ok(Some(vec![])),
            "n" | "no" => Ok(None),
            choice => {
                if choice.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(vec![choice.trim().to_owned()]))
                }
            }
        }
    } else {
        Ok(Some(
            choices
                .into_iter()
                .map(|folder| folder.trim().to_owned())
                .collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::CacheMode;

    use super::*;
    use std::{io::Cursor, path::PathBuf};

    #[test]
    fn test_select_source_folders_from_input() {
        let res = select_source_folders_from_input("");
        assert!(res.is_err());

        let res = select_source_folders_from_input(" ");
        assert!(res.is_err());

        let selected_folders = select_source_folders_from_input("dev").unwrap();
        assert_eq!(selected_folders, Some(vec!["dev".to_owned(),]));

        let selected_folders = select_source_folders_from_input("dev science").unwrap();
        assert_eq!(
            selected_folders,
            Some(vec!["dev".to_owned(), "science".to_owned()])
        );
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

    #[test]
    fn test_configure_source() {
        let mut cursor = Cursor::new(Vec::new());
        let mut settings = Settings::default();
        let mut settings_args = SettingsArgs::default();
        settings_args.source = Some(RawSource {
            path: PathBuf::from("test_data/bookmarks_simple.txt"),
            folders: vec!["dev".to_string(), "articles".to_string()],
        });

        let res = configure_settings(&mut settings, &settings_args, &mut cursor);
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
        let mut settings_args = SettingsArgs::default();
        settings_args.cache_mode = Some(CacheMode::Html);

        let res = configure_settings(&mut settings, &settings_args, &mut cursor);
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
        let mut settings_args = SettingsArgs::default();
        settings_args.ignored_urls = vec![
            "https://url1.com".to_string(),
            "https://url2.com".to_string(),
        ];
        settings_args.underlying_urls = vec!["https://news.ycombinator.com".to_string()];

        let res = configure_settings(&mut settings, &settings_args, &mut cursor);
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
        let mut settings_args = SettingsArgs::default();
        settings_args.ignored_urls = vec![
            "https://url1.com".to_string(),
            "https://url2.com".to_string(),
        ];
        settings_args.underlying_urls = vec!["https://news.ycombinator.com".to_string()];

        let res = configure_settings(&mut settings, &settings_args, &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let res = configure_settings(&mut settings, &settings_args, &mut cursor);
        assert!(res.is_ok(), "{}", res.unwrap_err());
    }
}
