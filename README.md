# Bogrep – Grep your bookmarks

[![Latest Version](https://img.shields.io/crates/v/bogrep.svg)](https://crates.io/crates/bogrep)
[![Build Status](https://github.com/quambene/bogrep/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/quambene/bogrep/actions/workflows/rust-ci.yml)
[![codecov](https://codecov.io/gh/quambene/bogrep/graph/badge.svg)](https://codecov.io/gh/quambene/bogrep)

Bogrep downloads and caches your bookmarks in plaintext without images or
videos. Use the Bogrep CLI to grep through your cached bookmarks in full-text
search.

``` bash
bogrep "reed-solomon code"
```

![Bogrep mockup](/assets/mockup.png)

- [Install Bogrep](#install-bogrep)
  - [Install Bogrep from crates.io](#install-bogrep-from-cratesio)
  - [Install Bogrep from github.com](#install-bogrep-from-githubcom)
- [Usage](#usage)
- [Getting help](#getting-help)
- [Import bookmarks](#import-bookmarks)
  - [Firefox](#firefox)
  - [Chrome](#chrome)
  - [Chromium](#chromium)
- [Specify bookmark folders](#specify-bookmark-folders)
- [Ignore URLs](#ignore-urls)
- [Diff websites](#diff-websites)
- [Request throttling](#request-throttling)
- [Supported operating systems](#supported-operating-systems)
- [Testing](#testing)

## Install Bogrep

### Install Bogrep from [crates.io](https://crates.io/crates/bogrep)

``` bash
# Build and install bogrep binary to ~/.cargo/bin
cargo install bogrep
```

### Install Bogrep from [github.com](https://github.com/quambene/bogrep)

``` bash
git clone git@github.com:quambene/bogrep.git
cd bogrep

# Build and install bogrep binary to ~/.cargo/bin
cargo install --path .
```

## Usage

Settings and cache are installed to `~/.config/bogrep`, after Bogrep has been
run for the first time.

``` bash
# Configure the path to the bookmarks file (e.g. of your browser)
bogrep config --source "my/path/to/bookmarks_file.json"

# Import bookmarks
bogrep import

# Fetch and cache bookmarks
bogrep fetch

# Search your bookmarks in full-text search
bogrep <pattern>
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

Currently, bookmarks in JSON format for Firefox, Chrome, and Chromium are
supported. Bookmark files in HTML format are not supported yet.

The path of bookmarks may be different for your operating system.

### Firefox

Configure Firefox as source for bookmarks, where `<my_profile>` is your Firefox profile:

``` bash
# Ubuntu (snap package)
bogrep config --source ~/snap/firefox/common/.mozilla/firefox/<my_profile>/bookmarkbackups

# Ubuntu (apt package)
bogrep config --source ~/.mozilla/firefox/<my_profile>/bookmarkbackups

# macOS
bogrep config --source ~/Library/Application Support/Firefox/Profiles/<my_profile>/bookmarkbackups
```

Directory `bookmarkbackups` contains multiple compressed backup files (in
format `.jsonlz4`), and `bogrep` will choose the most recent bookmarks file.

### Chrome

Configure Chrome as source for bookmarks:

``` bash
# Ubuntu
bogrep config --source ~/.config/google-chrome/Default/Bookmarks

# macOS
bogrep config --source ~/Library/Application Support/Google/Chrome/Default/Bookmarks
```

### Chromium

Configure Chromium as source for bookmarks:

``` bash
# Ubuntu (snap package)
bogrep config --source ~/snap/chromium/common/chromium/Default/Bookmarks
```

## Specify bookmark folders

Specify which bookmark folders are imported. Multiple folders are separated by comma:

``` bash
bogrep config --source "my/path/to/bookmarks_file.json" --folders dev,science,articles
```

## Ignore urls

Ignore specific urls. The content for these urls will not be fetched and cached.

It can be useful to ignore urls for video or music platforms which
usually don't include relevant text to grep.

``` bash
# Ignore one or more urls
bogrep ignore <url1> <url2> ...
```

## Diff websites

Fetch difference between cached and fetched website for multiple urls, and display changes:

``` bash
bogrep fetch --diff <url1> <url2> ...
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
    "request_throttling": 3000
}
```

where `request_throttling` is the waiting time between requests for the same
host in milliseconds.

## Supported operating systems

Bogrep assumes a configuration path at `~/.config/bogrep` in your home directory
for storing the `settings.json`, `bookmarks.json`, and `cache` folder. This
should work for most Linux derivatives. Feel free to open an issue if you need
support Windows.

You can configure the configuration path via the environment variable
`BOGREP_HOME`. For example,

``` bash
BOGREP_HOME="my/config/path" bogrep config --source bookmarks.json
```

## Testing

``` bash
# Run unit tests
cargo test

# Run integration tests
cargo test --test '*' --features integration-test

# Run unit and integration tests
cargo test --features integration-test
```
