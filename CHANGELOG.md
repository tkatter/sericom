# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

### Removed

### Fixed

### Deprecated

### Security

## sericom/[0.3.0](https://github.com/tkatter/sericom/releases/tag/sericom/v0.3.0) - 2025-09-01

## sericom-core/[0.1.0](https://github.com/tkatter/sericom/releases/tag/sericom-core/v0.1.0) - 2025-09-01

### Added

- Sericom-core library to house all of the underlying library code for sericom
- Add MIT license for sericom-core

### Changed

- Changed sericom to a workspace and dependent on the sericom-core crate
- GPLv3.0 license specifically applies to the sericom binary

### Removed

- All sericom's public api - transfered to sericom-core

## [0.2.0](https://github.com/tkatter/sericom/releases/tag/v0.2.0) - 2025-09-01

### Added

- User config file for sericom ([#1](https://github.com/tkatter/sericom/pull/1))
- Better printing of errors with miette and thiserror ([#2](https://github.com/tkatter/sericom/pull/2))
- Added CHANGELOG for documenting changes
- Added configuration reference files ([0f9379c](https://github.com/tkatter/sericom/commit/0f9379cd28379c74439e63d3535e1c4487e0d6fe))
- Added CI for pushes to main and PRs

### Changed

- Debug output now a public facing feature ([#1](https://github.com/tkatter/sericom/pull/1))
- Updated README to reflect new changes/functionality

## [0.1.0](https://github.com/tkatter/sericom/releases/tag/v0.1.0) - 2025-08-28

### Added

- Base functionality of sericom
- Ability to communicate with devices over a serial connection
- Write received data to a file
- Scrolling
- Copy/paste
- Open connection, list available ports, list settings for a port, list valid baud rates

## Diffs

sericom/[unreleased]: https://github.com/tkatter/sericom/compare/sericom/v0.3.0...HEAD
sericom-core/[unreleased]: https://github.com/tkatter/sericom/compare/sericom-core/v0.1.0...HEAD

sericom/[0.3.0]: https://github.com/tkatter/sericom/compare/v0.2.0...sericom/v0.3.0
sericom-core/[0.1.0]: https://github.com/tkatter/sericom/releases/tag/sericom-core/v0.1.0
[0.2.0]: https://github.com/tkatter/sericom/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/tkatter/sericom/releases/tag/v0.1.0
