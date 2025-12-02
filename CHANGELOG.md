# Changelog

All notable changes to debtmap will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed - Strategic Direction

- **Rust-Only Focus**: Debtmap now exclusively supports Rust code analysis
  - Removed Python, JavaScript, and TypeScript language support
  - Enables deeper Rust language integration and more accurate analysis
  - Allows focus on Rust-specific patterns, idioms, and best practices
  - Significantly reduced dependency footprint and complexity

### Benefits of Rust-Only Approach

- **Accuracy**: Native syn-based AST parsing provides 100% Rust language coverage
- **Performance**: Optimized for Rust analysis without multi-language overhead
- **Features**: Deep integration with Rust-specific constructs:
  - Trait implementations
  - Macro expansion tracking
  - Lifetime and ownership analysis
  - Async/await pattern detection
  - Property-based test detection
- **Maintenance**: Simpler codebase focused on a single language ecosystem

### Migration Guide

For projects using debtmap on Python, JavaScript, or TypeScript codebases:
- Use debtmap v0.1.x or earlier for multi-language support
- Consider language-specific alternatives:
  - Python: pylint, radon, vulture
  - JavaScript/TypeScript: ESLint, SonarJS, CodeClimate

For Rust projects:
- No changes required - continue using debtmap as normal
- Expect improved accuracy and new Rust-specific features in future releases

### Documentation Updates

- Updated ARCHITECTURE.md to reflect Rust-only implementation
- Verified book/src/analysis-guide.md focuses on Rust metrics
- Verified book/src/getting-started.md removes Python/JS/TS references
- All documentation now consistently describes Rust-only capabilities
