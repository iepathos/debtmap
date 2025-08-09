---
number: 01
title: Debtmap - Code Complexity and Technical Debt Analyzer
category: core
priority: high
status: draft
dependencies: []
created: 2025-08-09
---

# Specification 01: Debtmap - Code Complexity and Technical Debt Analyzer

**Category**: core
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Modern software projects accumulate technical debt and complexity over time, making maintenance increasingly difficult. Development teams need automated tools to identify complexity hotspots, detect code smells, and track technical debt items across their codebases.

Debtmap addresses this need as a standalone, language-agnostic tool that performs comprehensive code analysis without requiring language-specific testing infrastructure. The tool focuses on providing actionable insights about code quality through metrics-driven analysis.

## Objective

Create a Rust-based command-line tool called `debtmap` that analyzes code complexity and technical debt across multiple programming languages (initially Rust and Python), providing clear, actionable metrics to help teams identify and prioritize code improvement opportunities.

## Core Analysis Features

### Complexity Analysis
- **Cyclomatic Complexity**: Measure control flow complexity by counting decision points
- **Cognitive Complexity**: Assess mental effort required to understand code
- **Nesting Depth**: Track maximum nesting levels in functions
- **Function Length**: Identify overly long functions that violate single responsibility

### Technical Debt Detection
- **Code Smells**: Identify patterns indicating poor design (long methods, large classes, duplicate code)
- **TODO/FIXME Tracking**: Catalog and prioritize technical debt markers
- **Duplication Analysis**: Find copy-paste code using AST-based similarity detection
- **Dependency Issues**: Detect circular dependencies and tightly coupled modules

### Output and Reporting
- Multiple output formats (JSON, Markdown, Terminal)
- Configurable complexity thresholds
- Priority-ranked findings for actionable insights
- Incremental analysis support for CI/CD integration

## Implementation Principles

### Functional Programming Approach
- **Pure Functions**: Core analysis logic implemented as pure, side-effect-free functions
- **Immutable Data Structures**: Use persistent data structures for analysis state
- **Function Composition**: Build complex analyzers from simple, composable functions
- **Transformation Pipelines**: Process code through functional transformation chains
- **Monadic Error Handling**: Use Result/Option types for robust error propagation

## Acceptance Criteria

- [ ] Standalone binary runs independently without external dependencies
- [ ] Analyzes Rust projects using syn parser for AST analysis
- [ ] Analyzes Python projects using appropriate Python AST parser
- [ ] Calculates cyclomatic and cognitive complexity metrics
- [ ] Identifies at least 5 types of technical debt (TODOs, complexity, duplication, etc.)
- [ ] Produces structured JSON output for programmatic consumption
- [ ] Supports filtering and threshold configuration via CLI flags
- [ ] Includes comprehensive --help documentation
- [ ] Binary size under 10MB for release builds
- [ ] Processes large codebases (50k+ lines) in under 5 seconds

## Technical Architecture

### Functional Core Architecture
```rust
// Functional module structure
src/
├── main.rs                    // CLI entry point (IO boundary)
├── cli.rs                     // Command-line parsing
├── core/                      // Pure functional core
│   ├── mod.rs                // Core types and traits
│   ├── ast.rs                // AST representation
│   └── metrics.rs            // Metric calculations
├── analyzers/                 // Language-specific analyzers
│   ├── mod.rs                // Analyzer trait
│   ├── rust.rs               // Rust analysis (pure functions)
│   └── python.rs             // Python analysis (pure functions)
├── complexity/                // Complexity calculations (pure)
│   ├── mod.rs                // Complexity types
│   ├── cyclomatic.rs         // Cyclomatic complexity
│   └── cognitive.rs          // Cognitive complexity
├── debt/                      // Debt detection (pure)
│   ├── mod.rs                // Debt types and combinators
│   ├── patterns.rs           // Pattern matching functions
│   └── duplication.rs        // Similarity detection
├── transformers/              // Functional transformations
│   ├── mod.rs                // Transformation pipelines
│   └── filters.rs            // Filter combinators
└── io/                        // IO operations (at boundaries)
    ├── mod.rs                // IO helpers
    ├── output.rs             // Output formatting
    └── walker.rs             // File traversal
```

### Functional Analysis Pipeline

