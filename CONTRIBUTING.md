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

- **Rust**: Version 1.72+ ([Install Rust](https://www.rust-lang.org/tools/install))
- **Git**: For version control ([Install Git](https://git-scm.com/downloads))
- **OpenSSL**: Development libraries for secure connections
  - Ubuntu/Debian: `sudo apt-get install pkg-config libssl-dev`
  - macOS: `brew install pkg-config openssl`
  - Windows: Handled automatically by vcpkg

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

Git Editor has a test suite of roughly **105** tests (~83 unit including docs module + **22** integration).

### Test Structure

```
tests
├── Unit Tests (inline #[cfg(test)] under src/)
│   ├── args.rs
│   ├── docs.rs
│   ├── utils/datetime.rs
│   ├── utils/validator.rs
│   ├── utils/commit_history.rs
│   ├── utils/types.rs
│   ├── utils/prompt.rs
│   ├── utils/simulation.rs
│   ├── utils/git_clone.rs
│   ├── utils/git_config.rs
│   ├── rewrite/rewrite_specific.rs
│   └── rewrite/rewrite_range.rs
└── Integration Tests (22 tests)
    └── tests/integration_tests.rs
```

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

Workflows live under `.github/workflows/`:

### 1. Comprehensive Test Suite (`test.yml`)
- Runs on every push and PR
- Executes unit and integration tests

### 2. CI/CD Pipeline (`ci-cd.yaml`)
- Runs linting, formatting, and tests
- Cross-compiles for multiple platforms
- Uploads build artifacts

### 3. Multi-Platform Testing (`multi-platform-test.yml`)
- Tests on Ubuntu, Windows, and macOS
- Tests with stable and beta Rust versions

### 4. Coverage Report (`coverage.yml`)
- Generates test coverage reports
- Uploads coverage to Codecov
- Builds rustdoc and may deploy to GitHub Pages

### 5. User Docs Pages (`github-pages.yml`)
- Runs `git-editor --docs` and deploys the HTML user docs site

### 6. Release Pipeline (`release.yaml`)
- Runs on version tags (`v*`)
- Publishes to crates.io and creates GitHub releases with binaries / packages

## Project Structure

```
git-editor/
├── src/
│   ├── main.rs              # Entry point and mode dispatch
│   ├── lib.rs               # Library re-exports (args, rewrite, utils)
│   ├── args.rs              # Command-line argument parsing
│   ├── docs.rs              # --docs HTML generation
│   ├── rewrite/
│   │   ├── mod.rs
│   │   ├── rewrite_all.rs   # Full history rewriting
│   │   ├── rewrite_specific.rs # Pick-one commit editing
│   │   └── rewrite_range.rs # Range editing (crossterm TUI)
│   └── utils/
│       ├── mod.rs
│       ├── commit_history.rs
│       ├── datetime.rs
│       ├── prompt.rs
│       ├── types.rs
│       ├── validator.rs
│       ├── simulation.rs
│       ├── git_clone.rs
│       ├── git_config.rs
│       └── help.rs          # Unused custom help (clap --help is primary)
├── docs/
│   └── template.html        # Embedded by --docs / GitHub Pages
├── tests/
│   └── integration_tests.rs
├── .github/
│   └── workflows/
├── Cargo.toml
├── README.md
└── CONTRIBUTING.md
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
export PKG_CONFIG_PATH="/usr/local/opt/openssl/lib/pkgconfig"
```

#### Git2 Compilation Issues
```bash
# Ubuntu/Debian
sudo apt-get install libgit2-dev

# macOS
brew install libgit2
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
