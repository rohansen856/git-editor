# Git Editor: Git History Rewriting Tool

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Git Editor is a powerful Rust-based command-line utility designed to safely rewrite Git commit metadata within a specified date range. Perfect for fixing commit dates, adding consistency to repositories, or reconstructing development timelines.

## Features

- **Git URL Cloning**: Automatically clone remote repositories from URLs (GitHub, GitLab, etc.)
- **Multiple Operation Modes**: Full rewrite, specific commits, range editing, history viewing, and simulation
- **Simulation Mode**: Preview changes without applying them (dry-run functionality)
- **Flexible Range Editing**: Edit messages, authors, or timestamps selectively
- **Interactive Commit Selection**: Pick and edit specific commits with detailed previews
- **Smart Git Config Integration**: Auto-detects user name and email from Git configuration
- **Comprehensive History Analysis**: Show commit history with detailed statistics
- **Preserve Git Integrity**: Maintain commit order, relationships, and repository structure
- **Cross-platform Support**: Works on Linux, macOS, and Windows
- **Docker Support**: Containerized execution for consistent environments

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

## Documentation

📚 **Comprehensive documentation is available online:** [rohansen856.github.io/git-editor](https://rohansen856.github.io/git-editor)

The online documentation includes:
- Complete command reference with examples
- Technical implementation details
- Architecture overview and development guidelines
- Interactive copy-to-clipboard code examples
- Troubleshooting guide and FAQ
- Advanced usage patterns and best practices

### Quick Access to Documentation

You can also access the documentation directly from the command line:

```bash
git-editor --docs
```

This command will generate and open the comprehensive documentation in your default browser.

## Usage

Git Editor supports five main modes of operation:

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

# Edit only specific aspects of commits in range mode
git-editor --repo-path "/path/to/repo" -x --message  # Edit only commit messages
git-editor --repo-path "/path/to/repo" -x --author   # Edit only author information
git-editor --repo-path "/path/to/repo" -x --time     # Edit only timestamps
```

### 5. Simulation Mode (Dry-run)
```bash
# Preview changes without applying them
git-editor --simulate --repo-path "/path/to/repo" --email "user@example.com" --name "Author Name" --begin "YYYY-MM-DD HH:MM:SS" --end "YYYY-MM-DD HH:MM:SS"

# Show detailed diff in simulation
git-editor --simulate --show-diff --repo-path "/path/to/repo" --email "user@example.com" --name "Author Name" --begin "YYYY-MM-DD HH:MM:SS" --end "YYYY-MM-DD HH:MM:SS"
```

### 6. Git URL Cloning
```bash
# Automatically clone and process remote repositories
git-editor --simulate --repo-path "https://github.com/user/repo"
git-editor --simulate --repo-path "https://github.com/user/repo.git"
git-editor --repo-path "git@github.com:user/repo.git" --email "user@example.com" --name "Author Name" --begin "2023-01-01 00:00:00" --end "2023-12-31 23:59:59"
```

### Arguments

| Option | Short | Description | Required |
| ------ | ----- | ----------- | -------- |
| `--repo-path` | `-r` | Path or URL to the Git repository (defaults to current directory) | Optional |
| `--email` | | Email address to associate with rewritten commits | Only for full rewrite |
| `--name` | `-n` | Name to associate with rewritten commits | Only for full rewrite |
| `--begin` | `-b` | Start date for commits (format: YYYY-MM-DD HH:MM:SS) | Only for full rewrite |
| `--end` | `-e` | End date for commits (format: YYYY-MM-DD HH:MM:SS) | Only for full rewrite |
| `--show-history` | `-s` | Show commit history with statistics | Optional |
| `--pick-specific-commits` | `-p` | Interactive mode to edit specific commits | Optional |
| `--range` | `-x` | Interactive mode to edit a specific range of commits | Optional |
| `--simulate` | | Preview changes without applying them (dry-run mode) | Optional |
| `--show-diff` | | Show detailed diff preview (requires --simulate) | Optional |
| `--message` | | Edit only commit messages in range mode | Optional |
| `--author` | | Edit only author name and email in range mode | Optional |
| `--time` | | Edit only timestamps in range mode | Optional |

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

# Simulation mode: Preview changes before applying
git-editor --simulate --repo-path "/path/to/repo" --email "john.doe@example.com" --name "John Doe" --begin "2023-01-01 00:00:00" --end "2023-01-07 23:59:59"

# Git URL cloning: Work with remote repositories
git-editor --simulate --repo-path "https://github.com/rohansen856/git-editor"

# Selective range editing: Edit only timestamps in a commit range
git-editor --repo-path "/path/to/repo" -x --time

# Detailed simulation with diff preview
git-editor --simulate --show-diff --repo-path "/path/to/repo" --email "john.doe@example.com" --name "John Doe" --begin "2023-01-01 00:00:00" --end "2023-01-07 23:59:59"

# Using the Makefile (after editing the parameters)
make run
```

## How It Works

Git Editor operates by:

1. **Repository Access**: Validates local paths or automatically clones Git URLs to temporary directories
2. **Smart Configuration**: Auto-detects user name and email from Git config, with fallback prompts
3. **Operation Mode Selection**: Determines the appropriate mode based on provided flags
4. **Simulation Analysis**: In simulation mode, analyzes potential changes without modifying the repository
5. **Timestamp Generation**: Creates evenly distributed timestamps within the specified date range
6. **History Rewriting**: Safely rewrites commit metadata while preserving relationships and integrity
7. **Reference Updates**: Updates all branch and tag references to point to the rewritten history

The tool ensures that:
- Commit order and parent-child relationships are maintained
- Repository integrity is preserved throughout the process
- All Git objects remain valid and accessible
- Temporary directories are automatically cleaned up after URL-based operations

## Warning

**This tool rewrites Git history.** Always work on a separate branch or backup your repository before running Git Editor on important code bases.

## Development

### Project Structure

```
git-editor/
├── src/
│   ├── main.rs           # Entry point and operation mode handling
│   ├── args.rs           # Command line argument parsing and Git URL cloning
│   ├── rewrite/          # Git history rewriting logic
│   │   ├── mod.rs        # Module definition
│   │   ├── rewrite_all.rs    # Full repository history rewriting
│   │   ├── rewrite_specific.rs # Interactive commit selection
│   │   └── rewrite_range.rs    # Interactive range selection and editing
│   ├── utils/            # Utility modules
│   │   ├── mod.rs        # Module definition
│   │   ├── types.rs      # Type definitions and custom Result
│   │   ├── validator.rs  # Input validation for all modes
│   │   ├── datetime.rs   # Date and time functions
│   │   ├── commit_history.rs # Git commit operations
│   │   ├── prompt.rs     # Interactive user prompts
│   │   ├── git_clone.rs  # Git URL detection and repository cloning
│   │   ├── git_config.rs # Git configuration reading (cross-platform)
│   │   └── simulation.rs # Simulation mode and preview functionality
│   └── lib.rs            # Library interface
├── tests/
│   └── integration_tests.rs # Comprehensive integration tests (15 tests)
├── .github/workflows/    # CI/CD pipelines
│   ├── ci-cd.yaml        # Main build and test pipeline
│   ├── release.yaml      # Multi-platform release automation
│   ├── coverage.yml      # Code coverage reporting
│   └── multi-platform-test.yml # Cross-platform testing
├── Cargo.toml            # Project dependencies and metadata
├── Dockerfile            # Docker configuration
├── Makefile              # Build automation and development commands
├── CLAUDE.md             # Development guidance for AI assistants
├── CHANGELOG.md          # Version history and release notes
└── LICENSE               # MIT license
```

### Testing

The project includes comprehensive test coverage with both unit and integration tests:

```bash
# Run all tests (82 tests total: 67 unit + 15 integration)
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration_tests

# Run specific test categories
cargo test git_clone  # Test Git URL cloning functionality
cargo test simulation # Test simulation mode features
cargo test validator  # Test input validation

# Run specific integration test with output
cargo test --test integration_tests test_show_history_mode_integration -- --nocapture

# Test coverage includes:
# - All operation modes (full rewrite, specific commits, range editing, simulation)
# - Git URL detection and cloning functionality
# - Cross-platform Git configuration reading
# - Input validation and error handling
# - Simulation mode with diff preview
# - Integration tests for complete workflows
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.