```rust
// Pure functional analyzer trait
pub trait Analyzer {
    fn parse(content: &str) -> Result<Ast>;
    fn analyze(ast: &Ast) -> FileMetrics;
}

// Functional pipeline composition
pub fn analyze_file(content: String) -> Result<FileMetrics> {
    parse_content(&content)
        .map(|ast| transform_ast(ast))
        .map(|ast| calculate_metrics(ast))
        .map(|metrics| apply_filters(metrics))
}

// Immutable metrics structure
#[derive(Clone, Debug)]
pub struct FileMetrics {
    pub complexity: ComplexityMetrics,
    pub debt_items: Vec<DebtItem>,
    pub dependencies: Vec<Dependency>,
    pub duplications: Vec<DuplicationBlock>,
}

// Function composition for analysis
pub fn compose_analyzers(
    parsers: Vec<Parser>,
    transformers: Vec<Transformer>,
    calculators: Vec<Calculator>,
) -> impl Fn(&str) -> Result<FileMetrics>
```

### Immutable Data Structures

```rust
use im::{HashMap, Vector};  // Persistent data structures

// Immutable analysis results
#[derive(Clone, Debug)]
pub struct AnalysisResults {
    pub project_path: PathBuf,
    pub timestamp: DateTime<Utc>,
    pub complexity: ComplexityReport,
    pub technical_debt: TechnicalDebtReport,
    pub dependencies: DependencyReport,
}

// Complexity metrics using persistent vectors
#[derive(Clone, Debug)]
pub struct ComplexityReport {
    pub metrics: Vector<FunctionMetrics>,
    pub summary: ComplexitySummary,
}

// Individual function metrics
#[derive(Clone, Debug)]
pub struct FunctionMetrics {
    pub name: String,
    pub file: PathBuf,
    pub line: usize,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
}

// Technical debt using persistent collections
#[derive(Clone, Debug)]
pub struct TechnicalDebtReport {
    pub items: Vector<DebtItem>,
    pub by_type: HashMap<DebtType, Vector<DebtItem>>,
    pub priorities: Vector<Priority>,
}
```

### CLI Interface

```bash
# Basic usage
debtmap analyze <path>

# With options
debtmap analyze <path> \
  --format json|markdown|terminal \
  --output report.json \
  --threshold-complexity 10 \
  --threshold-duplication 50 \
  --languages rust,python

# Specific analysis types
debtmap complexity <path>    # Only complexity analysis
debtmap debt <path>          # Only debt detection
debtmap deps <path>          # Only dependency analysis

# Configuration
debtmap init                 # Create .debtmap.toml config
debtmap validate             # Validate against thresholds
```

### Output Formats

#### Terminal Output Format
```
Debtmap Analysis Report
=======================
📊 Summary:
  Files analyzed: 150
  Total lines: 12,500
  Average complexity: 4.2
  Debt items: 45

⚠️ Complexity Hotspots (top 5):
  1. src/analyzer/rust.rs:145 parse_ast() - Cyclomatic: 15, Cognitive: 18
  2. src/core/transform.rs:89 apply_transforms() - Cyclomatic: 12, Cognitive: 14
  3. src/parser/python.rs:234 process_node() - Cyclomatic: 11, Cognitive: 13

🔧 Technical Debt (45 items):
  High Priority (5):
    - src/main.rs:45 - TODO: Add error recovery
    - src/parser.rs:102 - FIXME: Handle edge case
  
  Code Duplication (8 blocks):
    - 15 lines duplicated between src/a.rs:10-25 and src/b.rs:40-55
    - 22 lines duplicated between src/complexity/cyclomatic.rs and cognitive.rs

✓ Pass/Fail: PASS (all metrics within thresholds)
```

