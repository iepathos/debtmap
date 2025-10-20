# Contributing to Debtmap

Thank you for your interest in contributing to Debtmap! This guide will help you get started with development, testing, and submitting contributions.

## Getting Started

### Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** (latest stable version): Install via [rustup](https://rustup.rs/)
- **Just** (command runner): Install with `cargo install just`
- **Git**: For version control

Optional but recommended:
- **cargo-nextest**: Faster test runner - `cargo install cargo-nextest`
- **cargo-tarpaulin**: Code coverage - `cargo install cargo-tarpaulin`
- **cargo-watch**: Auto-rebuild on file changes - `cargo install cargo-watch`

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/iepathos/debtmap.git
cd debtmap

# Build the project
cargo build

# Run tests to verify everything works
just test

# Try analyzing debtmap itself
cargo build --bin debtmap
./target/debug/debtmap analyze . --top 10
```

## Development Workflow

### Using Just Commands

Debtmap uses [Just](https://github.com/casey/just) for common development tasks. Run `just` or `just --list` to see all available commands:

```bash
# Development
just dev            # Run in development mode
just watch          # Run with hot reloading
just build          # Build the project

# Testing
just test           # Run all tests with nextest
just test-verbose   # Run tests with output
just coverage       # Generate coverage report
just analyze-self   # Analyze debtmap with coverage

# Code Quality
just fmt            # Format code with rustfmt
just lint           # Run clippy linter
just check          # Quick syntax check

# CI Simulation
just ci             # Run all CI checks locally
just pre-commit     # Run pre-commit checks
```

### Feature Branch Workflow

1. **Create a feature branch** from `master`:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following the code style guidelines below

3. **Run tests and linters**:
   ```bash
   just fmt
   just lint
   just test
   ```

4. **Commit your changes** with clear, descriptive messages:
   ```bash
   git add .
   git commit -m "Add feature: description of what changed"
   ```

5. **Push to your fork** and create a pull request

## Code Style

Debtmap follows functional programming principles and Rust best practices. Please review [CLAUDE.md](CLAUDE.md) for detailed coding guidelines.

### Key Principles

- **Functional-first design**: Prefer pure functions over stateful methods
- **Immutable data structures**: Use the `im` crate for persistent collections
- **Composable pipelines**: Chain transformations using iterators
- **Error handling**: Use `Result<T>` with the `?` operator and `anyhow::Context`
- **Maximum function length**: 20 lines (prefer 5-10)
- **Maximum cyclomatic complexity**: 5

### Formatting and Linting

Before committing, always run:

```bash
just fmt    # Auto-format code with rustfmt
just lint   # Check for clippy warnings (must pass with -D warnings)
```

**Required standards:**
- All code must be formatted with `cargo fmt`
- All clippy warnings must be addressed (no warnings allowed)
- Public APIs must have documentation comments
- Complex logic should have inline comments explaining "why"

### Example: Good Function Design

```rust
// Good: Pure, composable, single responsibility
fn calculate_complexity_score(metrics: &FunctionMetrics) -> f64 {
    let cyclomatic_weight = metrics.cyclomatic as f64 / 10.0;
    let cognitive_weight = metrics.cognitive as f64 / 20.0;
    (cyclomatic_weight + cognitive_weight) * 5.0
}

// Good: Clear error handling
fn parse_lcov_file(path: &Path) -> Result<CoverageData> {
    let content = fs::read_to_string(path)
        .context("Failed to read LCOV file")?;

    parse_lcov_content(&content)
        .context("Failed to parse LCOV content")
}

// Avoid: Long, complex functions with mixed concerns
fn bad_example(data: Vec<u8>) -> Option<String> {
    // Don't do this - mixes parsing, validation, I/O, and error handling
    // in one large function with unclear error handling
    ...
}
```

## Testing Requirements

All new features and bug fixes must include tests.

### Writing Tests

- **Unit tests**: Place in `#[cfg(test)]` modules in the same file as the code
- **Integration tests**: Add to `tests/` directory
- **Test naming**: Use descriptive names that explain what's being tested

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complexity_score_increases_with_cyclomatic() {
        let metrics = FunctionMetrics {
            cyclomatic: 10,
            cognitive: 5,
            ..Default::default()
        };
        let score = calculate_complexity_score(&metrics);
        assert!(score > 0.0);
    }

    #[test]
    fn lcov_parser_handles_empty_file() {
        let result = parse_lcov_content("");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().functions.len(), 0);
    }
}
```

### Running Tests

```bash
# Run all tests
just test

# Run with verbose output
just test-verbose

# Run specific test pattern
just test-pattern "complexity"

# Run with coverage
just coverage
just coverage-open  # Opens HTML report
```

### Test Coverage Expectations

- Aim for **85%+ code coverage** for new features
- Critical paths (scoring, analysis) should have **95%+ coverage**
- Pure functions are easy to test - no excuses for low coverage
- Integration tests should cover end-to-end workflows

## Pull Request Process

### Before Submitting

1. **Ensure all tests pass**:
   ```bash
   just ci  # Runs full CI suite locally
   ```

2. **Update documentation** if you've changed APIs or added features

3. **Add yourself to contributors** (if this is your first PR)

### PR Guidelines

- **Title**: Clear, descriptive summary of changes
- **Description**: Explain what changed and why
  - Link to related issues: `Fixes #123` or `Relates to #456`
  - Describe the problem being solved
  - Explain your approach
  - Note any breaking changes

