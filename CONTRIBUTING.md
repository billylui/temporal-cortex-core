# Contributing to Temporal Cortex Core

Thank you for your interest in contributing! This document provides guidelines for contributing to the project.

## Getting Started

1. Fork the repository
2. Clone your fork locally
3. Create a branch for your changes: `git checkout -b feature/your-feature`
4. Make your changes following the guidelines below
5. Push to your fork and submit a pull request

## Development Setup

### Prerequisites

- **Rust 1.88+** (1.93 recommended) — install via [rustup](https://rustup.rs/)
- **wasm32-unknown-unknown target** — `rustup target add wasm32-unknown-unknown`
- **wasm-bindgen-cli** — `cargo install wasm-bindgen-cli`
- **Node.js 18+** with pnpm — `npm install -g pnpm`
- **Python 3.12+** (optional, for temporal-cortex-toon-python)

### Building

```bash
# Build all Rust crates
cargo build --workspace

# Build WASM bindings
cargo build -p temporal-cortex-toon-wasm -p truth-engine-wasm --target wasm32-unknown-unknown --release

# Install JS dependencies
pnpm install
```

## Test-Driven Development (TDD)

This project strictly follows Red-Green-Refactor:

1. **Red** — Write a failing test that defines expected behavior
2. **Green** — Write the minimum code to make it pass
3. **Refactor** — Clean up while keeping all tests green

Every pull request must include tests for new functionality or bug fixes.

### Running Tests

```bash
# Full test suite
cargo test --workspace

# Code quality (must pass before submitting PR)
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
```

## Code Style

- **Formatting**: We use `rustfmt` with default settings. Run `cargo fmt` before committing.
- **Linting**: All clippy warnings are treated as errors in CI (`-D warnings`). Run `cargo clippy --workspace --all-targets -- -D warnings` locally.
- **Supply chain**: `cargo deny check` audits advisories, licenses, and sources.

## Pull Request Checklist

Before submitting a PR, verify:

- [ ] All tests pass: `cargo test --workspace`
- [ ] Formatting is correct: `cargo fmt --check --all`
- [ ] No clippy warnings: `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] Supply chain check passes: `cargo deny check`
- [ ] New functionality has corresponding tests
- [ ] Documentation is updated for public API changes

## Commit Messages

Use clear, descriptive commit messages:

- `feat: add tabular array support for nested objects`
- `fix: handle empty string quoting in inline arrays`
- `test: add roundtrip tests for Unicode strings`
- `docs: update TOON format specification examples`

## Reporting Issues

- Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.md) for bugs
- Use the [feature request template](.github/ISSUE_TEMPLATE/feature_request.md) for enhancements
- Search existing issues before creating a new one

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project: MIT OR Apache-2.0.
