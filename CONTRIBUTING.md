# Contributing to Torchforge Data

Thank you for your interest in contributing to Torchforge Data! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- Rust 1.85 or higher
- Git

### Setup

1. Fork the repository
2. Clone your fork locally
3. Create a new branch for your feature or bug fix
4. Make your changes
5. Ensure all tests pass
6. Submit a pull request

## Development Workflow

### Running Tests

```bash
cargo test
```

### Code Formatting

We use `rustfmt` for consistent code formatting:

```bash
cargo fmt
```

### Linting

We use `clippy` for linting:

```bash
cargo clippy -- -D warnings
```

## Pull Request Process

1. Update the CHANGELOG.md with your changes
2. Ensure your code follows the project's style guidelines
3. Add tests for new functionality
4. Run the full test suite
5. Submit a pull request with a clear description

## Code Style

- Follow Rust idiomatic style
- Use meaningful variable and function names
- Add documentation comments for public APIs
- Keep functions focused and small

## Reporting Issues

When reporting issues, please include:

- Rust version
- Operating system
- Steps to reproduce
- Expected vs actual behavior
- Any relevant logs or error messages

## Changelog Policy

All changes that affect user-facing behavior must be documented in `CHANGELOG.md`.

### When to Add Entries

Every pull request that meets any of these criteria requires a CHANGELOG entry under the `[Unreleased]` section:

- **Features**: New functionality or capabilities
- **Bug fixes**: Resolved issues that affected users
- **Breaking changes**: API modifications that break existing code
- **Deprecations**: Features that will be removed in future releases
- **Security**: Vulnerability fixes or security improvements

### Entry Format

Entries should follow this format:
```
### Category
- Brief description of the change
```

Categories: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`

### Release Process

When making a release:
1. Move all `[Unreleased]` entries to a new version section: `[x.y.z] — YYYY-MM-DD`
2. Create a new empty `[Unreleased]` section
3. Update the comparison links at the bottom

### Examples

Good:
```
### Added
- Streaming data loader with zero-copy optimization
- Support for custom data transforms

### Fixed
- Memory leak in batch processing
- Panic on empty dataset
```

Bad:
```
### Added
- Fixed some bugs and added stuff
```

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 license.

### License Header Policy

This project does **not** use per-file SPDX license headers in source files. Instead, licensing information is maintained centrally:

- The project license is specified in the root `LICENSE` file (Apache-2.0 full text)
- The SPDX identifier `Apache-2.0` is declared in `Cargo.toml`
- This approach keeps the codebase clean and avoids header maintenance overhead

Contributors should not add license headers to individual source files.