- **Commits**:
  - Use clear, descriptive commit messages
  - Follow conventional commits if possible: `feat:`, `fix:`, `docs:`, `test:`
  - Each commit should be a logical, atomic change

### Review Process

- Maintainers will review your PR and may request changes
- Address feedback by pushing new commits (don't force-push)
- Once approved, a maintainer will merge your PR
- PRs require:
  - All CI checks passing (tests, linting, formatting)
  - At least one maintainer approval
  - No merge conflicts with `master`

## Architecture Overview

Debtmap is organized into focused modules with clear responsibilities:

```
src/
├── analyzers/      # Language-specific AST parsers
├── core/           # Core data types and utilities
├── debt/           # Technical debt pattern detection
├── complexity/     # Complexity metrics calculation
├── risk/           # Risk analysis and coverage integration
├── io/             # File I/O and output formatting
└── testing/        # Test analysis and quality metrics
```

For detailed architecture documentation, see the [Architecture Guide](https://iepathos.github.io/debtmap/architecture.html).

### Adding Language Support

To add support for a new language:

1. Create a new analyzer in `src/analyzers/<language>.rs`
2. Implement the `LanguageAnalyzer` trait
3. Add parser integration (tree-sitter recommended)
4. Write comprehensive tests in `tests/<language>_tests.rs`
5. Update documentation

Example structure:

```rust
// src/analyzers/go.rs
pub struct GoAnalyzer;

impl LanguageAnalyzer for GoAnalyzer {
    fn parse_file(&self, path: &Path) -> Result<FileMetrics> {
        // Parse Go AST and extract metrics
    }

    fn calculate_complexity(&self, ast: &GoAst) -> ComplexityMetrics {
        // Calculate cyclomatic and cognitive complexity
    }
}
```

## Finding Good First Issues

New to Debtmap? Look for issues labeled:

- `good-first-issue` - Beginner-friendly issues
- `help-wanted` - Issues where contributions are especially welcome
- `documentation` - Improve docs (great for first-time contributors)

### Recommended First Contributions

- **Documentation improvements**: Fix typos, clarify confusing sections, add examples
- **Test coverage**: Add tests for untested code paths
- **Bug fixes**: Start with issues tagged `bug` and `good-first-issue`
- **New detectors**: Add new technical debt pattern detectors
- **Performance**: Optimize hot paths identified by profiling

## Communication

### Where to Ask Questions

- **GitHub Discussions**: For general questions, ideas, and discussions
- **GitHub Issues**: For bug reports and feature requests
- **Pull Requests**: For code review and implementation discussions

### Getting Help

Stuck? Don't hesitate to ask:

1. Check the [documentation](https://iepathos.github.io/debtmap/)
2. Search existing issues and discussions
3. Open a new discussion or issue with:
   - What you're trying to do
   - What you've tried so far
   - Any error messages or unexpected behavior

## Development Tips

### Performance Profiling

```bash
# Build with debug symbols
cargo build --release --profile release-with-debug

# Profile with perf (Linux)
perf record --call-graph=dwarf ./target/release/debtmap analyze .
perf report

# Use cargo-flamegraph
cargo install flamegraph
cargo flamegraph -- analyze .
```

### Debugging

```bash
# Run with debug logging
RUST_LOG=debug cargo run -- analyze .

# Run with backtrace
RUST_BACKTRACE=1 cargo run -- analyze .

# Use cargo-expand to debug macros
cargo expand
```

### Useful Development Commands

```bash
# Check for unused dependencies
just unused-deps

# Find security vulnerabilities
just audit

# Check for duplicate dependencies
just duplicate-deps

# Generate and open documentation
just doc
```

## Functional Programming in Rust

Debtmap emphasizes functional programming patterns. When contributing, prefer:

### Iterators over loops

```rust
// Good
let scores: Vec<f64> = functions
    .iter()
    .map(|f| calculate_score(f))
    .collect();

// Avoid
let mut scores = Vec::new();
for func in &functions {
    scores.push(calculate_score(func));
}
```

### Immutable transformations

```rust
// Good - returns new data
fn add_coverage(mut metrics: FileMetrics, coverage: f64) -> FileMetrics {
    metrics.coverage = coverage;
    metrics
}

// Avoid - mutates in place
fn add_coverage_mut(metrics: &mut FileMetrics, coverage: f64) {
    metrics.coverage = coverage;
}
```

### Pure functions

```rust
// Good - pure function, easy to test
fn calculate_risk_score(complexity: u32, coverage: f64) -> f64 {
    let complexity_factor = complexity as f64 / 10.0;
    let coverage_factor = 1.0 - coverage;
    complexity_factor * coverage_factor * 10.0
}

// Avoid - side effects, harder to test
fn calculate_risk_score_bad(func: &Function) -> f64 {
    println!("Calculating for {}", func.name); // Side effect!
    // ... calculation
}
```

For complete guidelines, see [CLAUDE.md](CLAUDE.md) in the repository root.

## License

By contributing to Debtmap, you agree that your contributions will be licensed under the MIT License.

## Code of Conduct

Please note that this project is released with a [Code of Conduct](CODE_OF_CONDUCT.md). By participating in this project you agree to abide by its terms.

## Thank You!

Your contributions make Debtmap better for everyone. Whether you're fixing bugs, adding features, improving documentation, or helping others in discussions - thank you for being part of the community!

If you have questions or need help getting started, don't hesitate to reach out through GitHub Discussions or Issues. We're here to help!
