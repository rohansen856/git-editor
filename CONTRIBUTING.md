# Contributing to Git Editor

Thank you for your interest in contributing to Git Editor! This guide will help you get started with contributing to the project.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Code Style and Standards](#code-style-and-standards)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [CI/CD Pipeline](#cicd-pipeline)
- [Project Structure](#project-structure)
- [Common Development Tasks](#common-development-tasks)
- [Troubleshooting](#troubleshooting)

## Getting Started

### Prerequisites

- **Rust**: 1.87+ (git2 0.21 needs it; checked by the MSRV job in CI) ([Install Rust](https://www.rust-lang.org/tools/install))
- **Git**: for version control ([Install Git](https://git-scm.com/downloads)); the tool itself does not need the `git` command at runtime
- **C toolchain and OpenSSL headers**: libgit2 is vendored and built from source
  - Ubuntu/Debian: `sudo apt-get install build-essential pkg-config libssl-dev`
  - macOS: `brew install pkg-config openssl`
  - Windows: MSVC build tools; OpenSSL is not needed (`openssl-sys` is not in the Windows dependency tree)

### Fork and Clone

1. Fork the repository on GitHub
2. Clone **your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USER/git-editor.git
   cd git-editor
   ```
3. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/rohansen856/git-editor.git
   ```

## Development Setup

### Install Dependencies

```bash
# Install Rust components
rustup component add clippy rustfmt

# Install development dependencies
cargo fetch
```

### Install Pre-Commit Hooks

We use [`pre-commit`](https://pre-commit.com/) to automatically run formatters, linters, and tests before each commit. This prevents failing builds in CI.

1. Ensure `pre-commit` is installed on your system (`pip install pre-commit`, `brew install pre-commit`, etc.)
2. Install the git hook scripts:

```bash
pre-commit install
```

### Build the Project

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run the application
cargo run -- --help
```

## Code Style and Standards

### Formatting

We use `rustfmt` for consistent code formatting:

```bash
# Format all code
cargo fmt

# Check formatting without making changes
cargo fmt --check
```

### Linting

We use `clippy` for code quality and style:

```bash
# Run clippy
cargo clippy --all-targets --all-features

# Run clippy with stricter rules (CI requirement)
cargo clippy --all-targets --all-features -- -D warnings
```

### Code Guidelines

- **Error Handling**: Use the custom `Result<T>` type defined in `utils/types.rs`
- **Imports**: Group imports logically and remove unused imports
- **Functions**: Keep functions focused and single-purpose
- **Testing**: Write tests for all new functionality
- **Documentation**: Keep `README.md` and `docs/template.html` aligned with real behavior

## Testing

`cargo test` runs four suites:

| Suite | Where | What it covers |
|---|---|---|
| Unit tests | `#[cfg(test)]` modules under `src/` | parsing, validation, dates, trailers, TUI state machine, JSON builders |
| Engine tests | `tests/engine.rs` | rewrite engine on raw fixture commits: reuse of untouched commits, preserved headers/encodings/offsets, committer modes, merges, backup/stale refs, `git fsck --strict` |
| End-to-end tests | `tests/e2e.rs` | the real binary with `--yes --json`: full rewrite, `--keep-dates`, `-p --commit`, `-x --select`, exit codes, refusals |
| Integration tests | `tests/integration_tests.rs` | library flows (history, timestamp generation, validation, docs, argument parsing) |

The binary is a thin wrapper over the `git_editor` library, so every test runs once.

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test --lib                    # Unit tests only
cargo test --test integration_tests # Integration tests only

# Run tests with output
cargo test -- --nocapture

# Skip opening a browser during docs-related tests
GIT_EDITOR_NO_BROWSER=1 cargo test

# Run specific test
cargo test test_name
```

### Writing Tests

When adding new functionality, follow these guidelines:

1. **Write unit tests** for individual functions and modules
2. **Write integration tests** for complete workflows
3. **Use descriptive test names** that explain what is being tested
4. **Test both success and failure cases**
5. **Use temporary directories** for Git repository tests
6. Prefer `serial_test` when tests would otherwise race on shared state

### Test Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --verbose --all-features --workspace --timeout 120

# Generate HTML coverage report
cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out html
```

## Pull Request Process

### Before Submitting

1. **Update your fork**:
   ```bash
   git fetch upstream
   git checkout master
   git merge upstream/master
   ```

2. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make your changes** and commit them:
   ```bash
   git add .
   git commit -m "feat: add new feature"
   ```

4. **Run the full test suite**:
   ```bash
   cargo test
   cargo fmt --check
   cargo clippy --all-targets --all-features -- -D warnings
   ```

### PR Requirements

- All tests must pass
- Code must be formatted with `rustfmt`
- Code must pass `clippy` linting
- New functionality must include tests
- Documentation must be updated if behavior changes

## CI/CD Pipeline

Workflows live under `.github/workflows/`. Every third-party action is pinned to a commit SHA, and the default token is read-only.

### 1. CI (`ci-cd.yaml`): pushes to master/main and all pull requests
- `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --locked` on Ubuntu, macOS and Windows
- MSRV check with Rust 1.87
- `cargo deny check` (advisories, licenses, sources, bans; see `deny.toml`)

### 2. Coverage (`coverage.yml`)
- `cargo tarpaulin` produces `cobertura.xml` and an HTML report (uploaded as a build artifact)
- Uploads to Codecov on pushes when the `CODECOV_TOKEN` secret is set

### 3. User docs (`github-pages.yml`)
- Builds the site with `git-editor --docs --docs-out docs-site/index.html`
- Deploys to GitHub Pages only from master/main; only the deploy job has `pages`/`id-token` write permissions

### 4. Release (`release.yaml`): `v*` tags
- Fails unless the tag equals `v` + the `version` in `Cargo.toml`, so bump the version before tagging
- Lint and test, then build binaries, `.deb`/`.rpm` (pinned `cargo-deb`/`cargo-generate-rpm`), `.msi` (checksum-verified WiX, stable GUIDs from `[package.metadata.wix]`), `.pkg` and archives
- Creates the GitHub release with `SHA256SUMS`; only this job gets `contents: write`
- Publishes to crates.io **after** the release, and only for `X.Y.0` tags, from the protected `release` environment using the `CARGO_REGISTRY_TOKEN` secret

## Project Structure

```
git-editor/
├── src/
│   ├── main.rs              # CLI entry point and mode dispatch
│   ├── lib.rs               # Library crate (all modules)
│   ├── args.rs              # Command-line arguments (clap) and argument resolution
│   ├── docs.rs              # --docs HTML generation
│   ├── output.rs            # say! macros and --json documents
│   ├── rewrite/
│   │   ├── engine.rs        # History rewrite engine (plans, raw commit rebuilding, ref update)
│   │   ├── report.rs        # Human/JSON reporting of rewrite outcomes
│   │   ├── rewrite_all.rs   # Full-rewrite plans
│   │   ├── rewrite_specific.rs # Pick-one-commit mode (menu and --commit)
│   │   └── rewrite_range.rs # Range mode (crossterm table and --select)
│   └── utils/
│       ├── commit_history.rs # Branch history listing
│       ├── dates.rs          # Date parsing/formatting with offsets
│       ├── datetime.rs       # Timestamp distribution over a range
│       ├── git_clone.rs      # URL detection, cloning with credentials, URL redaction
│       ├── git_config.rs     # Identity defaults from git config (libgit2)
│       ├── message_trailers.rs # Signed-off-by / Co-authored-by / Authored-by rewriting
│       ├── prompt.rs         # Prompts, confirmation, cancellation/exit codes
│       ├── sanitize.rs       # Escaping of control characters in displayed metadata
│       ├── simulation.rs     # Previews built from plans
│       ├── types.rs          # Shared types
│       └── validator.rs      # Input validation
├── docs/template.html       # Embedded by --docs / GitHub Pages
├── tests/                   # engine.rs, e2e.rs, integration_tests.rs
├── AGENTS.md                # Guide for LLM agents and scripts
└── deny.toml                # cargo-deny policy
```

## Common Development Tasks

### Adding New Functionality

1. Design the feature and identify where it fits
2. Write tests first when practical
3. Implement the functionality
4. Update `README.md` and `docs/template.html` if user-facing
5. Run the full test suite
6. Submit a PR

### Debugging Tests

```bash
# Run a specific test with output
cargo test test_name -- --nocapture

# Run tests with backtraces
RUST_BACKTRACE=1 cargo test

# Run tests in single-threaded mode
cargo test -- --test-threads=1
```

### Working with Git Repositories in Tests

Many tests create temporary Git repositories:

```rust
fn create_test_repo() -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap().to_string();

    let repo = git2::Repository::init(&repo_path).unwrap();
    // Create commits, etc.

    (temp_dir, repo_path)
}
```

## Troubleshooting

### Common Issues

#### OpenSSL Errors
```bash
# Ubuntu/Debian
sudo apt-get install pkg-config libssl-dev

# macOS
brew install pkg-config openssl
export PKG_CONFIG_PATH="$(brew --prefix openssl)/lib/pkgconfig"   # /opt/homebrew on Apple Silicon, /usr/local on Intel
```

#### Git2 Compilation Issues
`libgit2-sys` builds its bundled libgit2 1.9.0 from source unless a matching system libgit2 is found, so installing `libgit2-dev` is normally unnecessary. Build failures are almost always missing `pkg-config`, OpenSSL headers (Linux/macOS) or a C compiler.
```bash
# Ubuntu/Debian
sudo apt-get install build-essential pkg-config libssl-dev
```

#### Test Failures in CI
- Check that all dependencies are installed
- Verify that temporary directories are being created correctly
- Ensure tests don't interfere with each other (use `serial_test` if needed)

### Getting Help

- **Issues**: Check existing GitHub issues or create a new one
- **Discussions**: Use GitHub Discussions for questions
- **Code Review**: Tag maintainers in your PR for review

## Conduct

Be respectful and professional in all interactions.

## License

By contributing to Git Editor, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to Git Editor!
