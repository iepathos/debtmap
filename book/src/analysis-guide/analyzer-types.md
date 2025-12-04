# Analyzer Types

## Overview

**Debtmap is a Rust-only code analysis tool.** As of specification 191, debtmap focuses exclusively on Rust codebases to provide deep, language-specific insights into code complexity, technical debt, and architectural patterns.

While the architecture supports extensibility through the `Analyzer` trait, only Rust is actively supported and maintained. Files in other programming languages are automatically filtered during discovery and never reach the analysis phase.

**Source**: As documented in src/core/mod.rs:376-377 and src/core/injection.rs:198-200

## Rust Analyzer

Debtmap provides comprehensive analysis for Rust codebases using the `syn` crate for native AST parsing.

### Core Capabilities

The Rust analyzer (`src/analyzers/rust.rs`) provides:

- **Complexity Metrics**: Cyclomatic complexity, cognitive complexity, and entropy analysis
- **Purity Detection**: Identifies pure functions with confidence scoring
- **Call Graph Analysis**: Tracks upstream callers and downstream callees with transitive relationships
- **Trait Implementation Tracking**: Monitors trait implementations across the codebase
- **Macro Expansion Support**: Analyzes complexity within macros accurately
- **Pattern-Based Adjustments**: Recognizes and adjusts for code generation patterns
- **Visibility Tracking**: Distinguishes `pub`, `pub(crate)`, and private functions
- **Test Module Detection**: Identifies `#[cfg(test)]` modules and `#[test]` functions

**Source**: Capabilities verified in src/analyzers/rust.rs:1-100

### Semantic Function Classification

The Rust analyzer automatically classifies functions by their role in the system. This classification feeds into the unified scoring system's role multiplier for accurate technical debt assessment.

**Classification Categories** (src/analyzers/rust.rs):

- **Entry Points**: Functions named `main`, `start`, or public functions in `bin/` modules
- **Business Logic**: Core domain functions containing complex algorithms and business rules
- **Data Access**: Functions performing database queries, file I/O, or network operations
- **Infrastructure**: Logging, configuration, monitoring, and error handling utilities
- **Utilities**: Helper functions, formatters, type converters, and validation functions
- **Test Code**: Functions in `#[cfg(test)]` modules or marked with `#[test]` attribute

These classifications are used to calculate role-based priority multipliers in the risk scoring system. See [Risk Scoring](risk-scoring.md) for details on how semantic classification affects debt prioritization.

## Language Support

### Supported: Rust Only

Debtmap exclusively analyzes Rust source files (`.rs` extension). All analysis features, metrics, and debt detection patterns are designed specifically for Rust's syntax and semantics.

**Language Detection** (src/core/mod.rs:386-391):

```rust
pub fn from_path(path: &std::path::Path) -> Self {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(Self::from_extension)
        .unwrap_or(Language::Unknown)
}
```

The `Language` enum (src/core/mod.rs:368-372) includes `Rust`, `Python`, and `Unknown` variants, but only `Rust` is actively processed:

```rust
pub enum Language {
    Rust,
    Python,  // Architectural placeholder, not supported
    Unknown,
}
```

### File Filtering Behavior

During file discovery, debtmap filters files by extension:

1. **Rust files (`.rs`)**: Parsed and analyzed
2. **All other files**: Silently filtered out—no warnings or errors generated
3. **Unknown extensions**: Mapped to `Language::Unknown` and filtered during discovery

**Source**: Language detection implemented in src/core/mod.rs:375-391

**Example Usage**:

```bash
# Analyze all Rust files in current directory
debtmap analyze .

# Analyze specific Rust file
debtmap analyze src/main.rs

# Python, JavaScript, and other files are ignored
# (no error messages, just skipped)
```

## Extensibility

While debtmap currently focuses on Rust-only analysis, the architecture is designed to support additional languages in the future through the `Analyzer` trait.

### Analyzer Trait

The core `Analyzer` trait defines the interface for language-specific analyzers (src/analyzers/mod.rs:39-43):

```rust
pub trait Analyzer: Send + Sync {
    fn parse(&self, content: &str, path: std::path::PathBuf) -> Result<Ast>;
    fn analyze(&self, ast: &Ast) -> FileMetrics;
    fn language(&self) -> crate::core::Language;
}
```

**Note**: There is also a generic `Analyzer` trait with associated types in src/core/traits.rs:11-16, used for internal abstractions. The trait shown above is the public extension point for language analyzers.

### Current Implementation

The `AnalyzerFactory` (src/core/injection.rs:190-203) creates language-specific analyzers:

```rust
impl AnalyzerFactory {
    pub fn create_analyzer(&self, language: Language) -> Box<dyn Analyzer<...>> {
        match language {
            Language::Rust => Box::new(RustAnalyzerAdapter::new()),
            Language::Python => {
                panic!("Python analysis is not currently supported.
                       Debtmap is focusing exclusively on Rust analysis.")
            }
        }
    }
}
```

### Adding Language Support (Future)

To add support for a new language:

1. **Implement the `Analyzer` trait** with language-specific parsing and analysis
2. **Add the language variant** to the `Language` enum (src/core/mod.rs:368-372)
3. **Update `from_extension()`** to recognize the file extension (src/core/mod.rs:375-384)
4. **Register in `AnalyzerFactory`** to instantiate your analyzer (src/core/injection.rs:196-201)

**Reference Implementation**: See `src/analyzers/rust.rs` for a complete example of implementing the `Analyzer` trait with full complexity analysis, purity detection, and call graph support.

## See Also

- [Overview](overview.md) - Analysis pipeline and workflow
- [Complexity Metrics](complexity-metrics.md) - Detailed metric calculations
- [Risk Scoring](risk-scoring.md) - How semantic classification affects prioritization

