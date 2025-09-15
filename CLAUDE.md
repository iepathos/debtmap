# Debtmap Development Guidelines

## Project Context

Debtmap is a code complexity and technical debt analyzer written in Rust. It analyzes Rust, Python, JavaScript, and TypeScript codebases to identify complexity patterns, technical debt, and architectural issues.

## Philosophy

### Core Beliefs

- **Functional-first design** - Pure functions with minimal side effects
- **Immutable data structures** - Use `im` crate for persistent collections
- **Composable analysis pipelines** - Chain pure transformations
- **Incremental progress over big bangs** - Small, testable changes
- **Type-driven development** - Let the type system guide correctness
- **Performance through parallelism** - Use `rayon` for data parallelism

### Rust-Specific Principles

#### Functional Programming in Rust

- **Prefer iterators over loops** - Use `.map()`, `.filter()`, `.fold()` for transformations
- **Leverage trait composition** - Build complex behavior through trait bounds
- **Minimize mutable state** - Use `Cow<'_, T>` or immutable builders
- **Pure computation cores** - Separate parsing, analysis, and I/O concerns
- **Result chains** - Use `?` operator and monadic patterns for error handling

#### Memory and Performance

- **Zero-copy where possible** - Use string slices and borrowed data
- **Parallel processing** - Use `rayon::par_iter()` for CPU-intensive analysis
- **Lazy evaluation** - Compute only what's needed when needed
- **Efficient collections** - Use `dashmap::DashMap` for concurrent access

#### Type System Leverage

- **Phantom types** - Encode state transitions in types
- **Builder patterns** - Ensure correct construction through types
- **Exhaustive pattern matching** - Handle all enum variants explicitly
- **Newtype wrappers** - Prevent mixing incompatible values

## Architecture Patterns

### Module Organization

```
src/
├── core/           # Core data types and utilities
├── analyzers/      # Language-specific parsers and analyzers
├── analysis/       # Advanced analysis algorithms
├── debt/           # Technical debt detection patterns
├── complexity/     # Complexity calculation algorithms
├── risk/           # Risk assessment and prioritization
├── io/             # Input/output formatting and handling
└── testing/        # Test analysis and quality metrics
```

### Data Flow Architecture

1. **Parser Layer** - Language-specific AST generation (pure)
2. **Analysis Layer** - Metric computation and pattern detection (pure)
3. **Aggregation Layer** - Combine results across files (functional)
4. **Output Layer** - Format and emit results (I/O boundary)

### Functional Composition Patterns

```rust
// Good: Functional pipeline
fn analyze_project(config: &Config) -> Result<AnalysisResults> {
    discover_files(config)?
        .into_par_iter()
        .map(|path| parse_file(&path))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .map(|ast| analyze_complexity(&ast))
        .fold(AnalysisResults::empty(), |acc, result| acc.merge(result))
}

// Avoid: Imperative with mutation
fn analyze_project_bad(config: &Config) -> Result<AnalysisResults> {
    let mut results = AnalysisResults::new();
    for file in discover_files(config)? {
        let ast = parse_file(&file)?;
        let metrics = analyze_complexity(&ast);
        results.add_metrics(metrics); // Mutation!
    }
    Ok(results)
}
```

## Function Design Guidelines

### Size and Complexity Limits

- **Maximum function length**: 20 lines (prefer 5-10)
- **Maximum cyclomatic complexity**: 5
- **Maximum nesting depth**: 2 levels
- **Maximum parameters**: 3-4 (use structs for more)

### Pure Function Patterns

```rust
// Good: Pure function with clear input/output
fn calculate_cognitive_complexity(ast: &Ast) -> u32 {
    ast.functions()
        .map(|func| func.cognitive_weight())
        .sum()
}

// Good: Pure transformation
fn normalize_path(path: &Path) -> PathBuf {
    path.components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect()
}

// Avoid: Side effects mixed with computation
fn calculate_complexity_bad(ast: &Ast) -> u32 {
    println!("Calculating complexity..."); // Side effect!
    let mut total = 0;
    for func in ast.functions() {
        total += func.cognitive_weight(); // Mutation!
    }
    total
}
```

### Error Handling Patterns

```rust
use anyhow::{Context, Result};

// Good: Functional error handling
fn parse_and_analyze(content: &str, path: &Path) -> Result<FileMetrics> {
    parse_content(content, path)
        .context("Failed to parse file")?
        .analyze()
        .context("Analysis failed")
}

// Good: Monadic chaining with custom extension
fn process_file(path: &Path) -> Result<AnalysisResult> {
    read_file(path)?
        .map_ok(|content| preprocess_content(&content))
        .and_then_async(|content| parse_content(&content, path))
        .map_ok(|ast| analyze_ast(&ast))
}
```

## Testing Standards

### Test Organization

- **Unit tests**: In same file as implementation using `#[cfg(test)]`
- **Integration tests**: In `tests/` directory for cross-module scenarios
- **Property tests**: Use `proptest` for invariant verification
- **Benchmarks**: Use `criterion` for performance-critical code

### Test Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn pure_function_deterministic() {
        let input = create_test_ast();
        let result1 = calculate_complexity(&input);
        let result2 = calculate_complexity(&input);
        assert_eq!(result1, result2);
    }

    proptest! {
        #[test]
        fn complexity_never_negative(
            functions in prop::collection::vec(any::<FunctionMetrics>(), 0..100)
        ) {
            let total = calculate_total_complexity(&functions);
            prop_assert!(total >= 0);
        }
    }
}
```

## Quality Gates

### Definition of Done

- [ ] Functions under 20 lines with single responsibility
- [ ] Pure functions separated from I/O operations
- [ ] Comprehensive test coverage (aim for 85%+)
- [ ] No clippy warnings with `#![deny(clippy::all)]`
- [ ] Formatted with `cargo fmt`
- [ ] Documented public APIs with examples
- [ ] Benchmarks for performance-critical paths
- [ ] Integration tests for user-facing features

