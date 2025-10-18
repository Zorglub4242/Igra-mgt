# Contributing to IGRA Orchestra CLI

Thank you for your interest in contributing to the IGRA Orchestra CLI! This document provides guidelines and instructions for contributing.

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [Development Setup](#development-setup)
4. [Making Changes](#making-changes)
5. [Coding Standards](#coding-standards)
6. [Testing](#testing)
7. [Submitting Changes](#submitting-changes)
8. [Issue Guidelines](#issue-guidelines)

## Code of Conduct

This project follows the IGRA Community standards. Please be respectful, collaborative, and constructive in all interactions.

## Getting Started

### Prerequisites

- Rust 1.70+ ([install from rustup.rs](https://rustup.rs/))
- Docker 23.0+ with Docker Compose V2
- Git
- A working IGRA Orchestra installation (for testing)

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR-USERNAME/Igra-mgt.git
   cd Igra-mgt
   ```
3. Add upstream remote:
   ```bash
   git remote add upstream https://github.com/Zorglub4242/Igra-mgt.git
   ```

## Development Setup

### Build the Project

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with logs
RUST_LOG=debug cargo run
```

### Install for Testing

```bash
# Build and install
./build.sh
sudo ./install.sh

# Or manually
cargo build --release
sudo cp target/release/igra-cli /usr/local/bin/
```

### Project Structure

```
src/
├── main.rs              # CLI entry point and command handlers
├── cli.rs               # Command-line argument parsing
├── app.rs               # TUI application state and event handling
├── core/                # Core business logic
│   ├── docker.rs        # Docker API operations
│   ├── config.rs        # Configuration management
│   ├── wallet.rs        # Wallet operations
│   ├── rpc.rs           # RPC token management
│   ├── ssl.rs           # SSL certificate checking
│   ├── log_parser.rs    # Log parsing and metrics extraction
│   ├── backup.rs        # Backup procedures
│   ├── health.rs        # Health check documentation
│   └── metrics.rs       # Metrics collection documentation
├── screens/
│   ├── mod.rs           # Screen implementations
│   └── dashboard.rs     # TUI rendering logic
├── widgets/
│   └── mod.rs           # Widget documentation
└── utils/
    ├── constants.rs     # Service definitions
    ├── helpers.rs       # Utility functions
    └── app_config.rs    # Application configuration
```

## Making Changes

### Workflow

1. Create a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes following the coding standards

3. Test your changes thoroughly

4. Commit with descriptive messages:
   ```bash
   git commit -m "feat: Add feature description"
   ```

5. Push to your fork:
   ```bash
   git push origin feature/your-feature-name
   ```

6. Open a Pull Request

### Branch Naming

- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions or modifications

## Coding Standards

### Rust Style

- Follow official Rust style guidelines
- Use `cargo fmt` before committing
- Ensure `cargo clippy` passes without warnings
- Add documentation comments (`///`) for public APIs
- Use meaningful variable and function names

### Code Formatting

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Lint code
cargo clippy

# Fix lints
cargo clippy --fix
```

### Documentation

- Document all public functions, structs, and modules
- Include examples in documentation where helpful
- Update README.md and USER_GUIDE.md for user-facing changes
- Update CHANGELOG.md following [Keep a Changelog](https://keepachangelog.com/) format

### Commit Messages

Follow conventional commits format:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code refactoring
- `test`: Test additions or modifications
- `chore`: Maintenance tasks
- `perf`: Performance improvements

**Examples:**
```
feat(log-parser): Add kaspad sync status detection

Implemented regex-based parsing to detect when kaspad is synced
vs syncing by analyzing log patterns for "Accepted blocks via relay".

Closes #123
```

```
fix(dashboard): Correct memory percentage calculation

The memory percentage was incorrectly calculated when memory_limit
was zero, causing division by zero panics.
```

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests in specific module
cargo test core::log_parser
```

### Writing Tests

- Add unit tests for all new functions
- Add integration tests for new features
- Test edge cases and error conditions
- Use meaningful test names that describe what is being tested

**Example:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kaspad_synced_status() {
        let logs = "Accepted 7 blocks via relay\nTx throughput: 5.0 u-tps";
        let metrics = parse_kaspad_logs(logs);

        assert_eq!(metrics.status_text, Some("Synced".to_string()));
        assert_eq!(metrics.primary_metric, Some("5.0 TPS".to_string()));
        assert!(metrics.is_healthy);
    }
}
```

### Manual Testing

Before submitting a PR, manually test:

1. **TUI Functionality**:
   - Launch TUI and verify all 7 screens work
   - Test navigation (Tab, numbers, arrow keys)
   - Test search functionality (`/`)
   - Test service actions (restart, stop, logs)

2. **CLI Commands**:
   ```bash
   igra-cli status
   igra-cli start --profile backend
   igra-cli logs viaduct -n 100
   igra-cli config view
   ```

3. **Error Handling**:
   - Test with Docker stopped
   - Test with invalid input
   - Test with missing configuration

## Submitting Changes

### Pull Request Process

1. **Update Documentation**:
   - Update CHANGELOG.md in the `[Unreleased]` section
   - Update README.md if adding user-facing features
   - Update USER_GUIDE.md with new functionality

2. **Ensure Quality**:
   ```bash
   cargo fmt
   cargo clippy
   cargo test
   cargo build --release
   ```

3. **Create Pull Request**:
   - Use a clear, descriptive title
   - Reference related issues (`Fixes #123`, `Closes #456`)
   - Describe what changed and why
   - Include screenshots for UI changes
   - List any breaking changes

4. **PR Template**:
   ```markdown
   ## Description
   Brief description of changes

   ## Type of Change
   - [ ] Bug fix
   - [ ] New feature
   - [ ] Breaking change
   - [ ] Documentation update

   ## Testing
   - [ ] Tests pass locally
   - [ ] Added new tests
   - [ ] Manual testing completed

   ## Screenshots (if applicable)
   [Attach screenshots]

   ## Checklist
   - [ ] Code follows project style guidelines
   - [ ] Documentation updated
   - [ ] CHANGELOG.md updated
   - [ ] No new warnings from clippy
   ```

### Review Process

1. Maintainers will review your PR
2. Address any requested changes
3. Once approved, your PR will be merged
4. Your contribution will be credited in the CHANGELOG

## Issue Guidelines

### Reporting Bugs

Use the bug report template and include:

- Clear, descriptive title
- Steps to reproduce
- Expected vs actual behavior
- System information (OS, Rust version, Docker version)
- Logs or error messages
- Screenshots if applicable

**Example:**

```markdown
**Bug**: TUI crashes when pressing 'd' on stopped container

**Steps to Reproduce**:
1. Launch `igra-cli`
2. Navigate to Services screen
3. Select a stopped container
4. Press 'd' for logs

**Expected**: Show error message
**Actual**: Application crashes with panic

**Environment**:
- OS: Ubuntu 22.04
- Rust: 1.75.0
- igra-cli: 0.2.1

**Logs**:
```
thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value'
```
```

### Feature Requests

Use the feature request template and include:

- Clear description of the feature
- Use case and benefits
- Proposed implementation (if you have ideas)
- Any alternative solutions considered

### Questions

For questions:
- Check existing documentation first
- Search closed issues
- Open a discussion or issue with `question` label

## Development Tips

### Useful Commands

```bash
# Watch for changes and rebuild
cargo watch -x build

# Run with specific features
cargo run --features "feature-name"

# Generate documentation
cargo doc --open

# Check dependencies
cargo tree

# Update dependencies
cargo update
```

### Debugging

```bash
# Enable debug logs
RUST_LOG=debug igra-cli

# Enable trace logs
RUST_LOG=trace igra-cli

# Debug specific module
RUST_LOG=igra_cli::core::log_parser=debug igra-cli
```

### Performance Profiling

```bash
# Profile release build
cargo build --release
perf record --call-graph=dwarf ./target/release/igra-cli
perf report
```

## Getting Help

- **Documentation**: See README.md and USER_GUIDE.md
- **Issues**: [GitHub Issues](https://github.com/Zorglub4242/Igra-mgt/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Zorglub4242/Igra-mgt/discussions)
- **IGRA Community**: [Discord Server](https://discord.gg/igra)

## Recognition

Contributors will be:
- Listed in the CHANGELOG
- Credited in release notes
- Thanked in the community

Thank you for contributing to IGRA Orchestra CLI! 🦀⚡

---

**Questions?** Open an issue with the `question` label or reach out on Discord.
