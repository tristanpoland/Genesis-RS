# Contributing to Genesis-RS

Thank you for your interest in contributing to Genesis-RS! This document provides guidelines and information for contributors.

## Code of Conduct

This project adheres to the Contributor Covenant Code of Conduct. By participating, you are expected to uphold this code.

## Getting Started

### Prerequisites

- Rust 1.75.0 or later
- Git
- BOSH CLI, spruce, safe/vault (for integration tests)

### Setup

```bash
git clone https://github.com/genesis-community/genesis-rs
cd genesis-rs
cargo build
cargo test
```

## Development Workflow

1. **Fork the repository** and create your branch from `main`
2. **Make your changes** with clear, focused commits
3. **Write or update tests** for your changes
4. **Ensure tests pass** with `cargo test --all`
5. **Format your code** with `cargo fmt --all`
6. **Lint your code** with `cargo clippy --all -- -D warnings`
7. **Submit a pull request**

## Project Structure

```
Genesis-RS/
├── crates/
│   ├── genesis-types/      # Core types and traits
│   ├── genesis-core/       # Utilities, config, logging
│   ├── genesis-manifest/   # Manifest generation
│   ├── genesis-services/   # External service clients
│   ├── genesis-secrets/    # Secret management
│   ├── genesis-kit/        # Kit handling
│   ├── genesis-env/        # Environment management
│   └── genesis-cli/        # CLI binary
└── tests/                  # Integration tests
```

## Coding Standards

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Keep functions focused and small
- Write documentation for public APIs
- Add examples to complex functions

### Documentation

- All public APIs must have rustdoc comments
- Include examples where appropriate
- Document error cases
- Keep documentation up-to-date with code changes

### Testing

- Write unit tests for all new functionality
- Add integration tests for end-to-end scenarios
- Use property-based testing where appropriate
- Aim for >80% code coverage

### Commits

- Write clear, descriptive commit messages
- Use conventional commit format:
  - `feat: add new feature`
  - `fix: resolve bug`
  - `docs: update documentation`
  - `test: add tests`
  - `refactor: improve code structure`
  - `perf: performance improvement`
  - `chore: maintenance tasks`

## Pull Request Process

1. Update documentation for any API changes
2. Add tests for new functionality
3. Ensure all tests pass
4. Update CHANGELOG.md if applicable
5. Request review from maintainers
6. Address review feedback

## Testing

### Running Tests

```bash
# All tests
cargo test --all

# Specific crate
cargo test -p genesis-core

# Integration tests
cargo test --test '*'

# With coverage
cargo tarpaulin --all --out Html
```

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange
        let input = ...;

        // Act
        let result = function(input);

        // Assert
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_async_feature() {
        // Test async code
    }
}
```

## Areas Needing Help

We especially welcome contributions in these areas:

- [ ] Core utility implementations
- [ ] Service client implementations
- [ ] Secret type implementations
- [ ] Hook execution system
- [ ] CLI command implementations
- [ ] Documentation
- [ ] Testing
- [ ] Performance optimization

## Questions?

- Open an issue for bugs or feature requests
- Join our Slack channel for discussions
- Check existing issues and pull requests first

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
