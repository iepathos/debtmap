# Changelog

All notable changes to debtmap will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0] - 2025-12-01

### Changed - Strategic Direction

- **Rust-Only Focus**: Debtmap now exclusively supports Rust code analysis
  - Removed Python, JavaScript, and TypeScript language support
  - Enables deeper Rust language integration and more accurate analysis
  - Allows focus on Rust-specific patterns, idioms, and best practices
  - Significantly reduced dependency footprint and complexity

### Removed

- **Python Language Support** (Spec 191)
  - Removed Python AST extraction and analysis
  - Removed Python-specific analyzers: asyncio patterns, dead code detection, exception flow, purity analysis
  - Removed Python complexity calculations and pattern detection
  - Removed Python resource tracking: async resources, circular references, context managers, unbounded collections
  - Removed Python testing analyzers: test detection, assertion patterns, flaky test detection, framework detection
  - Removed Python-specific benchmarks and integration tests
  - Removed 17,000+ lines of Python-specific code

- **JavaScript and TypeScript Support** (Spec 192)
  - Removed JavaScript complexity analysis
  - Removed JavaScript dead code detection
  - Removed JavaScript resource management tracking
  - Removed JavaScript testing analyzers
  - Removed 3,000+ lines of JavaScript/TypeScript code
  - Removed framework pattern detection for web frameworks

- **Multi-Language Infrastructure**
  - Removed cross-language analysis utilities
  - Removed language-agnostic pattern extraction
  - Simplified file walker to focus on `.rs` files
  - Removed language-specific configuration options

### Improved

- **Architecture Refinement** (Spec 181)
  - Implemented "Pure Core, Imperative Shell" pattern for analyzers
  - Separated I/O operations from pure analysis logic
  - Improved testability and composability of analysis functions
  - Better adherence to functional programming principles

- **Code Quality**
  - Applied Stillwater evaluation recommendations
  - Improved code organization and modularity
  - Enhanced type safety and error handling
  - Reduced cyclomatic complexity in key modules

### Fixed

- **Tier Classification Accuracy** (Commit 2b037ce0)
  - Fixed tier classification to use sophisticated scoring metrics instead of raw cyclomatic complexity
  - Tier classification now respects weighted complexity (30% cyclomatic + 70% cognitive), entropy dampening, nesting depth, and other advanced metrics
  - Functions with high cognitive complexity (e.g., cyclo=9, cognitive=16, nesting=4) now correctly appear as T2 (Complex) instead of being hidden as T4 (Maintenance)
  - Resolves false negatives where high-cognitive-load functions were incorrectly filtered from analysis results
  - Example: `reconcile_state()` in examples/rethinking-code-quality-analysis now correctly ranks as #1 priority
  - Impact: Tier classification now aligns with debtmap's core philosophy that cognitive load and nesting matter more than raw cyclomatic complexity

### Benefits of Rust-Only Approach

- **Accuracy**: Native syn-based AST parsing provides 100% Rust language coverage
- **Performance**: Optimized for Rust analysis without multi-language overhead
- **Codebase Size**: Reduced from ~51,000 lines to ~38,000 lines (-25%)
- **Dependencies**: Streamlined dependency tree by removing multi-language parsers
- **Features**: Deep integration with Rust-specific constructs:
  - Trait implementations
  - Macro expansion tracking
  - Lifetime and ownership analysis
  - Async/await pattern detection
  - Property-based test detection
- **Maintenance**: Simpler codebase focused on a single language ecosystem

### Migration Guide

For projects using debtmap on Python, JavaScript, or TypeScript codebases:
- Use debtmap v0.6.0 or earlier for multi-language support
- Consider language-specific alternatives:
  - Python: pylint, radon, vulture
  - JavaScript/TypeScript: ESLint, SonarJS, CodeClimate

For Rust projects:
- No changes required - continue using debtmap as normal
- Expect improved accuracy and new Rust-specific features in future releases

### Documentation

- Updated ARCHITECTURE.md to reflect Rust-only implementation (Spec 193)
- Updated analysis guide to focus on Rust metrics
- Updated getting started guide to remove Python/JS/TS references
- Updated README with Rust-only focus and capabilities
- Updated CONTRIBUTING.md with Rust-specific development guidelines
- All documentation now consistently describes Rust-only capabilities

### Internal

- Added STILLWATER_EVALUATION.md documenting architectural assessment
- Created specs 181-200 for future improvements:
  - Terminal output UX enhancements (specs 194-200)
  - State transition metrics display (spec 190)
  - Additional architectural refinements (specs 182-189)
