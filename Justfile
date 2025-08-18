# Debtmap - Justfile
# Quick development commands for Rust projects

# Default recipe - show available commands
default:
    @just --list

# Development commands
alias d := dev
alias r := run
alias t := test
alias c := check
alias f := fmt
alias l := lint

# === DEVELOPMENT ===

# Run the project in development mode
dev:
    cargo run

# Run the project with hot reloading
watch:
    cargo watch -x run

# Run the project in release mode
run:
    cargo run --release

# Run with all features enabled
run-all:
    cargo run --all-features

# === BUILDING ===

# Build the project
build:
    cargo build

# Build in release mode
build-release:
    cargo build --release

# Build with optimizations for native CPU
build-native:
    RUSTFLAGS="-C target-cpu=native" cargo build --release

# Clean build artifacts
clean:
    cargo clean

# === TESTING ===

# Run all tests
test:
    cargo build
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run tests with specific pattern
test-pattern PATTERN:
    cargo test {{PATTERN}}

# Run tests and watch for changes
test-watch:
    cargo watch -x test

# Run tests with coverage using cargo-tarpaulin
coverage:
    #!/usr/bin/env bash
    echo "Building debtmap binary for integration tests..."
    cargo build --bin debtmap
    echo "Generating code coverage report with cargo-tarpaulin..."
    cargo tarpaulin --config .tarpaulin.toml --out Html --out Json --output-dir target/coverage
    echo "Coverage report generated at target/coverage/tarpaulin-report.html"

# Run tests with coverage (lcov format)
coverage-lcov:
    #!/usr/bin/env bash
    echo "Building debtmap binary for integration tests..."
    cargo build --bin debtmap
    echo "Generating code coverage report with cargo-tarpaulin (lcov format)..."
    cargo tarpaulin --config .tarpaulin.toml --out Lcov --output-dir target/coverage
    echo "Coverage report generated at target/coverage/lcov.info"

# Run tests with coverage and check threshold
coverage-check:
    #!/usr/bin/env bash
    echo "Building debtmap binary for integration tests..."
    cargo build --bin debtmap
    echo "Checking code coverage threshold..."
    COVERAGE=$(cargo tarpaulin --config .tarpaulin.toml --out Json --output-dir target/coverage --quiet | jq -r '.files | to_entries | map(.value.coverage) | add / length')
    echo "Current coverage: ${COVERAGE}%"
    if (( $(echo "$COVERAGE < 80" | bc -l) )); then
        echo "⚠️  Coverage is below 80%: $COVERAGE%"
        exit 1
    else
        echo "✅ Coverage meets 80% threshold: $COVERAGE%"
    fi

# Open coverage report in browser
coverage-open: coverage
    open target/coverage/tarpaulin-report.html

# Analyze the current repository with debtmap using coverage data
analyze-self:
    #!/usr/bin/env bash
    echo "Building debtmap in release mode..."
    cargo build --release --bin debtmap
    echo "Generating code coverage (lcov format)..."
    cargo tarpaulin --config .tarpaulin.toml --out Lcov --output-dir target/coverage
    echo "Analyzing current repository with debtmap..."
    ./target/release/debtmap analyze . --lcov target/coverage/lcov.info --context-aware -vv
    echo "Analysis complete!"

# Run property-based tests only (if using proptest)
test-prop:
    cargo test prop

# Run integration tests only
test-integration:
    cargo test --test '*'

# Run benchmarks
bench:
    cargo bench

# Run ignored tests (including performance tests)
test-ignored:
    cargo test -- --ignored

# Run performance tests only
test-perf:
    cargo test -- --ignored perf

# Run all tests including ignored ones
test-all:
    cargo test -- --include-ignored

# === CODE QUALITY ===

# Format code
fmt:
    cargo fmt

# Check formatting without making changes
fmt-check:
    cargo fmt --check

# Run clippy linter
lint:
    cargo clippy -- -D warnings

# Run clippy with all targets
lint-all:
    cargo clippy --all-targets --all-features -- -D warnings

# Quick check without building
check:
    cargo check

# Check all targets and features
check-all:
    cargo check --all-targets --all-features

# Fix automatically fixable lints
fix:
    cargo fix --allow-dirty

# === DOCUMENTATION ===

# Generate and open documentation
doc:
    cargo doc --open

# Generate documentation for all dependencies
doc-all:
    cargo doc --all --open

# Check documentation for errors
doc-check:
    cargo doc --no-deps

# === DEPENDENCIES ===

# Update dependencies
update:
    cargo update

