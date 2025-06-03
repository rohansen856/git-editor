# Git Editor: Git History Rewriting Tool

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Git Editor is a powerful Rust-based command-line utility designed to safely rewrite Git commit timestamps within a specified date range. Perfect for fixing commit dates, adding consistency to repositories, or reconstructing development timelines.

## Features

- Rewrite commit timestamps within a specified date range
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
git clone https://github.com/yourusername/git-editor.git
cd git-editor

# Build the project
cargo build --release

# The binary will be available at target/release/git-editor
```

## Usage

```bash
git-editor --repo-path "/path/to/repo" --email "user@example.com" --name "Author Name" --start "YYYY-MM-DD HH:MM:SS" --end "YYYY-MM-DD HH:MM:SS"
```

### Arguments

| Option | Description |
| ------ | ----------- |
| `--repo-path` | Path to the Git repository |
| `--email` | Email address to associate with rewritten commits |
| `--name` | Name to associate with rewritten commits |
| `--start` | Start date for commits (format: YYYY-MM-DD HH:MM:SS) |
| `--end` | End date for commits (format: YYYY-MM-DD HH:MM:SS) |

### Examples

```bash
# Rewrite commits to occur between January 1 and January 7, 2023
git-editor --repo-path "/path/to/repo" --email "john.doe@example.com" --name "John Doe" --start "2023-01-01 00:00:00" --end "2023-01-07 23:59:59"

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
│   ├── main.rs        # Entry point
│   ├── args.rs        # Command line argument handling
│   ├── datetime.rs    # Date and time functions
│   ├── validator.rs   # Input validation
│   ├── rewrite.rs     # Git history rewriting logic
│   ├── types.rs       # Type definitions
│   ├── heatmap.rs     # GitHub contribution data handling
│   └── utils/         # Utility functions
├── Cargo.toml         # Project dependencies
├── Dockerfile         # Docker configuration
└── Makefile           # Build automation
```

### Testing

```bash
cargo test
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.