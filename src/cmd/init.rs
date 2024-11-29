use crate::{
    bookmark_reader::{SourceOs, SourceReader},
    bookmarks::RawSource,
    errors::BogrepError,
    utils::{self},
    Config, Settings,
};
use anyhow::anyhow;
use std::{
    collections::HashSet,
    io::{self},
    path::Path,
};

pub fn init(mut config: Config) -> Result<(), anyhow::Error> {
    let home_dir = dirs::home_dir().ok_or(anyhow!("Missing home dir"))?;

    if config.settings.sources.is_empty() {
        if let Some(source_os) = utils::get_supported_os() {
            init_sources(&mut config.settings, &home_dir, &source_os)?;
            utils::write_settings(&config.settings_path, &config.settings)?;
        }
    } else {
        println!("Bookmark sources already configured");
    }

    Ok(())
}

/// Initialize sources if no sources are configured.
pub fn init_sources(
    settings: &mut Settings,
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
            settings.sources.push(source.to_owned());
        } else {
            settings.sources.clear();
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
    use super::*;

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
}