# Audit dependencies for security vulnerabilities
audit:
    cargo audit

# Check for outdated dependencies
outdated:
    cargo outdated

# Add a new dependency
add CRATE:
    cargo add {{CRATE}}

# Add a development dependency
add-dev CRATE:
    cargo add --dev {{CRATE}}

# Remove a dependency
remove CRATE:
    cargo remove {{CRATE}}

# === UTILITY ===

# Show project tree structure
tree:
    tree -I 'target|node_modules'

# Show git status
status:
    git status

# Create a new module
new-module NAME:
    mkdir -p src/{{NAME}}
    echo "//! {{NAME}} module" > src/{{NAME}}/mod.rs
    echo "pub mod {{NAME}};" >> src/lib.rs

# Create a new integration test
new-test NAME:
    echo "//! Integration test for {{NAME}}" > tests/{{NAME}}.rs

# Create a new example
new-example NAME:
    echo "//! Example: {{NAME}}" > examples/{{NAME}}.rs

# === CI/CD SIMULATION ===

# Run all CI checks locally (matches GitHub Actions)
ci:
    @echo "Running CI checks (matching GitHub Actions)..."
    @echo "Setting environment variables..."
    @export CARGO_TERM_COLOR=always && \
     export CARGO_INCREMENTAL=0 && \
     export RUSTFLAGS="-Dwarnings" && \
     export RUST_BACKTRACE=1 && \
     echo "Running tests..." && \
     cargo test --all-features && \
     echo "Running clippy..." && \
     cargo clippy --all-targets --all-features -- -D warnings && \
     echo "Checking formatting..." && \
     cargo fmt --all -- --check && \
     echo "Checking documentation..." && \
     cargo doc --no-deps --document-private-items && \
     echo "All CI checks passed!"

# Run compatibility tests only
test-compatibility:
    cargo test --test compatibility -- --test-threads=1

# Run performance tests only  
test-performance:
    cargo test --test performance

# Full CI build pipeline (equivalent to scripts/ci-build.sh)
ci-build:
    @echo "Building debtmap..."
    @echo "Checking code formatting..."
    cargo fmt --all -- --check
    @echo "Running clippy..."
    cargo clippy --all-targets --all-features -- -D warnings
    @echo "Building project..."
    cargo build --release
    @echo "Running tests..."
    cargo test --all
    @echo "Building benchmarks..."
    cargo bench --no-run
    @echo "Build successful!"

# Pre-commit hook simulation
pre-commit: fmt lint test
    @echo "Pre-commit checks passed!"

# Full development cycle check
full-check: clean build test lint doc audit
    @echo "Full development cycle completed successfully!"

# === INSTALLATION ===

# Install development tools
install-tools:
    rustup component add rustfmt clippy
    cargo install cargo-watch cargo-tarpaulin cargo-audit cargo-outdated

# Install additional development tools
install-extras:
    cargo install cargo-expand cargo-machete cargo-deny cargo-udeps

# Install git hooks
install-hooks:
    #!/usr/bin/env bash
    echo "Installing git hooks..."
    for hook in git-hooks/*; do
        if [ -f "$hook" ]; then
            hook_name=$(basename "$hook")
            cp "$hook" ".git/hooks/$hook_name"
            chmod +x ".git/hooks/$hook_name"
            echo "  ✓ Installed $hook_name"
        fi
    done
    echo "Git hooks installed successfully!"

# === RELEASE ===

# Prepare for release (dry run)
release-check:
    cargo publish --dry-run

# Create a new release (requires manual version bump)
release:
    cargo publish

# === ADVANCED ===

# Profile the application
profile:
    cargo build --release
    perf record --call-graph=dwarf ./target/release/$(basename $(pwd))
    perf report

# Expand macros for debugging
expand:
    cargo expand

# Find unused dependencies
unused-deps:
    cargo machete

# Security-focused dependency check
security-check:
    cargo deny check

# Find duplicate dependencies
duplicate-deps:
    cargo tree --duplicates

# === HELP ===

# Show detailed help for cargo commands
help:
    @echo "Cargo commands reference:"
    @echo "  cargo run      - Run the project"
    @echo "  cargo test     - Run tests"
    @echo "  cargo build    - Build the project"
    @echo "  cargo fmt      - Format code"
    @echo "  cargo clippy   - Run linter"
    @echo "  cargo check    - Quick syntax check"
    @echo "  cargo doc      - Generate documentation"
    @echo ""
    @echo "Use 'just <command>' for convenience aliases!"
