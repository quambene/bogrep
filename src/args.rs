use crate::cache::CacheMode;
use clap::{ArgAction, Args as ClapArgs, Parser, Subcommand};

/// Describes the available arguments in the CLI.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// The search term.
    pub pattern: Option<String>,
    #[arg(short, long, action = ArgAction::Count)]
    pub verbose: u8,
    /// Search the cached bookmarks in HTML or plaintext format.
    #[arg(short, long, value_enum)]
    pub mode: Option<CacheMode>,
    /// Ignore case distinctions in patterns.
    #[arg(short = 'i', long)]
    pub ignore_case: bool,
    /// Print only URLs of bookmarks with selected lines.
    #[arg(short = 'l', long)]
    pub files_with_matches: bool,
    /// Match only whole words.
    #[arg(short = 'w', long)]
    pub word_regexp: bool,
    #[command(subcommand)]
    pub subcommands: Option<Subcommands>,
}

/// Describes the available subcommands in the CLI.
#[derive(Subcommand, Debug)]
pub enum Subcommands {
    /// Configure the source files to import the bookmarks.
    Config(ConfigArgs),
    /// Synchronize source and target bookmarks. Fetch and cache websites for
    /// new bookmarks; delete cache for removed bookmarks.
    Sync(SyncArgs),
    /// Import bookmarks from the configured source files.
    Import(ImportArgs),
    /// Fetch and cache bookmarks.
    Fetch(FetchArgs),
    /// Clean up cache for removed bookmarks.
    Clean(CleanArgs),
    /// Add a bookmark.
    Add(AddArgs),
    /// Remove a bookmark.
    Remove(RemoveArgs),
}

/// Describes the arguments for the `config` subcommand.
#[derive(ClapArgs, Debug)]
pub struct ConfigArgs {
    /// Run command in dry mode.
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,
    #[command(flatten)]
    pub set_source: SetSource,
    #[command(flatten)]
    pub set_cache_mode: SetCacheMode,
    #[command(flatten)]
    pub set_ignored_urls: SetIgnoredUrls,
    #[command(flatten)]
    pub set_underlying_urls: SetUnderlyingUrls,
    #[command(flatten)]
    pub set_request_timeout: SetRequestTimeout,
    #[command(flatten)]
    pub set_request_throttling: SetRequestThrottling,
    #[command(flatten)]
    pub set_max_concurrent_requests: SetMaxConcurrentRequests,
    #[command(flatten)]
    pub set_max_idle_connections_per_host: SetMaxIdleConnectionsPerHost,
    #[command(flatten)]
    pub set_idle_connections_timeout: SetIdleConnectionsTimeout,
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
    /// Cache the fetched bookmarks as text, HTML or markdown file.
    #[arg(long)]
    pub cache_mode: Option<CacheMode>,
}

#[derive(ClapArgs, Debug)]
#[group(required = false)]
pub struct SetIgnoredUrls {
    #[arg(long, value_name = "URLs", num_args = 0.., value_delimiter = ' ')]
    pub ignore: Vec<String>,
}

#[derive(ClapArgs, Debug)]
#[group(required = false)]
pub struct SetUnderlyingUrls {
    #[arg(long, value_name = "URLs", num_args = 0.., value_delimiter = ' ')]
    pub underlying: Vec<String>,
}

#[derive(ClapArgs, Debug)]
#[group(required = false)]
pub struct SetRequestTimeout {
    #[arg(long)]
    pub request_timeout: Option<u64>,
}

#[derive(ClapArgs, Debug)]
#[group(required = false)]
pub struct SetRequestThrottling {
    #[arg(long)]
    pub request_throttling: Option<u64>,
}

#[derive(ClapArgs, Debug)]
#[group(required = false)]
pub struct SetMaxConcurrentRequests {
    #[arg(long)]
    pub max_concurrent_requests: Option<usize>,
}

#[derive(ClapArgs, Debug)]
#[group(required = false)]
pub struct SetMaxIdleConnectionsPerHost {
    #[arg(long)]
    pub max_idle_connections_per_host: Option<usize>,
}

#[derive(ClapArgs, Debug)]
#[group(required = false)]
pub struct SetIdleConnectionsTimeout {
    #[arg(long)]
    pub idle_connections_timeout: Option<u64>,
}

/// Describes the arguments for the `import` subcommand.
#[derive(ClapArgs, Debug)]
pub struct ImportArgs {
    /// Run command in dry mode.
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,
}

/// Describes the arguments for the `fetch` subcommand.
#[derive(ClapArgs, Debug)]
pub struct FetchArgs {
    /// Fetch and replace bookmarks.
    ///
    /// If the flag is set, existing bookmarks will be fetched, and
    /// the cached content will be replaced with the fetched content.
    #[arg(short, long)]
    pub replace: bool,
    /// Cache the fetched bookmarks as text, HTML or markdown file.
    #[arg(short, long, value_enum)]
    pub mode: Option<CacheMode>,
    /// Get the difference between the fetched and cached
    /// bookmark for the given urls.
    ///
    /// Multiple urls are separated by a whitespace.
    #[arg(short, long, value_name = "URLs", num_args = 0.., value_delimiter = ' ')]
    pub diff: Vec<String>,
    /// Fetch and cache specified URLs.
    ///
    /// Multiple URLs are separated by a whitespace.
    /// If an URL is missing in the bookmarks, it will be imported.
    #[arg(long, num_args = 0.., value_delimiter = ' ')]
    pub urls: Vec<String>,
    /// Run command in dry mode.
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,
}

/// Describes the arguments for the `sync` subcommand.
#[derive(ClapArgs, Debug)]
pub struct SyncArgs {
    /// Cache the fetched bookmarks as text, HTML or markdown file.
    #[arg(short, long, value_enum)]
    pub mode: Option<CacheMode>,
    /// Run command in dry mode.
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,
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

/// Describes the arguments for the `add` subcommand.
#[derive(ClapArgs, Debug)]
pub struct AddArgs {
    /// Add specified URLs as bookmark.
    ///
    /// Multiple URLs are separated by a whitespace.
    #[arg(num_args = 0.., value_name = "URLs", value_delimiter = ' ')]
    pub urls: Vec<String>,
    /// Run command in dry mode.
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,
}

/// Describes the arguments for the `remove` subcommand.
#[derive(ClapArgs, Debug)]
pub struct RemoveArgs {
    /// Remove specified URLs from bookmark.
    ///
    /// Multiple URLs are separated by a whitespace.
    #[arg(num_args = 0.., value_name = "URLs", value_delimiter = ' ')]
    pub urls: Vec<String>,
}