#### Markdown Output Format
```markdown
# Debtmap Analysis Report

Generated: 2025-08-09 10:30:00 UTC
Version: 1.0.0

## Executive Summary

| Metric | Value | Status |
|--------|-------|--------|
| Files Analyzed | 150 | - |
| Total Lines | 12,500 | - |
| Average Complexity | 4.2 | ✅ Good |
| High Complexity Functions | 8 | ⚠️ Warning |
| Technical Debt Items | 45 | ⚠️ Medium |
| Code Duplication | 12.5% | ❌ High |

## Complexity Analysis

### Hotspots Requiring Attention

| File:Line | Function | Cyclomatic | Cognitive | Recommendation |
|-----------|----------|------------|-----------|----------------|
| src/analyzer/rust.rs:145 | parse_ast | 15 | 18 | Refactor: Split into smaller functions |
| src/core/transform.rs:89 | apply_transforms | 12 | 14 | Review: Consider extracting complex logic |
| src/parser/python.rs:234 | process_node | 11 | 13 | Review: Simplify control flow |

### Complexity Distribution

- Functions with complexity > 10: 8 (5.3%)
- Functions with complexity 5-10: 24 (16%)
- Functions with complexity < 5: 118 (78.7%)

## Technical Debt

### Priority Items

#### High Priority (5 items)
- [ ] `src/main.rs:45` - TODO: Add error recovery for network failures
- [ ] `src/parser.rs:102` - FIXME: Handle Unicode edge cases in parser

#### Medium Priority (15 items)
- [ ] `src/analyzer/mod.rs:78` - TODO: Optimize memory usage for large files
- [ ] `src/complexity/cognitive.rs:234` - Refactor: Reduce nesting depth

### Code Duplication

| Location 1 | Location 2 | Lines | Similarity |
|------------|------------|-------|------------|
| src/a.rs:10-25 | src/b.rs:40-55 | 15 | 95% |
| src/complexity/cyclomatic.rs:45-67 | src/complexity/cognitive.rs:89-111 | 22 | 88% |

## Recommendations

1. **Immediate Action**: Address high-priority debt items and refactor top 3 complexity hotspots
2. **Short Term**: Reduce code duplication by extracting common functionality
3. **Long Term**: Establish complexity budget and monitor trends over time
```

### Functional Library API

```rust
use debtmap::{analyze, Config, Pipeline};

// Functional configuration builder
let config = Config::default()
    .with_languages(vec![Language::Rust, Language::Python])
    .with_threshold(Threshold::Complexity(10))
    .with_threshold(Threshold::Duplication(50));

// Compose analysis pipeline
let pipeline = Pipeline::new()
    .add_parser(rust_parser())
    .add_parser(python_parser())
    .add_transformer(normalize_ast())
    .add_calculator(complexity_calculator())
    .add_calculator(debt_calculator());

// Run analysis (pure function)
let results = pipeline.analyze(read_files("./src")?);
```

## Dependencies

**Build Dependencies**:
- `syn` - Rust AST parsing
- `rustpython-parser` - Python AST parsing  
- `clap` - CLI argument parsing
- `serde`/`serde_json` - Serialization
- `im` - Persistent immutable data structures
- `rayon` - Parallel processing
- `sha2` - Content hashing for duplication detection

**Runtime**: Single static binary with no external dependencies

## Testing Strategy

### Property-Based Testing
- Use proptest for testing pure functions with generated inputs
- Verify invariants hold across transformations
- Test commutativity and associativity of metric combinations

### Unit Tests
- Test individual pure functions in isolation
- Verify complexity calculations with known examples
- Test function composition and pipeline construction
- Validate monadic error handling

### Integration Tests
- Test full analysis pipeline on sample codebases
- Verify output format correctness
- Test incremental analysis caching
- Validate cross-language consistency

### Performance Tests
- Benchmark pure function performance
- Test memory usage with persistent data structures
- Verify lazy evaluation efficiency
- Profile parallel analysis with rayon

## Documentation Requirements

- **Code Documentation**:
  - Comprehensive rustdoc for all public APIs
  - Functional programming patterns used
  - Examples demonstrating function composition
  
- **User Documentation**:
  - README with installation instructions
  - User guide with CLI examples
  - Configuration file format
  - CI/CD integration patterns
  
- **Architecture Documentation**:
  - Functional core / imperative shell pattern
  - Pure function design principles
  - Data flow and transformation pipeline

## Implementation Guidelines

### Functional Programming Practices
- Keep IO operations at the boundaries (main function and output modules)
- Use pure functions for all analysis logic
- Leverage Rust's type system for correctness (Result, Option, newtypes)
- Prefer function composition over imperative loops
- Use persistent data structures for efficiency with immutability
- Apply map/filter/fold patterns for data transformations

### Development Approach
- Start with Rust language support using syn parser
- Build pure functional core before adding IO
- Use rayon for parallel processing of independent files
- Implement caching as a pure function (content hash → results)
- Add Python support after Rust implementation stabilizes

## Future Enhancements

- Additional language support via tree-sitter
- Language Server Protocol (LSP) implementation
- Real-time incremental analysis
- Custom analysis rules via functional combinators
- Parallel distributed analysis for monorepos
- Integration with version control for historical trends