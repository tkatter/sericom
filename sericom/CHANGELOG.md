# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.1](https://github.com/tkatter/sericom/compare/sericom-v0.5.0...sericom-v0.5.1) - 2025-09-13

### Fixed

- fixed ascii clear-screen bug and file path bug

### Other

- Updated Cargo.toml for use with cargo-wix
- Release sericom v0.5.0 & sericom-core v0.4.0
- moved tracing subscriber & appender from workspace deps to sericom deps
- docs and made some screen_buffer items public
- cargo fmt
- handled file error for passing in a dir, refactored stdin_loop, started playing with tracing
- Added CLI config-overrides for text color and out_dir
- Update CHANGELOG and README
- Release sericom v0.4.0 && sericom-core v0.3.0
- Fixed handling of ASCII escape sequences ([#5](https://github.com/tkatter/sericom/pull/5))
- Release sericom 0.3.1 & sericom-core v0.2.0
- Added the standard 'break' signal keybinding ([#4](https://github.com/tkatter/sericom/pull/4))
- Updated sericom/Cargo.toml with 'cargo-wix' release configurations
- Updated release download links in README and added documentation field to sericom's Cargo.toml since it is a binary crate now.
- added sericom-core as dependency for sericom, bumped version
- Create sericom-core as a library for sericom ([#3](https://github.com/tkatter/sericom/pull/3))
- Updated README curl links to latest version, also added steps for checking the sha256 checksum
- Updated README and added files relative to configuring sericom
- Updated README with crates.io badge and navigation links
- Updated README with installation instructions
- Updated README
- cleaned up for pub
- Update README.md
- Update README.md
- updated README
- bumped version and updated README
- updated readme
- README formatting
- updated README and started working on reading a serial port
- edited README
- edited README
- Added README
