use crate::cache::CacheMode;
use clap::{ArgAction, Args as ClapArgs, Parser, Subcommand};

/// Describes the available arguments in the CLI.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    pub pattern: Option<String>,
    #[arg(short, long, action = ArgAction::Count)]
    pub verbose: u8,
    /// Cache the fetched bookmarks as HTML or markdown file.
    #[arg(short, long, value_enum)]
    pub mode: Option<CacheMode>,
    #[command(subcommand)]
    pub subcommands: Option<Subcommands>,
}

/// Describes the available subcommands in the CLI.
#[derive(Subcommand, Debug)]
pub enum Subcommands {
    /// Configure the source files to import the bookmarks.
    Config(ConfigArgs),
    /// Import bookmarks, fetch bookmarks from url, and save fetched website in cache.
    Init(InitArgs),
    /// Determine diff of source and target bookmarks. Fetch and cache websites
    /// for new bookmarks; delete cache for removed bookmarks.
    Update(UpdateArgs),
    /// Import bookmarks from the configured source files.
    Import,
    /// Fetch and cache bookmarks.
    Fetch(FetchArgs),
    /// Clean up cache for removed bookmarks.
    Clean(CleanArgs),
}

/// Describes the arguments for the `config` subcommand.
#[derive(ClapArgs, Debug)]
pub struct ConfigArgs {
    #[command(flatten)]
    pub set_source: SetSource,
    #[command(flatten)]
    pub set_cache_mode: SetCacheMode,
    #[arg(short, long, num_args = 0.., value_delimiter = ' ')]
    pub urls: Vec<String>,
}

#[derive(ClapArgs, Debug)]
#[group(required = false, multiple = true)]
pub struct SetSource {
    /// The path of the bookmark file to be imported.
    #[arg(long)]
    pub source: Option<String>,
    /// The bookmark folders to be imported.
    ///
    /// Multiple folders are separated by a comma.
    #[arg(long, num_args = 0.., value_delimiter = ',')]
    pub folders: Vec<String>,
}

#[derive(ClapArgs, Debug)]
#[group(required = false)]
pub struct SetCacheMode {
    #[arg(long)]
    pub cache_mode: Option<CacheMode>,
}

/// Describes the arguments for the `fetch` subcommand.
#[derive(ClapArgs, Debug)]
pub struct FetchArgs {
    /// Fetch all bookmarks.
    ///
    /// If flag is not set, bookmarks are only fetched if a bookmark is not
    /// cached yet. Otherwise, the cached content will be updated with
    /// the fetched content.
    #[arg(short, long)]
    pub all: bool,
    /// Cache the fetched bookmarks as text, HTML or markdown file.
    #[arg(short, long, value_enum)]
    pub mode: Option<CacheMode>,
    /// Get the difference between the fetched and cached
    /// bookmark for the given urls.
    ///
    /// Multiple urls are separated by a whitespace.
    #[arg(short, long, value_name = "URLs", num_args = 0.., value_delimiter = ' ')]
    pub diff: Vec<String>,
}

/// Describes the arguments for the `init` subcommand.
#[derive(ClapArgs, Debug)]
pub struct InitArgs {
    /// Cache the fetched bookmarks as text, HTML or markdown file.
    #[arg(short, long, value_enum)]
    pub mode: Option<CacheMode>,
}

/// Describes the arguments for the `update` subcommand.
#[derive(ClapArgs, Debug)]
pub struct UpdateArgs {
    /// Cache the fetched bookmarks as text, HTML or markdown file.
    #[arg(short, long, value_enum)]
    pub mode: Option<CacheMode>,
}

/// Describes the arguments for the `clean` subcommand.
#[derive(ClapArgs, Debug)]
pub struct CleanArgs {
    /// Clean cache for all file extensions (.txt, .md, .html).
    #[arg(short, long)]
    pub all: bool,
    /// Cache the fetched bookmarks as text, HTML or markdown file.
    #[arg(short, long, value_enum)]
    pub mode: Option<CacheMode>,
}
