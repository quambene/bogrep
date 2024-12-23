# Bogrep – Grep your bookmarks

[![Latest Version](https://img.shields.io/crates/v/bogrep.svg)](https://crates.io/crates/bogrep)
[![Build Status](https://github.com/quambene/bogrep/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/quambene/bogrep/actions/workflows/rust-ci.yml)
[![codecov](https://codecov.io/gh/quambene/bogrep/graph/badge.svg)](https://codecov.io/gh/quambene/bogrep)
[![dependency status](https://deps.rs/repo/github/quambene/bogrep/status.svg)](https://deps.rs/repo/github/quambene/bogrep)

Bogrep downloads and caches your bookmarks in plaintext without images or
videos. Use the Bogrep CLI to grep through your cached bookmarks in full-text
search.

``` bash
bogrep -i "reed-solomon code"
```

![Bogrep mockup](/assets/mockup.png)

- [Install Bogrep](#install-bogrep)
  - [Install Bogrep from crates.io](#install-bogrep-from-cratesio)
  - [Install Bogrep from github.com](#install-bogrep-from-githubcom)
- [Usage](#usage)
  - [Search](#search)
- [Getting help](#getting-help)
- [Import bookmarks](#import-bookmarks)
- [Filter bookmark folders](#filter-bookmark-folders)
- [Ignore URLs](#ignore-urls)
- [Fetch underlying urls](#fetch-underlying-urls)
- [Diff websites](#diff-websites)
- [Manage internal bookmarks](#manage-internal-bookmarks)
- [Request throttling](#request-throttling)
- [Supported operating systems](#supported-operating-systems)
- [Troubleshooting](#troubleshooting)
- [Testing](#testing)

## Install Bogrep

### Install Bogrep from [crates.io](https://crates.io/crates/bogrep)

``` bash
# Build and install bogrep binary to ~/.cargo/bin
cargo install bogrep
```

To update bogrep to a new version, run `cargo install bogrep` again. Versions
0.x will not be backwards compatible and breaking changes are expected. Remove
Bogrep's configuration directory (see [Supported operating
systems](#supported-operating-systems)) if you experience an issue when running
Bogrep.

### Install Bogrep from [github.com](https://github.com/quambene/bogrep)

``` bash
git clone git@github.com:quambene/bogrep.git
cd bogrep

# Build and install bogrep binary to ~/.cargo/bin
cargo install --path .
```

## Usage

Settings and cache are installed to the configuration path, after Bogrep has
been run for the first time. The configuration path depends on your operating
system (see [Supported operating systems](#supported-operating-systems)).

``` bash
# Initialize bookmark sources
bogrep init

# Import bookmarks from selected sources
bogrep import

# Fetch and cache bookmarks
bogrep fetch

# Search your bookmarks in full-text search
bogrep <pattern>
```

To simulate the import of bookmarks, use `bogrep import --dry-run`.

### Search

``` bash
bogrep [OPTIONS] [PATTERN]
```

``` properties
Options:
  -v, --verbose...          
  -m, --mode <MODE>         Search the cached bookmarks in HTML or plaintext format [possible values: html, text]
  -i, --ignore-case         Ignore case distinctions in patterns
  -l, --files-with-matches  Print only URLs of bookmarks with matched lines
  -h, --help                Print help
  -V, --version             Print version
```

## Getting help

``` bash
# Check version
bogrep --version

# Print help
bogrep --help

# Print help for subcommands
bogrep config --help
bogrep import --help
bogrep fetch --help
```

## Import bookmarks

Import of bookmarks is supported from the following browsers:

- Firefox (in `.json` and `.jsonlz4` format)
- Chromium (in `.json` format)
- Chrome (in `.json` format)
- Edge (in `.json` format)
- Safari (in `.plist` format)

If bookmark files are not detected by `bogrep import`, you can configure them
manually using:

``` bash
bogrep config --source ~/path/to/bookmarks/file
```

## Filter bookmark folders

Filter which bookmark folders are imported. Multiple folders are separated by whitespaces:

``` bash
bogrep config --source "my/path/to/bookmarks_file.json" --folders dev science articles
```

## Ignore urls

Ignore specific urls. The content for these urls will not be fetched and cached.

It can be useful to ignore urls for video or music platforms which
usually don't include relevant text to grep.

``` bash
# Ignore one or more urls
bogrep config --ignore <url1> <url2> ...
```

## Fetch underlying urls

Fetch the underlying urls of supported websites:

``` bash
bogrep config --underlying <url1> <url2> ...
```

For example, if a specific url like
`https://news.ycombinator.com/item?id=00000000` is bookmarked, the underlying
article will be fetched and cached.

Supported domains are:

- news.ycombinator.com
- reddit.com

## Diff websites

Fetch difference between cached and fetched website for multiple urls, and display changes:

``` bash
bogrep fetch --diff <url1> <url2> ...
```

## Manage internal bookmarks

If you need to add specific URLs to the search index, use the `bogrep add` subcommand.

``` bash
# Add URLs to search index
bogrep add <url1> <url2> ...

# Remove URLs from search index
bogrep remove <url1> <url2> ...

# Add URLs to search index and fetch content from URLs
bogrep fetch <url1> <url2> ...
```

## Request throttling

Fetching of bookmarks from the same host is conservatively throttled, but can
also be configured in the `settings.json` usually
placed at `~/.config/bogrep` in your home directory:

``` json
{
    "cache_mode": "text",
    "max_concurrent_requests": 100,
    "request_timeout": 60000,
    "request_throttling": 3000,
    "max_idle_connections_per_host": 10,
    "idle_connections_timeout": 5000
}
```

where `request_throttling` is the waiting time between requests for the same
host in milliseconds.

Too speed up fetching, set `max_concurrent_requests` to e.g. 1000. The maximum
number of available sockets depends on your operating system. Run `ulimit -n` to
show the maximum number of open sockets allowed on your system.

For the available settings see <https://docs.rs/bogrep/latest/bogrep/struct.Settings.html>.

## Supported operating systems

Bogrep assumes and creates a configuration path at

- `$HOME/.config/bogrep` for Linux,
- `$HOME/Library/Application Support/bogrep` for macOS,
- `C:\Users\<Username>\AppData\Roaming/bogrep` for Windows,

in your home directory for storing the `settings.json`, `bookmarks.json`, and
`cache` folder.

You can configure the configuration path via the environment variable
`BOGREP_HOME`.

## Troubleshooting

### Missing file permissions on macOS

If file permissions are missing for Bogrep, allow CLI applications to access the
filesystem in the System Preferences.

### Missing search results

The default bogrep search is case-sensitive. Try `bogrep -i` for
case-insensitive search.

## Testing

``` bash
# Run unit tests and integration tests
cargo test

# Run unit tests
cargo test --lib

# Run integration tests
cargo test --test '*'
```
