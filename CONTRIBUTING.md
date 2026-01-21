# Contributing to GeoVeil-MP

Thank you for your interest in contributing to GeoVeil-MP! This document provides guidelines and instructions for contributing.

## Code of Conduct

Please be respectful and constructive in all interactions. We aim to maintain a welcoming environment for all contributors.

## Getting Started

### Prerequisites

- Rust 1.75 or later
- Python 3.8+ (for Python bindings)
- Maturin (for building Python wheels)

### Setting Up Development Environment

```bash
# Clone the repository
git clone https://github.com/miluta7/geoveil-mp.git
cd geoveil-mp

# Build the Rust library
cargo build

# Run tests
cargo test

# Build Python bindings
pip install maturin
maturin develop --features python
```

## How to Contribute

### Reporting Bugs

1. Check existing [issues](https://github.com/miluta7/geoveil-mp/issues) to avoid duplicates
2. Create a new issue with:
   - Clear, descriptive title
   - Steps to reproduce
   - Expected vs actual behavior
   - RINEX file sample (if applicable, anonymized)
   - Environment details (OS, Rust version, Python version)

### Suggesting Features

1. Open an issue with the "enhancement" label
2. Describe the feature and use case
3. Explain why it benefits the project

### Pull Requests

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass: `cargo test`
6. Run clippy: `cargo clippy`
7. Format code: `cargo fmt`
8. Commit with clear messages
9. Push and create a Pull Request

### Commit Messages

Follow conventional commits:
- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `test:` Test additions/changes
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `ci:` CI/CD changes

Example: `feat: add BeiDou B2b frequency support`

## Code Style

### Rust

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Address all `cargo clippy` warnings
- Document public APIs with doc comments
- Add examples in doc comments where helpful

### Python

- Follow PEP 8
- Add type hints where appropriate
- Document functions with docstrings

## Testing

### Running Tests

```bash
# All Rust tests
cargo test

# With logging
RUST_LOG=debug cargo test -- --nocapture

# Specific test
cargo test test_rinex_parsing

# Python tests (after maturin develop)
pytest tests/
```

### Writing Tests

- Add unit tests for new functions
- Add integration tests for new features
- Include edge cases and error conditions
- Use test data from the `tests/data/` directory

## Documentation

- Update README.md for user-facing changes
- Update CHANGELOG.md following Keep a Changelog format
- Add doc comments for public APIs
- Include examples for complex functionality

## Release Process

1. Update version in `Cargo.toml` and `pyproject.toml`
2. Update CHANGELOG.md
3. Create PR with version bump
4. After merge, tag release: `git tag v0.x.x`
5. GitHub Actions will handle crates.io and PyPI publishing

## Areas Where Help is Needed

- [ ] Additional RINEX 4.xx format support
- [ ] More GNSS system support (SBAS types)
- [ ] Performance optimizations
- [ ] Additional Python examples
- [ ] Documentation improvements
- [ ] Test coverage expansion
- [ ] CI/CD improvements

## Questions?

- Open a [Discussion](https://github.com/miluta7/geoveil-mp/discussions)
- Check existing issues and PRs

Thank you for contributing! 🛰️
