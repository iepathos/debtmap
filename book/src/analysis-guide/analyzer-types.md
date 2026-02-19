# Analyzer Types

## Overview

Debtmap provides deep, language-specific code analysis through specialized analyzers for each supported language. The architecture uses the `Analyzer` trait for extensibility, with full implementations for Rust, TypeScript, and JavaScript.

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

### Supported Languages

Debtmap provides full analysis for the following languages:

| Language | Parser | File Extensions |
|----------|--------|-----------------|
| **Rust** | `syn` (native AST) | `.rs` |
| **TypeScript** | tree-sitter | `.ts`, `.tsx` |
| **JavaScript** | tree-sitter | `.js`, `.jsx` |

### TypeScript/JavaScript Analyzer

The TypeScript and JavaScript analyzers use tree-sitter for AST parsing and provide:

- **Complexity Metrics**: Cyclomatic complexity, cognitive complexity, nesting depth
- **Entropy Analysis**: Pattern-based false positive reduction
- **Async Pattern Detection**: Promise chains, async/await, callback nesting
- **React Support**: JSX/TSX component analysis
- **Type-Specific Analysis**: TypeScript `any` usage, type assertions

### File Detection

During file discovery, debtmap detects files by extension and routes them to the appropriate analyzer:

```bash
# Analyze all supported files in current directory
debtmap analyze .

# Analyze specific languages only
debtmap analyze . --languages rust,typescript

# All supported languages are enabled by default
```

## Extensibility

The architecture supports adding new languages through the `Analyzer` trait.

### Analyzer Trait

The core `Analyzer` trait defines the interface for language-specific analyzers:

```rust
pub trait Analyzer: Send + Sync {
    fn parse(&self, content: &str, path: std::path::PathBuf) -> Result<Ast>;
    fn analyze(&self, ast: &Ast) -> FileMetrics;
    fn language(&self) -> crate::core::Language;
}
```

### Adding Language Support

To add support for a new language:

1. **Implement the `Analyzer` trait** with language-specific parsing and analysis
2. **Add the language variant** to the `Language` enum
3. **Update `from_extension()`** to recognize the file extension
4. **Register in `AnalyzerFactory`** to instantiate your analyzer

**Reference Implementation**: See `src/analyzers/rust.rs` for Rust or `src/analyzers/typescript.rs` for TypeScript as examples of implementing the `Analyzer` trait.

## See Also

- [Overview](overview.md) - Analysis pipeline and workflow
- [Complexity Metrics](complexity-metrics.md) - Detailed metric calculations
- [Risk Scoring](risk-scoring.md) - How semantic classification affects prioritization

