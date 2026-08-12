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

# Run the bounded fast feedback suite
test: test-fast

# Run library tests and the smallest stable integration targets
test-fast:
    @echo "Running tests with cargo nextest..."
    cargo nextest run --profile fast --lib \
        --test analyzer_tests --test complexity_tests --test core_metrics_tests \
        --test debt_tests --test entropy_tests

# Backwards-compatible fast plus bounded integration suite
test-safe: test-fast test-integration

# Run the bounded cross-module regression suite
test-integration:
    cargo nextest run --profile integration \
        --test analyzer_tests --test apply_entropy_dampening_tests --test batch_integration \
        --test bug_documentation_tests --test call_graph_closure_test \
        --test call_graph_extraction_test --test call_graph_improved_test \
        --test call_graph_resolution_test --test call_graph_cross_file_resolution_test \
        --test cognitive_complexity_tests --test complexity_comparison_test \
        --test complexity_module_tests --test complexity_tests \
        --test context_aware_integration_test --test context_aware_test --test core_ast_tests \
        --test core_display_tests --test core_metrics_tests --test core_monadic_tests \
        --test core_untested_functions_tests --test cyclomatic_complexity_tests \
        --test debt_grouping_tests --test debt_tests --test dependency_and_coupling_tests \
        --test demo_library_api_test --test determinism_regression_test \
        --test entropy_integration_tests --test entropy_tests \
        --test error_swallowing_pipeline_test --test error_swallowing_test \
        --test external_api_detection_tests --test external_api_detector_integration_test \
        --test false_positive_reproduction_tests --test fast_unit_tests \
        --test field_access_chain_test --test io_walker_tests --test json_serialization_test \
        --test language_tests --test python_complexity_tests --test python_extraction_test \
        --test solidity_analyzer_tests --test suppression_tests --test token_classification_tests \
        --test validate_improvement_integration_test

# Run tests with output
test-verbose:
    cargo nextest run --nocapture

# Run tests with specific pattern
test-pattern PATTERN:
    cargo nextest run {{PATTERN}}

# Run tests and watch for changes
test-watch:
    cargo watch -x 'nextest run'

# Run fast coverage using the default local test scope
coverage: coverage-fast

# Run fast coverage using cargo-llvm-cov's low-profile-count libtest harness
coverage-fast:
    #!/usr/bin/env bash
    set -euo pipefail
    # Ensure rustup's cargo is in PATH (needed for llvm-tools-preview)
    export PATH="$HOME/.cargo/bin:$PATH"
    mkdir -p target/coverage
    cargo llvm-cov clean --profraw-only
    echo "Generating fast HTML coverage report with cargo-llvm-cov..."
    cargo llvm-cov --no-clean --html --output-dir target/coverage \
        --lib --test analyzer_tests --test complexity_tests --test core_metrics_tests \
        --test debt_tests --test entropy_tests --test parallel_unified_analysis_test \
        --test cli_output_format_integration_test --test batch_integration \
        --test call_graph_cross_file_resolution_test --test call_graph_comprehensive_test \
        --test risk_analysis_tests --test integrated_analysis \
        --test validate_improvement_integration_test --test output_validation_test \
        --test solidity_analyzer_tests --test python_extraction_test \
        --test risk_context_tests -- --quiet
    echo "Coverage report generated at target/coverage/html/index.html"

# Run tests with coverage (lcov format)
coverage-lcov: coverage-fast-lcov

# Run fast tests with coverage (lcov format)
coverage-fast-lcov:
    #!/usr/bin/env bash
    set -euo pipefail  # Exit on error, undefined variables, and pipe failures
    # Ensure rustup's cargo is in PATH (needed for llvm-tools-preview)
    export PATH="$HOME/.cargo/bin:$PATH"
    # Ensure target/coverage directory exists
    mkdir -p target/coverage
    cargo llvm-cov clean --profraw-only
    # Use the default libtest harness for LCOV: nextest is fast at running tests,
    # but source coverage creates one raw profile per test process, which makes
    # LLVM's final merge/export step dominate this suite.
    echo "Generating LCOV report with cargo-llvm-cov..."
    cargo llvm-cov --no-clean --lcov --output-path target/coverage/lcov.info \
        --lib --test analyzer_tests --test complexity_tests --test core_metrics_tests \
        --test debt_tests --test entropy_tests --test parallel_unified_analysis_test \
        --test cli_output_format_integration_test --test batch_integration \
        --test call_graph_cross_file_resolution_test --test call_graph_comprehensive_test \
        --test risk_analysis_tests --test integrated_analysis \
        --test validate_improvement_integration_test --test output_validation_test \
        --test solidity_analyzer_tests --test python_extraction_test \
        --test risk_context_tests -- --quiet
    echo "Coverage report generated at target/coverage/lcov.info"
    # Verify the file was actually created
    if [ ! -f target/coverage/lcov.info ]; then
        echo "ERROR: Coverage file was not generated at target/coverage/lcov.info"
        exit 1
    fi

