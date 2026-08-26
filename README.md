# Git Editor: Git History Rewriting Tool

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Git Editor is a Rust CLI that rewrites Git commit metadata (author, email, timestamps, messages) within a date range or interactively, while preserving commit order and parent relationships.

## Features

- **Git URL Cloning**: Automatically clone remote repositories from URLs (GitHub, GitLab, etc.)
- **Multiple Operation Modes**: Full rewrite, pick commits, range editing, history viewing, simulation, and docs
- **Simulation Mode**: Preview changes without applying them (`--simulate`, optional `--show-diff`)
- **Flexible Range Editing**: Crossterm TUI; limit fields with `--message`, `--author`, or `--time`
- **Interactive Commit Selection**: Pick one commit by number and edit its fields
- **Smart Git Config Integration**: Auto-detects user name and email from Git configuration
- **KEEP_ORIGINAL timestamps**: Accepting prompt date defaults can keep original times and only rewrite author info
- **`--skip-range-check`**: Bypass the default ≥3h gap rule for tightly packed timestamps
- **Preserve Git Integrity**: Maintain commit order, relationships, and repository structure
- **Signoff trailers**: Matching `Signed-off-by` / `Co-authored-by` / `Authored-by` lines are updated with the new author
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

### Quick Access to Documentation

```bash
git-editor --docs
```

This generates local HTML from `docs/template.html` and opens it in your browser (skipped if `NO_BROWSER` or `GIT_EDITOR_NO_BROWSER` is set).

## Usage

Git Editor supports six main modes of operation (precedence: `--docs` → `--simulate` → `-x` → `-p` → `-s` → full rewrite):

### 1. Full History Rewrite (Default)
```bash
git-editor --repo-path "/path/to/repo" --email "user@example.com" --name "Author Name" --begin "YYYY-MM-DD HH:MM:SS" --end "YYYY-MM-DD HH:MM:SS"
```

Shows a simulation preview and asks for confirmation before rewriting.

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

Interactive numbered list; select one commit and edit its fields.

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

Range input: `start-end` (e.g. `5-11`) or `*` for all commits.

### 5. Simulation Mode (Dry-run)
```bash
# Preview changes without applying them
git-editor --simulate --repo-path "/path/to/repo" --email "user@example.com" --name "Author Name" --begin "YYYY-MM-DD HH:MM:SS" --end "YYYY-MM-DD HH:MM:SS"

# Show detailed diff in simulation
git-editor --simulate --show-diff --repo-path "/path/to/repo" --email "user@example.com" --name "Author Name" --begin "YYYY-MM-DD HH:MM:SS" --end "YYYY-MM-DD HH:MM:SS"
```

### 6. Documentation
```bash
git-editor --docs
```

### Git URL Cloning
```bash
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
| `--show-history` | `-s` | Show commit history with statistics (read-only) | Optional |
| `--pick-specific-commits` | `-p` | Interactive mode to edit one specific commit | Optional |
| `--range` | `-x` | Interactive mode to edit a specific range of commits | Optional |
| `--simulate` | | Preview changes without applying them (dry-run mode) | Optional |
| `--show-diff` | | Show detailed diff preview (requires --simulate) | Optional |
| `--message` | | Edit only commit messages in range mode | Optional |
| `--author` | | Edit only author name and email in range mode | Optional |
| `--time` | | Edit only timestamps in range mode | Optional |
| `--skip-range-check` | | Skip the ≥3h minimum gap check; pack with ≥5-minute gaps | Optional |
| `--docs` | | Open documentation in the browser | Optional |

### Examples

```bash
# Full rewrite: Rewrite commits to occur between January 1 and January 7, 2023
git-editor --repo-path "/path/to/repo" --email "john.doe@example.com" --name "John Doe" --begin "2023-01-01 00:00:00" --end "2023-01-07 23:59:59"

# Tight date range without the 3-hour gap rule
git-editor --repo-path "/path/to/repo" --email "john.doe@example.com" --name "John Doe" --begin "2023-01-01 00:00:00" --end "2023-01-01 12:00:00" --skip-range-check

# Show history: Display commit history with detailed statistics
git-editor --repo-path "/path/to/repo" -s

# Pick specific commits: Interactively select and edit one commit
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

# Using the Makefile
make run
```

## How It Works

Git Editor operates by:

1. **Repository Access**: Validates local paths or automatically clones Git URLs to temporary directories
2. **Smart Configuration**: Auto-detects user name and email from Git config, with fallback prompts
3. **Operation Mode Selection**: Determines the mode from flags (see precedence above)
4. **Simulation Analysis**: In simulation mode (or as a preview before full rewrite), analyzes potential changes without modifying the repository until confirmed
5. **Timestamp Generation**: Creates randomly weighted timestamps within the specified date range (minimum 3-hour gaps by default; 5-minute gaps with `--skip-range-check`)
6. **History Rewriting**: Creates new commits with remapped parents via libgit2 (`git2`), and rewrites matching `Signed-off-by` / `Co-authored-by` / `Authored-by` trailers when author identity changes
7. **Reference Updates**: Updates `refs/heads/<branch>` to the rewritten tip

The tool ensures that:
- Commit order and parent-child relationships are maintained
- Temporary directories from URL clones are cleaned up when the process exits

## Warning

**This tool rewrites Git history.** Always work on a separate branch or backup your repository before running Git Editor on important code bases. Force pushes will be required for remotes after rewriting.

## Development

### Testing

```bash
# Run all tests (~105 total: ~83 unit including docs + 22 integration)
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
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
