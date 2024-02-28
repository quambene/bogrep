<!-- markdownlint-disable MD041 MD034 -->

### Unreleased

- added
- changed
- removed

### v0.8.0

- added
  - Implement running in dry mode
  - Select sources from user input
- changed
  - Select sources for `bogrep import` if no sources are configured
  - Update rust toolchain to 1.76
  - Fix `settings.json` for `bogrep config --ignore` and `bogrep config --underlying` if run
    multiple times

### v0.7.0

- added
  - Add `action` to `TargetBookmark`
  - Add benchmarks for fetching
  - Take ignored urls into account in `bogrep import`
  - Fetch underlying urls
  - Clean up lock file if `bogrep` is aborted
  - Add `SelectSource` and `ReadSource` traits
  - Add `PlistReader` and `SafariReader`
- changed
  - Update to rust 1.75
  - Fix duplicate cache files for `bogrep fetch --urls`
  - Fix report of processed bookmarks
- removed
  - Remove `integration-test` feature

### v0.6.1

- changed
  - Fix `Cache::is_empty`

### v0.6.0

- added
  - Implement `bogrep -w <pattern>` (match only whole words)
- changed
  - Set default for `max_idle_connections_per_host` from 100 to 10
  - Fetch: Degrade hard failure to warning message for `BogrepError::CreateFile`
    and `BogrepError::ConvertHost`

### v0.5.0

- added
  - Implement `bogrep add <URLs>` (add specified URLs to bookmarks)
  - Implement `bogrep remove <URLs>` (remove specified URLs from bookmarks)
  - Implement `bogrep fetch --urls <URLs>` (fetch specified URLs)
  - Add sources to `SourceBookmark` and `TargetBookmark`
  - Add cache modes to `TargetBookmark`
  - Implement progress bar and status report for processing fetched bookmarks
- changed
  - Fix dns errors for `bogrep fetch`
  - Fix panic for `bogrep <pattern>`

### v0.4.0

- added
  - Implement `bogrep -i <pattern>` (ignore case distinctions in patterns)
  - Implement `bogrep -l <pattern>` (print only URLs of bookmarks with matched lines)

### v0.3.0

- added
  - Add integration test for `bogrep config --ignore`
  - Support Edge browser
- changed
  - Refactor from `bogrep ignore` to `bogrep config --ignore`
  - Fix overwrite of `Source` in settings
  - Write `bookmarks.json` atomically
  - Filter out responses with content type `application/*`, `image/*`, `audio/*`, `video/*`

### v0.2.0

- added
  - Extend CI pipeline
    - Build doc
    - Add doc tests
    - Test on macOS and Windows
  - Add `--all` flag for `bogrep clean`
  - Improve documentation
- changed
  - Refactor `trait ReadBookmark`
  - Fix duplicate sources in `settings.json`
  - Make config path platform-independent
  - Fix `bogrep fetch --diff`
  - Downgrade fetching error to warning
- removed
  - Remove cache mode for markdown

### v0.1.5

- changed
  - Validate source file in `cmd::configure`
  - Fix format not supported for source files

### v0.1.4

- added
  - Add integration tests
- changed
  - Improve test coverage