# Run tests with coverage and check threshold
coverage-check:
    #!/usr/bin/env bash
    set -euo pipefail
    # Ensure rustup's cargo is in PATH (needed for llvm-tools-preview)
    export PATH="$HOME/.cargo/bin:$PATH"
    mkdir -p target/coverage
    cargo llvm-cov clean --profraw-only
    echo "Checking fast line coverage threshold..."
    cargo llvm-cov --no-clean --json --summary-only --output-path target/coverage/coverage.json \
        --lib --test analyzer_tests --test complexity_tests --test core_metrics_tests \
        --test debt_tests --test entropy_tests --test parallel_unified_analysis_test \
        --test cli_output_format_integration_test --test batch_integration \
        --test call_graph_cross_file_resolution_test --test call_graph_comprehensive_test \
        --test risk_analysis_tests --test integrated_analysis \
        --test validate_improvement_integration_test --test output_validation_test \
        --test solidity_analyzer_tests --test python_extraction_test \
        --test risk_context_tests -- --quiet
    COVERAGE=$(jq -r '.data[0].totals.lines.percent' target/coverage/coverage.json)
    echo "Current coverage: ${COVERAGE}%"
    if (( $(echo "$COVERAGE < 80" | bc -l) )); then
        echo "⚠️  Coverage is below 80%: $COVERAGE%"
        exit 1
    else
        echo "✅ Coverage meets 80% threshold: $COVERAGE%"
    fi

# Run exhaustive coverage using all feature combinations in the default cargo test harness
coverage-full:
    #!/usr/bin/env bash
    set -euo pipefail
    # Ensure rustup's cargo is in PATH (needed for llvm-tools-preview)
    export PATH="$HOME/.cargo/bin:$PATH"
    echo "Building debtmap binary for integration tests..."
    cargo build --bin debtmap
    echo "Cleaning previous coverage data..."
    cargo llvm-cov clean
    echo "Generating full HTML coverage report with cargo-llvm-cov..."
    cargo llvm-cov --all-features --html --output-dir target/coverage
    echo "Coverage report generated at target/coverage/html/index.html"

# Run exhaustive tests with coverage (lcov format)
coverage-full-lcov:
    #!/usr/bin/env bash
    set -euo pipefail
    # Ensure rustup's cargo is in PATH (needed for llvm-tools-preview)
    export PATH="$HOME/.cargo/bin:$PATH"
    echo "Building debtmap binary for integration tests..."
    cargo build --bin debtmap
    echo "Cleaning previous coverage data..."
    cargo llvm-cov clean
    mkdir -p target/coverage
    echo "Generating full LCOV report with cargo-llvm-cov..."
    cargo llvm-cov --all-features --lcov --output-path target/coverage/lcov.info
    echo "Coverage report generated at target/coverage/lcov.info"
    if [ ! -f target/coverage/lcov.info ]; then
        echo "ERROR: Coverage file was not generated at target/coverage/lcov.info"
        exit 1
    fi

# Run exhaustive tests with coverage and check threshold
coverage-full-check:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Building debtmap binary for integration tests..."
    cargo build --bin debtmap
    echo "Setting up LLVM tools..."
    RUSTUP_TOOLCHAIN=$(rustup show active-toolchain | cut -d' ' -f1)
    LLVM_DIR=$(rustup which rustc | xargs dirname | xargs dirname)/lib/rustlib/$(rustc -vV | grep host | cut -d' ' -f2)/bin
    export LLVM_PROFDATA="$LLVM_DIR/llvm-profdata"
    export LLVM_COV="$LLVM_DIR/llvm-cov"
    echo "Checking code coverage threshold..."
    cargo llvm-cov clean
    cargo llvm-cov --all-features --json --output-path target/coverage/coverage.json
    COVERAGE=$(cat target/coverage/coverage.json | jq -r '.data[0].totals.lines.percent')
    echo "Current coverage: ${COVERAGE}%"
    if (( $(echo "$COVERAGE < 80" | bc -l) )); then
        echo "⚠️  Coverage is below 80%: $COVERAGE%"
        exit 1
    else
        echo "✅ Coverage meets 80% threshold: $COVERAGE%"
    fi

# Open coverage report in browser
coverage-open: coverage
    open target/coverage/html/index.html

# Analyze the current repository with debtmap using coverage data
analyze-self:
    #!/usr/bin/env bash
    echo "Building debtmap..."
    cargo build --bin debtmap
    echo "Setting up LLVM tools..."
    RUSTUP_TOOLCHAIN=$(rustup show active-toolchain | cut -d' ' -f1)
    LLVM_DIR=$(rustup which rustc | xargs dirname | xargs dirname)/lib/rustlib/$(rustc -vV | grep host | cut -d' ' -f2)/bin
    export LLVM_PROFDATA="$LLVM_DIR/llvm-profdata"
    export LLVM_COV="$LLVM_DIR/llvm-cov"
    echo "Generating code coverage (lcov format)..."
    cargo llvm-cov clean
    cargo llvm-cov --all-features --lcov --output-path target/coverage/lcov.info
    echo "Analyzing current repository with debtmap..."
    ./target/debug/debtmap analyze . --lcov target/coverage/lcov.info -vv
    echo "Analysis complete!"

