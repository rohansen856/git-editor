# Changelog

All notable changes to this project will be documented in this file.

## [2.0.0] - 2024-09-27

### Added
- Interactive tabular editor for range mode with background highlighting
- Cross-platform git config integration for default email/name values
- Interactive prompts with faded git config defaults
- Background color highlighting for selection cursor
- Terminal formatting fixes for range mode
- Default CLI behavior changed to run full rewrite mode instead of help
- Comprehensive simulation mode with detailed change preview
- Support for selective field editing in range mode

### Changed
- Default behavior: `cargo run` now executes full history rewrite instead of showing help
- Range mode UI: replaced bracket `[]` indicators with colored backgrounds
- Terminal handling: fixed raw mode timing issues
- Prompts: git config values shown in faded color with Enter to accept

### Fixed
- Terminal display corruption in range mode
- Cross-platform git config file reading
- Merge conflicts resolution
- Cross-compilation issues with OpenSSL dependency

### Technical
- Added 64 unit tests + 15 integration tests
- Improved cross-platform compatibility
- Enhanced error handling and validation
- Updated dependencies for better performance

## [1.8.0] - Previous releases
See Git history for previous release notes.