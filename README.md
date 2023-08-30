# Bogrep – Grep your bookmarks

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
- [Import bookmarks](#import-bookmarks)
  - [Firefox](#firefox)
  - [Chrome](#chrome)
- [Ignore URLs](#ignore-urls)
- [Diff websites](#diff-websites)

## Install Bogrep

### Install Bogrep from [crates.io](https://crates.io/crates/bogrep)

``` bash
# Build and install bogrep binary to ~/.cargo/bin; settings and cache are installed to ~/.config/bogrep
cargo install bogrep
```

### Install Bogrep from [github.com](https://github.com/quambene/bogrep)

``` bash
git clone git@github.com:quambene/bogrep.git
cd bogrep

# Build and install bogrep binary to ~/.cargo/bin; settings and cache are installed to ~/.config/bogrep
cargo install --path .
```

## Usage

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

## Import bookmarks

The path of bookmarks may depend on your operating system.

### Firefox

Configure Firefox as source for bookmarks, where `<my_profile>` is your Firefox profile:

``` bash
# snap package
bogrep config --source ~/snap/firefox/common/.mozilla/firefox/<my_profile>/bookmarkbackups

# apt package
bogrep config --source ~/.mozilla/firefox/<my_profile>/bookmarkbackups
```

Directory `bookmarkbackups` contains multiple backups files, and `bogrep` will
choose the most recent bookmarks file.

### Chrome

Configure Chrome as source for bookmarks:

``` bash
bogrep config --source ~/.config/google-chrome/Default/bookmarks
```

### Specify bookmark folders

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

Ignored urls will be added to the file `~/.config/bogrep/.bogrepignore`, which
can be edited manually as well.

## Diff websites

Fetch difference between cached and fetched website for multiple urls, and display changes:

``` bash
bogrep fetch --diff <url1> <url2> ...
```
