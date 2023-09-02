use crate::{cache::CacheMode, Cache, Config, TargetBookmarks};
use anyhow::{anyhow, Context};
use colored::Colorize;
use log::info;
use regex::Regex;
use std::{
    fs::File,
    io::{self, BufRead},
    path::Path,
};

pub fn search(
    pattern: String,
    config: &Config,
    cache_mode: &Option<CacheMode>,
) -> Result<(), anyhow::Error> {
    if config.verbosity >= 1 {
        info!("{pattern:?}");
    }

    let target_bookmarks = TargetBookmarks::read(config)?;

    if target_bookmarks.bookmarks.is_empty() {
        Err(anyhow!("Missing bookmarks, run `bogrep update` first"))
    } else {
        search_bookmarks(pattern, &target_bookmarks, &config.cache_path, cache_mode)?;
        Ok(())
    }
}

#[allow(clippy::comparison_chain)]
fn search_bookmarks(
    pattern: String,
    bookmarks: &TargetBookmarks,
    cache_path: &Path,
    cache_mode: &Option<CacheMode>,
) -> Result<(), anyhow::Error> {
    let max_columns = 1000;
    let re = format!("(?i){pattern}");
    let regex = Regex::new(&re)?;
    let cache = Cache::new(cache_path, cache_mode)?;

    for bookmark in &bookmarks.bookmarks {
        let cache_path = cache.get_path(bookmark);

        if cache_path.exists() {
            let cache_file = File::open(&cache_path).context(format!(
                "Can't open cached file at {}",
                cache_path.display()
            ))?;
            let reader = io::BufReader::new(cache_file);

            let mut matched_lines = vec![];

            for line in reader.lines() {
                let start_index;
                let end_index;
                let line = line?;

                if regex.is_match(&line) {
                    if line.len() >= max_columns {
                        let first_match =
                            regex.find(&line).ok_or(anyhow!("Can't find first match"))?;
                        let match_start = first_match.start();
                        let match_end = first_match.end();
                        let half_max = max_columns / 2;
                        start_index = match_start.saturating_sub(half_max);
                        end_index = (match_end + half_max).min(line.len());
                    } else {
                        start_index = 0;
                        end_index = line.len();
                    }

                    let truncated_line = &line[start_index..end_index];
                    let colored_line = regex
                        .replace_all(truncated_line, |caps: &regex::Captures| {
                            caps[0].bold().red().to_string()
                        });

                    matched_lines.push(colored_line.into_owned());
                }
            }

            if matched_lines.len() == 1 {
                println!("Match in bookmark: {}", bookmark.url.blue());
            } else if matched_lines.len() > 1 {
                println!("Matches in bookmark: {}", bookmark.url.blue());
            }

            for line in &matched_lines {
                println!("{line}");
            }
        }
    }

    Ok(())
}
