# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.4](https://github.com/tkatter/sericom/compare/sericom/v0.5.3...sericom/v0.5.4) - 2025-09-25

### Added

- *(configs)* added config option for a default dir for debug output

## [0.5.3](https://github.com/tkatter/sericom/compare/sericom/v0.5.2...sericom/v0.5.3) - 2025-09-25

### Documentation

- *(exit-script)* Updated README and config example files

### Feat

- *(exit-script)* Ability to specify a script to run after writing a file

## [0.5.2](https://github.com/tkatter/sericom/compare/sericom/v0.5.1...sericom/v0.5.2) - 2025-09-13

### Changed

- Removed aarch64 from cargo-dist action and added path to

## [0.5.1](https://github.com/tkatter/sericom/compare/sericom/v0.5.0...sericom/v0.5.1) - 2025-09-13

### Changed

- Added cargo-dist ci for binary creations
- Swap git-cliff for release-plz
- git-cliff configuration

### Fixed

- *(tracing)* Fixed compatability between Windows and Linux serial ports as paths
- *(configs)* [**breaking**] Removed hl_fg and hl_bg configurations to use the inverse

## [0.5.0](https://github.com/tkatter/sericom/releases/tag/sericom/v0.5.0) - 2025-09-11

### Added

- Added CLI config-overrides for text color and out_dir by @tkatter

### Fixed

- Fixed ascii clear-screen bug and file path bug by @tkatter

## [0.3.1](https://github.com/tkatter/sericom/releases/tag/sericom/v0.3.1) - 2025-09-02

### Added

- Added the standard 'break' signal keybinding by @tkatter in [#4](https://github.com/tkatter/sericom/pull/4)

## [0.3.0](https://github.com/tkatter/sericom/releases/tag/sericom/v0.3.0) - 2025-09-02

### Added

- Added sericom-core as dependency for sericom, bumped version by @tkatter

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

[unreleased]: https://github.com/tkatter/sericom/compare/sericom/v0.5.0..HEAD
[0.5.0]: https://github.com/tkatter/sericom/compare/sericom/v0.4.0..sericom/v0.5.0
[0.3.1]: https://github.com/tkatter/sericom/compare/sericom/v0.3.0..sericom/v0.3.1
[0.3.0]: https://github.com/tkatter/sericom/compare/v0.2.0..sericom/v0.3.0
[0.2.0]: https://github.com/tkatter/sericom/compare/v0.1.0..v0.2.0
