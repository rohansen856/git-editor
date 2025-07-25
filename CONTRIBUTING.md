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
2. Clone your fork locally:
   ```bash
   git clone https://github.com/rohansen856/git-editor.git
   cd git-editor
   ```

## Development Setup

### Install Dependencies

```bash
# Install Rust components
rustup component add clippy rustfmt

# Install development dependencies
cargo fetch
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
- **Documentation**: Use clear, descriptive variable and function names

## Testing

Git Editor has a comprehensive test suite with 44 tests covering all functionality.

### Test Structure

```
tests
├── Unit Tests (36 tests)
│   ├── args.rs (4 tests)
│   ├── utils/datetime.rs (3 tests)
│   ├── utils/validator.rs (10 tests)
│   ├── utils/commit_history.rs (6 tests)
│   ├── utils/types.rs (6 tests)
│   ├── utils/prompt.rs (2 tests)
│   └── rewrite/rewrite_specific.rs (5 tests)
└── Integration Tests (8 tests)
    └── integration_tests.rs
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

# Run tests in verbose mode
cargo test --verbose

# Run specific test
cargo test test_name
```

### Test Categories

#### Unit Tests
- **Argument Parsing**: Tests for command-line argument handling
- **Datetime Functionality**: Tests for timestamp generation and validation
- **Validation**: Tests for input validation (email, dates, repository paths)
- **Commit History**: Tests for Git commit retrieval and processing
- **Types**: Tests for custom data structures and type aliases
- **Rewrite Functionality**: Tests for commit modification logic

#### Integration Tests
- **Show History Mode**: End-to-end testing of `-s` flag
- **Pick Specific Commits Mode**: End-to-end testing of `-p` flag
- **Full Rewrite Mode**: End-to-end testing of full history rewriting
- **Error Handling**: Tests for invalid inputs and edge cases
- **Workflow Testing**: Tests for combined operations

### Writing Tests

When adding new functionality, follow these guidelines:

1. **Write unit tests** for individual functions and modules
2. **Write integration tests** for complete workflows
3. **Use descriptive test names** that explain what is being tested
4. **Test both success and failure cases**
5. **Use temporary directories** for Git repository tests
6. **Mock external dependencies** when possible

Example test structure:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_function_name_success_case() {
        // Arrange
        let input = "test input";
        
        // Act
        let result = function_name(input);
        
        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_value);
    }
    
    #[test]
    fn test_function_name_error_case() {
        // Test error conditions
        let result = function_name("invalid input");
        assert!(result.is_err());
    }
}
```

### Test Coverage

We aim for comprehensive test coverage. To generate coverage reports:

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

### Pull Request Template

When submitting a pull request, include:

- **Clear description** of what the PR does
- **Testing information** about what tests were added/modified
- **Breaking changes** if any
- **Related issues** that the PR addresses

### PR Requirements

- ✅ All tests must pass
- ✅ Code must be formatted with `rustfmt`
- ✅ Code must pass `clippy` linting
- ✅ New functionality must include tests
- ✅ Documentation must be updated if needed

## CI/CD Pipeline

Our CI/CD pipeline includes multiple workflows:

### 1. Comprehensive Test Suite (`test.yml`)
- Runs on every push and PR
- Executes unit tests, integration tests, and full test suite
- Generates test reports

### 2. CI/CD Pipeline (`ci-cd.yaml`)
- Runs linting, formatting, and tests
- Cross-compiles for multiple platforms
- Uploads build artifacts

### 3. Multi-Platform Testing (`multi-platform-test.yml`)
- Tests on Ubuntu, Windows, and macOS
- Tests with stable and beta Rust versions
- Ensures cross-platform compatibility

### 4. Security Audit (`security.yml`)
- Runs security audits with `cargo-audit`
- Checks for vulnerable dependencies
- Runs weekly security scans

### 5. Coverage Report (`coverage.yml`)
- Generates test coverage reports
- Uploads coverage to Codecov
- Generates and deploys documentation

### 6. Release Pipeline (`release.yaml`)
- Runs comprehensive tests before release
- Publishes to crates.io
- Creates GitHub releases with binaries

## Project Structure

```
git-editor/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Library root
│   ├── args.rs              # Command-line argument parsing
│   ├── rewrite/             # Git rewriting functionality
│   │   ├── mod.rs
│   │   ├── rewrite_all.rs   # Full history rewriting
│   │   └── rewrite_specific.rs # Specific commit editing
│   └── utils/               # Utility modules
│       ├── mod.rs
│       ├── commit_history.rs # Git commit operations
│       ├── datetime.rs      # Date/time handling
│       ├── prompt.rs        # User input handling
│       ├── types.rs         # Custom types and structs
│       └── validator.rs     # Input validation
├── tests/
│   └── integration_tests.rs # Integration tests
├── .github/
│   └── workflows/           # CI/CD pipelines
├── Cargo.toml              # Project dependencies
├── README.md               # Project documentation
└── CONTRIBUTING.md         # This file
```

## Common Development Tasks

### Adding New Functionality

1. **Design the feature** and identify where it fits in the codebase
2. **Write tests first** (TDD approach recommended)
3. **Implement the functionality**
4. **Update documentation** if needed
5. **Run the full test suite**
6. **Submit a PR**

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

Many tests create temporary Git repositories. Use the provided helper functions:

```rust
fn create_test_repo() -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_str().unwrap().to_string();
    
    // Initialize git repo
    let repo = git2::Repository::init(&repo_path).unwrap();
    
    // Create commits, etc.
    
    (temp_dir, repo_path)
}
```

### Performance Testing

```bash
# Run tests with timing information
cargo test -- --nocapture --test-threads=1 --exact

# Profile the application
cargo build --release
time ./target/release/git-editor --help
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
# Install system git development headers
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

## Code of Conduct

Please note that this project follows a code of conduct. Be respectful and professional in all interactions.

## License

By contributing to Git Editor, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to Git Editor! Your help makes this project better for everyone.