use crate::{
    args::ImportArgs,
    bookmark_reader::{SourceOs, SourceReader, TargetReaderWriter},
    bookmarks::{BookmarkManager, BookmarkService, RunMode, ServiceConfig},
    client::ClientConfig,
    errors::BogrepError,
    json, utils, Cache, CacheMode, Client, Config,
};
use anyhow::anyhow;
use chrono::Utc;
use log::debug;
use std::{
    collections::HashSet,
    io::{self, Write},
    path::Path,
};

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
    let client_config = ClientConfig::new(&config.settings);
    let client = Client::new(&client_config)?;
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

    let now = Utc::now();
    let run_mode = if args.dry_run {
        RunMode::DryRun
    } else {
        RunMode::Import
    };
    let service_config = ServiceConfig::new(
        run_mode,
        &config.settings.ignored_urls,
        config.settings.max_concurrent_requests,
    )?;
    let mut bookmark_manager = BookmarkManager::default();
    let bookmark_service = BookmarkService::new(service_config, client, cache);

    bookmark_service
        .run(
            &mut bookmark_manager,
            &mut source_readers,
            &mut target_reader_writer.reader(),
            &mut target_reader_writer.writer(),
            now,
        )
        .await?;

    target_reader_writer.close()?;

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