# Run property-based tests only (if using proptest)
test-prop:
    cargo nextest run prop

# Run bounded CLI/output tests serially against Cargo's prebuilt debug binary
test-cli:
    cargo nextest run --profile cli \
        --test clean_dispatcher_output_test --test cli_output_format_integration_test \
        --test parallel_sequential_equivalence_test \
        --test progress_display_integration_test --test validate_parallel_test

# Run benchmarks
bench:
    cargo bench

# Run bounded slow repository/context regressions
test-slow:
    cargo nextest run --profile slow --run-ignored only --lib \
        --test god_object_detection_test --test integration_false_positive_test \
        test_git_history_on_real_repo test_git_history_with_analysis_style_paths \
        test_git_history_via_context_aggregator test_large_call_graph_no_stack_overflow \
        test_context_aggregator_large_codebase test_real_project_health_score \
        test_detects_rust_call_graph_scenario test_spec_130_god_class_vs_god_file_detection \
        test_context_aware_on_entire_codebase
    cargo nextest run --profile slow --run-ignored only --test ui_tests

# Run bounded scale and timing tests
test-stress:
    cargo nextest run --profile stress --run-ignored only --lib \
        test_analyze_many_functions_with_context_no_stack_overflow
    cargo nextest run --profile stress --run-ignored only \
        --test call_graph_debug_output_test --test call_graph_stress_test \
        --test demo_library_api_test --test functional_composition_validation_test \
        --test parallel_unified_analysis_test --test stress_test_large_projects
    cargo nextest run --profile stress \
        --test boilerplate_performance_test --test coverage_performance_regression_test

# Run opt-in tests that depend on terminal state, binaries, or local artifacts
test-environment:
    cargo nextest run --profile environment --run-ignored only --lib \
        --test trait_method_coverage_test \
        test_editor_command_construction test_execute_detail_action_open_in_editor \
        test_coverage_matching_integration test_explain_coverage_finds_trait_method

# Run only ignored tests that document unfinished behavior
test-known-broken:
    cargo nextest run --profile known-broken --run-ignored only --lib \
        --test clustering_integration --test god_object_config_rs_test \
        --test inter_procedural_purity_test --test tier_aware_filtering_integration_test \
        test_simple_import_resolution test_select_macro test_clustering_determinism \
        test_god_object_detection_on_config_rs test_impure_caller_propagates_impurity \
        test_all_error_swallowing_patterns_visible_with_defaults test_t1_bypasses_high_threshold

# Run every supported bounded group expected to pass in a configured environment
test-supported: test-fast test-integration test-cli test-slow test-stress

# Compatibility name for all ignored dispositions; diagnostics may fail by design
test-ignored: test-slow test-stress test-environment test-known-broken
test-perf: test-stress

# Exhaustive bounded run; environment and known-broken diagnostics may fail by design
test-all: test-supported test-environment test-known-broken

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
ci: test-fast test-integration test-cli
    @echo "Running CI checks (matching GitHub Actions)..."
    @echo "Setting environment variables..."
    @export CARGO_TERM_COLOR=always && \
     export CARGO_INCREMENTAL=0 && \
     export RUSTFLAGS="-Dwarnings" && \
     export RUST_BACKTRACE=1 && \
     echo "Running doc tests..." && \
     cargo test --doc && \
     echo "Running clippy..." && \
     cargo clippy --all-targets --all-features -- -D warnings && \
     echo "Checking formatting..." && \
     cargo fmt --all -- --check && \
     echo "Checking documentation..." && \
     cargo doc --no-deps --document-private-items && \
     echo "All CI checks passed!"

# Full CI build pipeline (equivalent to scripts/ci-build.sh)
ci-build: test-fast test-integration test-cli
    @echo "Building debtmap..."
    @echo "Checking code formatting..."
    cargo fmt --all -- --check
    @echo "Running clippy..."
    cargo clippy --all-targets --all-features -- -D warnings
    @echo "Building project..."
    cargo build
    @echo "Checking benchmarks..."
    cargo check --benches
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
    rustup component add rustfmt clippy llvm-tools-preview
    cargo install cargo-watch cargo-llvm-cov cargo-audit cargo-outdated cargo-nextest

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
    #!/usr/bin/env bash
    set -euo pipefail

    profiler=""
    if command -v perf >/dev/null 2>&1; then
        profiler="perf"
    elif [[ "$(uname -s)" == "Darwin" ]]; then
        if command -v samply >/dev/null 2>&1; then
            profiler="samply"
        else
            echo "No supported profiler found for macOS."
            echo "Install samply with: cargo install samply"
            echo "Then rerun: just profile"
            exit 127
        fi
    else
        echo "No supported profiler found."
        echo "Install Linux perf or, on macOS, install samply with: cargo install samply"
        exit 127
    fi

    cargo build --release

    binary="./target/release/$(basename "$PWD")"
    if [[ "$profiler" == "perf" ]]; then
        perf record --call-graph=dwarf "$binary"
        perf report
    elif [[ "$profiler" == "samply" ]]; then
        samply record "$binary"
    fi

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
