# Introduction

> 🚧 **Early Prototype** - This project is under active development and APIs may change

Debtmap is a fast code complexity and technical debt analyzer written in Rust. Debtmap identifies which code to refactor for maximum cognitive debt reduction and which code to test for maximum risk reduction, providing data-driven prioritization for both.

## What is Debtmap?

Unlike traditional static analysis tools that simply flag complex code, Debtmap answers two critical questions:
1. **"What should I refactor to reduce cognitive burden?"** - Identifies overly complex code that slows down development
2. **"What should I test first to reduce the most risk?"** - Pinpoints untested complex code that threatens stability

Debtmap analyzes your codebase to identify complexity hotspots, technical debt patterns, and architectural risks. It supports multiple programming languages including Rust (full support), Python, JavaScript, and TypeScript (partial support).

**What Makes Debtmap Different:**
- **Entropy-Based Complexity Analysis**: Uses information theory to distinguish genuinely complex code from pattern-based repetitive code, reducing false positives by up to 70%
- **Coverage-Risk Correlation**: The only tool that combines complexity metrics with test coverage to identify genuinely risky code (high complexity + low coverage = critical risk)
- **Risk-Driven Prioritization**: Prioritizes refactoring and testing efforts based on complexity, coverage, and dependency factors
- **Actionable Guidance**: Provides specific recommendations like "extract nested conditions" or "split this 80-line function" rather than just flagging issues
- **Performance**: 10-100x faster than Java/Python-based competitors (written in Rust with parallel processing)

## Why Use Debtmap?

Debtmap helps you make data-driven decisions about where to focus your refactoring and testing efforts:

- **Identify Complexity** - Find complex functions and modules that need refactoring, with concrete metrics showing which changes will have the most impact
- **Detect Technical Debt** - Discover 20+ debt patterns including code smells, security vulnerabilities, resource management issues, and architectural problems
- **Assess Risk** - Prioritize improvements based on sophisticated risk scoring that combines complexity, test coverage, and dependency impact
- **Track Quality** - Monitor code quality metrics over time with the `compare` command to verify that refactoring efforts achieved their goals
- **Get Actionable Recommendations** - Receive specific guidance like "refactoring this will reduce complexity by 60%" or "testing this will reduce risk by 5%"

## Key Features

### Analysis Capabilities
- **Multi-language support** - Fully supports Rust. Partial support for Python, JavaScript, and TypeScript
- **Entropy-based complexity analysis** - Distinguishes between genuinely complex code and pattern-based repetitive code using information theory
- **Token classification system** - Advanced token categorization with weighted entropy for accurate complexity assessment
- **Comprehensive debt detection** - Identifies 13 different technical debt types across security, organization, testing, and resource management
- **Security vulnerability detection** - Finds hardcoded secrets, weak crypto, SQL injection risks, and unsafe code patterns
- **Resource management analysis** - Identifies inefficient allocations, nested loops, and blocking I/O patterns
- **Code organization analysis** - Detects god objects, feature envy, primitive obsession, and magic values
- **Testing quality assessment** - Analyzes test complexity, flaky patterns, and assertion quality
- **Context-aware analysis** - Reduces false positives through intelligent context detection (enabled by default)

### Risk Analysis & Prioritization
- **Coverage-based risk analysis** - Correlates complexity with test coverage to identify truly risky code
- **Risk-driven testing recommendations** - Prioritizes testing efforts based on complexity-coverage correlation and dependency impact
- **Call graph analysis** - Tracks upstream callers and downstream callees to understand dependency impact
- **Tiered prioritization** - Surfaces critical architectural issues above simple testing gaps
- **Quantified impact** - Shows concrete metrics like "refactoring this will reduce complexity by 60%"

### Performance & Output
- **Parallel processing** - Built with Rust and Rayon for blazing-fast analysis of large codebases
- **Multiple output formats** - JSON, Markdown, and human-readable terminal formats
- **Configurable thresholds** - Customize complexity and duplication thresholds to match your standards
- **Incremental analysis** - Smart caching system for analyzing only changed files
- **Verbosity controls** - Multiple verbosity levels (-v, -vv, -vvv) for progressive detail

### Configuration & Customization
- **Flexible suppression** - Inline comment-based suppression for specific code sections
- **Configuration file** - `.debtmap.toml` for project-specific settings
- **Test-friendly** - Easily exclude test fixtures and example code from debt analysis
- **Macro expansion support** - Handles Rust macro expansions with configurable warnings

### Commands
- **`analyze`** - Comprehensive debt analysis with unified prioritization
- **`validate`** - Enforce quality thresholds in CI/CD pipelines
- **`compare`** - Track improvements over time and verify refactoring goals
- **`init`** - Generate configuration file with sensible defaults

## Target Audience

Debtmap is designed for:

- **Development teams** - Get concrete metrics for planning sprints. Know exactly which refactoring will reduce complexity by 60% or which function needs 6 unit tests for full coverage.
- **Engineering managers** - Track quality trends over time with the `compare` command. Monitor whether refactoring efforts are actually improving codebase health.
- **Code reviewers** - Focus reviews on high-risk areas identified by Debtmap. Prioritize reviewing untested complex code over simple utility functions.
- **Developers refactoring legacy codebases** - Receive actionable guidance like "extract nested conditions", "split this 80-line function into 3 smaller functions", or "add error handling for this catch block".

## Getting Started

Ready to analyze your codebase? Check out:
- [Getting Started](./getting-started.md) - Installation and first analysis
- [Analysis Guide](./analysis-guide.md) - Understanding the metrics and output
- [Output Formats](./output-formats.md) - JSON, Markdown, and terminal formats
