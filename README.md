# Git Editor: Git History Rewriting Tool

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Git Editor is a powerful Rust-based command-line utility designed to safely rewrite Git commit metadata within a specified date range. Perfect for fixing commit dates, adding consistency to repositories, or reconstructing development timelines.

## Features

- Rewrite commit timestamps within a specified date range
- Pick and edit specific commits interactively
- Show commit history with detailed statistics
- Preserve commit order and relationships
- Maintain author and committer information
- Compatible with any Git repository
- Docker support for containerized execution

## Installation

### Prerequisites

- Rust 1.72+ ([Install Rust](https://www.rust-lang.org/tools/install))
- Git ([Install Git](https://git-scm.com/downloads))
- OpenSSL development libraries

### From Source

```bash
# Clone the repository
git clone https://github.com/rohansen856/git-editor.git
cd git-editor

# Build the project
cargo build --release

# The binary will be available at target/release/git-editor
```

## Usage

Git Editor supports four main modes of operation:

### 1. Full History Rewrite (Default)
```bash
git-editor --repo-path "/path/to/repo" --email "user@example.com" --name "Author Name" --begin "YYYY-MM-DD HH:MM:SS" --end "YYYY-MM-DD HH:MM:SS"
```

### 2. Show History Only
```bash
git-editor --repo-path "/path/to/repo" --show-history
# or
git-editor --repo-path "/path/to/repo" -s
```

### 3. Pick Specific Commits
```bash
git-editor --repo-path "/path/to/repo" --pick-specific-commits
# or
git-editor --repo-path "/path/to/repo" -p
```

### 4. Range Editing
```bash
git-editor --repo-path "/path/to/repo" --range
# or
git-editor --repo-path "/path/to/repo" -x
```

### Arguments

| Option | Short | Description | Required |
| ------ | ----- | ----------- | -------- |
| `--repo-path` | `-r` | Path to the Git repository (defaults to current directory) | Optional |
| `--email` | | Email address to associate with rewritten commits | Only for full rewrite |
| `--name` | `-n` | Name to associate with rewritten commits | Only for full rewrite |
| `--begin` | `-b` | Start date for commits (format: YYYY-MM-DD HH:MM:SS) | Only for full rewrite |
| `--end` | `-e` | End date for commits (format: YYYY-MM-DD HH:MM:SS) | Only for full rewrite |
| `--show-history` | `-s` | Show commit history with statistics | Optional |
| `--pick-specific-commits` | `-p` | Interactive mode to edit specific commits | Optional |
| `--range` | `-x` | Interactive mode to edit a specific range of commits | Optional |

### Examples

```bash
# Full rewrite: Rewrite commits to occur between January 1 and January 7, 2023
git-editor --repo-path "/path/to/repo" --email "john.doe@example.com" --name "John Doe" --begin "2023-01-01 00:00:00" --end "2023-01-07 23:59:59"

# Show history: Display commit history with detailed statistics
git-editor --repo-path "/path/to/repo" -s

# Pick specific commits: Interactively select and edit individual commits
git-editor --repo-path "/path/to/repo" -p

# Range editing: Interactively select and edit a range of commits
git-editor --repo-path "/path/to/repo" -x

# Using the Makefile (after editing the parameters)
make run
```

## How It Works

Git Editor operates by:

1. Validating the provided repository path and Git environment
2. Generating a list of timestamps between the start and end dates
3. Creating a mapping between original commits and new commit timestamps
4. Rewriting the Git history while preserving parent-child relationships
5. Updating branch references to point to the new commit history

The tool ensures that commit order is maintained and distributes commits evenly across the specified time range.

## Warning

**This tool rewrites Git history.** Always work on a separate branch or backup your repository before running Git Editor on important code bases.

## Development

### Project Structure

```
git-editor/
├── src/
│   ├── main.rs           # Entry point
│   ├── args.rs           # Command line argument handling
│   ├── rewrite/          # Git history rewriting logic
│   │   ├── mod.rs        # Module definition
│   │   ├── rewrite_all.rs    # Full repository history rewriting
│   │   ├── rewrite_specific.rs # Interactive commit selection
│   │   └── rewrite_range.rs    # Interactive range selection
│   ├── utils/            # Utility modules
│   │   ├── mod.rs        # Module definition
│   │   ├── types.rs      # Type definitions and custom Result
│   │   ├── validator.rs  # Input validation
│   │   ├── datetime.rs   # Date and time functions
│   │   ├── commit_history.rs # Git commit operations
│   │   └── prompt.rs     # Interactive user prompts
│   └── lib.rs            # Library interface
├── tests/
│   └── integration_tests.rs # Integration tests
├── Cargo.toml            # Project dependencies
├── Dockerfile            # Docker configuration
├── Makefile              # Build automation
└── CLAUDE.md             # Development guidance
```

### Testing

The project includes comprehensive test coverage with both unit and integration tests:

```bash
# Run all tests (53 tests total: 45 unit + 8 integration)
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration_tests

# Run specific integration test with output
cargo test --test integration_tests test_show_history_mode_integration -- --nocapture
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.