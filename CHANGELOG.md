<!-- markdownlint-disable MD041 MD034 -->

### Unreleased

- added
  - Implement `bogrep import --urls <URLs>` (import specified URLs)
  - Implement `bogrep fetch --urls <URLs>` (fetch specified URLs)

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