### Required Commands Before Commit

```bash
# These commands must pass before any commit
cargo test --all-features    # All tests pass
cargo clippy --all-targets --all-features -- -D warnings  # No warnings
cargo fmt --all -- --check  # Code is formatted
cargo doc --no-deps         # Documentation builds
cargo deny check             # License and security compliance
```

### Continuous Integration Validation

- **Multi-platform testing** - Ubuntu and macOS
- **All feature combinations** - Test with/without optional features
- **Security auditing** - `cargo deny` for vulnerabilities
- **Documentation generation** - Ensure docs build without warnings

## Code Review Standards

### Architectural Review

- [ ] New code follows functional patterns
- [ ] Side effects isolated to I/O boundaries
- [ ] Data transformations are pure and testable
- [ ] Error handling uses `Result` chains appropriately
- [ ] Performance implications considered (parallel vs sequential)

### Rust-Specific Review

- [ ] Appropriate use of `Clone` vs borrowing
- [ ] Memory allocations minimized in hot paths
- [ ] Proper `Send + Sync` bounds for parallel code
- [ ] No unnecessary `.unwrap()` calls (use `?` or proper error handling)
- [ ] Lifetime annotations only where necessary

## Performance Guidelines

### Parallel Processing

```rust
// Good: Parallel analysis of independent files
files.into_par_iter()
    .map(|file| analyze_file_complexity(&file))
    .collect::<Result<Vec<_>>>()

// Good: Parallel aggregation with reduce
results.par_iter()
    .map(|result| extract_metrics(result))
    .reduce(|| Metrics::default(), |a, b| a.merge(b))
```

### Memory Efficiency

```rust
// Good: Streaming processing for large datasets
fn process_large_codebase(files: impl Iterator<Item = PathBuf>) -> impl Iterator<Item = FileMetrics> {
    files
        .map(|path| analyze_file(&path))
        .filter_map(Result::ok)
}

// Avoid: Loading everything into memory
fn process_large_codebase_bad(files: Vec<PathBuf>) -> Vec<FileMetrics> {
    files.into_iter()
        .map(|path| analyze_file(&path).unwrap())
        .collect() // Memory explosion!
}
```

## Common Patterns

### Builder Pattern with Type State

```rust
pub struct AnalysisConfigBuilder<State> {
    config: AnalysisConfig,
    _state: PhantomData<State>,
}

impl AnalysisConfigBuilder<Incomplete> {
    pub fn complexity_threshold(mut self, threshold: u32) -> AnalysisConfigBuilder<HasThreshold> {
        self.config.complexity_threshold = threshold;
        AnalysisConfigBuilder {
            config: self.config,
            _state: PhantomData,
        }
    }
}

impl AnalysisConfigBuilder<HasThreshold> {
    pub fn build(self) -> AnalysisConfig {
        self.config
    }
}
```

### Immutable Updates

```rust
// Good: Functional update pattern
fn update_metrics(metrics: FileMetrics, new_function: FunctionMetrics) -> FileMetrics {
    FileMetrics {
        functions: metrics.functions.update(new_function),
        ..metrics
    }
}

// Good: Using im crate for persistent collections
fn add_debt_item(report: TechnicalDebtReport, item: DebtItem) -> TechnicalDebtReport {
    TechnicalDebtReport {
        items: report.items.push_back(item),
        ..report
    }
}
```

## Documentation Standards

- **Module-level docs** - Explain purpose and usage patterns
- **Function docs** - Include examples for public APIs
- **Error docs** - Document error conditions and recovery
- **Architecture docs** - Maintain ARCHITECTURE.md for high-level design

## Tool Integration

### Required Development Tools

- **rustfmt** - Code formatting (`cargo fmt`)
- **clippy** - Linting and best practices (`cargo clippy`)
- **cargo-deny** - License and security auditing
- **criterion** - Benchmarking for performance-critical code
- **proptest** - Property-based testing

### Editor Configuration

Recommended VS Code extensions:
- rust-analyzer
- Better TOML
- GitLens
- Thunder Client (for API testing)

## Specialized Agents

Use these agents proactively when working on debtmap:

- **refactor-assistant**: Extract complex analysis functions into pure, composable pieces
- **test-runner**: Ensure all tests pass before commits, especially integration tests
- **code-reviewer**: Review functional programming adherence and Rust idioms
- **file-ops**: Organize analysis output files and manage test data
- **git-ops**: Handle feature branches and ensure clean commit history

## Important Reminders

**NEVER**:
- Mix computation with I/O in the same function
- Use `.unwrap()` in production code paths
- Ignore clippy warnings (fix or explicitly allow with justification)
- Commit failing tests or non-compiling code
- Introduce performance regressions without benchmarking

**ALWAYS**:
- Write tests for pure functions (they're easy to test!)
- Use `Result<T>` for fallible operations
- Prefer iterator chains over manual loops
- Document complex analysis algorithms
- Run full test suite before pushing (`cargo test --all-features`)
- Use appropriate parallel processing for CPU-intensive analysis

This project exemplifies functional programming principles in Rust while maintaining high performance for code analysis tasks.