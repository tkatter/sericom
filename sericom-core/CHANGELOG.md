# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0](https://github.com/tkatter/sericom/compare/sericom-core/v0.6.0...sericom-core/v0.7.0) - 2025-09-25

### Added

- *(configs)* added config option for a default dir for debug output

### Changed

- Make unix imports conditional

## [0.6.0](https://github.com/tkatter/sericom/compare/sericom-core/v0.5.0...sericom-core/v0.6.0) - 2025-09-25

### Added

- *(exit-script)* Added ability to run script after file is written
  - Added APIs to for script/path validation

### Documentation

- *(exit-script)* Documentation for new APIs and updating old API docs respectively

### Fixed

- *(exit-script)* Windows compatibility for exit-scripts

## [0.5.0](https://github.com/tkatter/sericom/compare/sericom-core/v0.4.0...sericom-core/v0.5.0) - 2025-09-13

### Changed

- Swap git-cliff for release-plz
- git-cliff configuration

### Fixed

- *(ascii)* Fixed processing of ascii highlight & bold escape sequences
- *(configs)* [**breaking**] Removed hl_fg and hl_bg configurations to use the inverse

## [0.4.0](https://github.com/tkatter/sericom/releases/tag/sericom-core/v0.4.0) - 2025-09-11

### Added

- Added CLI config-overrides for text color and out_dir by @tkatter

### Fixed

- Fixed ascii clear-screen bug and file path bug by @tkatter

## [0.2.0](https://github.com/tkatter/sericom/releases/tag/sericom-core/v0.2.0) - 2025-09-02

### Added

- Added the standard 'break' signal keybinding by @tkatter in [#4](https://github.com/tkatter/sericom/pull/4)
- Added sericom-core as dependency for sericom, bumped version by @tkatter

## [0.1.0](https://github.com/tkatter/sericom/releases/tag/sericom-core/v0.1.0) - 2025-09-02

### Added

- Added description for crates.io publish by @tkatter

## [0.2.0](https://github.com/tkatter/sericom/releases/tag/0.2.0) - 2025-09-01

### Added

- Added link to config options in the TomlError error message by @tkatter
- Added CHANGELOG by @tkatter
- Added some CI by @tkatter
- Added package field to Cargo.toml for creating .deb packages with cargo-deb by @tkatter

### Fixed

- Fixed CI yml by @tkatter
- Fixed CI yml by @tkatter
- Fixed CI yml by @tkatter

## [0.1.0](https://github.com/tkatter/sericom/releases/tag/0.1.0) - 2025-08-28

### Added

- Added LICENSE by @tkatter
- Added conditional compilation of debugging output feature by @tkatter
- Added nxos by @tkatter
- Added writing settings to a file by @tkatter
- Added simple profile for windows release by @tkatter
- Added README by @tkatter
- Added a few subcommands by @tkatter

### Changed

- Changed character handling to handle ascii escape sequences and render by @tkatter
- Changed file blocking thread from tokio::task::spawn_blockng to use std::thread::spawn by @tkatter
- Started implementing mouse event handling, changed handling of file writing, added debug output, very simple/basic implementation of 'scripting' by @tkatter

### Fixed

- Implemented a screen buffer to enable mouse-related functionality. CHORE: fix backspace bug by @tkatter
- Fix for Windows double-sending of input by @tkatter
- Fixed bug where it wouldn't properly recognize '\n' by @tkatter

### Removed

- Removed tracing/tokio console deps by @tkatter
- Removed debug print line by @tkatter

**Diffs:**

[unreleased]: https://github.com/tkatter/sericom/compare/sericom-core/v0.4.0..HEAD
[0.4.0]: https://github.com/tkatter/sericom/compare/sericom-core/v0.3.0..sericom-core/v0.4.0
[0.2.0]: https://github.com/tkatter/sericom/compare/sericom-core/v0.1.0..sericom-core/v0.2.0
[0.1.0]: https://github.com/tkatter/sericom/compare/v0.2.0..sericom-core/v0.1.0
[0.2.0]: https://github.com/tkatter/sericom/compare/v0.1.0..v0.2.0
