# Introduction

> 🚧 **Early Prototype** - This project is under active development and APIs may change

Debtmap is a code complexity and technical debt analyzer that identifies which code to refactor for maximum cognitive debt reduction and which code to test for maximum risk reduction.

## What is Debtmap?

Unlike traditional static analysis tools that simply flag complex code, Debtmap answers two critical questions:
1. **"What should I refactor to reduce cognitive burden?"** - Identifies overly complex code that slows down development
2. **"What should I test first to reduce the most risk?"** - Pinpoints untested complex code that threatens stability

Debtmap analyzes your codebase to identify complexity hotspots, technical debt patterns, and architectural risks. It supports Rust, Python, JavaScript, and TypeScript with full AST parsing and analysis capabilities. Rust includes additional advanced features like macro expansion and trait tracking.

**What Makes Debtmap Different:**
- **Coverage-Risk Correlation**: Combines complexity metrics with test coverage to identify genuinely risky code (high complexity + low coverage = critical risk)
- **Multi-Factor Analysis**: Analyzes complexity, coverage, dependencies, and call graphs for comprehensive prioritization
- **Reduced False Positives**: Uses entropy analysis and pattern detection to distinguish genuinely complex code from repetitive patterns, reducing false positives by up to 70%. This is achieved through an advanced token classification system that categorizes code tokens and applies weighted entropy to accurately assess complexity.
- **Actionable Guidance**: Provides specific recommendations like "extract nested conditions" or "split this 80-line function" with quantified impact metrics
- **Performance**: Significantly faster than Java/Python-based competitors (written in Rust with parallel processing)

## Why Use Debtmap?

Debtmap helps you make data-driven decisions about where to focus your refactoring and testing efforts:

- **Identify Complexity** - Find complex functions and modules that need refactoring, with concrete metrics showing which changes will have the most impact
- **Detect Technical Debt** - Discover 30+ debt patterns including code smells, security vulnerabilities, resource management issues, and architectural problems
- **Assess Risk** - Prioritize improvements based on sophisticated risk scoring that combines complexity, test coverage, and dependency impact
- **Track Quality** - Monitor code quality metrics over time with the `compare` command (which can use `--plan` to automatically extract target locations from implementation plans and track improvements) to verify that refactoring efforts achieved their goals
- **Get Actionable Recommendations** - Receive specific guidance like "refactoring this will reduce complexity by 60%" or "testing this will reduce risk by 5%"
- **Automated Debt Reduction** - Integrates with [Prodigy workflows](./prodigy-integration.md) for AI-driven automated refactoring with iterative validation and testing (via external integration)

## Key Features

### Analysis Capabilities
- **Multi-language support** - Full support for Rust, Python, JavaScript, and TypeScript with AST parsing, complexity analysis, and debt detection
- **Reduced false positives** - Uses entropy analysis and pattern detection to distinguish genuinely complex code from repetitive patterns (up to 70% reduction)
- **Token classification system** - Advanced token categorization with weighted entropy for accurate complexity assessment
- **Threshold presets** - Quick setup with strict, balanced (default), or lenient presets matching different project types and quality standards
- **Comprehensive debt detection** - Identifies 30+ technical debt patterns across security (5 types), code organization (god objects, feature envy, magic values), resource management (5 types), testing quality (3 types), and error handling (4 types: error swallowing, poor error propagation, panic patterns, inadequate exception handling)
- **Security vulnerability detection** - Finds hardcoded secrets, weak crypto, SQL injection risks, and unsafe code patterns
- **Resource management analysis** - Identifies inefficient allocations, nested loops, and blocking I/O patterns
- **Code organization analysis** - Detects god objects, feature envy, primitive obsession, and magic values
- **Testing quality assessment** - Analyzes test complexity, flaky patterns, and assertion quality
- **File-level aggregation** - Multiple aggregation methods (sum, weighted, logarithmic) for identifying files needing organizational refactoring
- **Context-aware analysis** - Reduces false positives through intelligent context detection (enabled by default)

### Risk Analysis & Prioritization
- **Coverage-based risk analysis** - Correlates complexity with test coverage to identify truly risky code
- **Risk-driven testing recommendations** - Prioritizes testing efforts based on complexity-coverage correlation and dependency impact
- **Call graph analysis** - Tracks upstream callers and downstream callees to understand dependency impact
- **Tiered prioritization** - Multi-stage pipeline (zero coverage, complexity-risk, critical path, dependency impact, effort optimization) surfaces critical architectural issues above simple testing gaps
- **Quantified impact** - Shows concrete metrics like "refactoring this will reduce complexity by 60%"

### Performance & Output
- **Parallel processing** - Built with Rust and Rayon for blazing-fast analysis of large codebases
- **Multiple output formats** - JSON (legacy and unified structures), Markdown, and human-readable terminal formats for different tool integration needs
- **Configurable thresholds** - Customize complexity and duplication thresholds to match your standards
- **Incremental analysis** - Smart caching system for analyzing only changed files
- **Intelligent caching** - Smart cache system with automatic pruning, configurable strategies (LRU, LFU, FIFO), location options (local/shared/custom path), and environment-based configuration for fast repeated analysis
- **Verbosity controls** - Multiple verbosity levels (-v, -vv, -vvv) for progressive detail

### Configuration & Customization
- **Flexible suppression** - Inline comment-based suppression for specific code sections
- **Configuration file** - `.debtmap.toml` or `debtmap.toml` for project-specific settings
- **Test-friendly** - Easily exclude test fixtures and example code from debt analysis
- **Macro expansion support** - Handles Rust macro expansions with configurable warnings

### Commands
- **`analyze`** - Comprehensive debt analysis with unified prioritization
- **`validate`** - Enforce quality thresholds in CI/CD pipelines
- **`compare`** - Track improvements over time and verify refactoring goals
- **`init`** - Generate configuration file with sensible defaults (--force to overwrite)

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

**Tip:** Start with `debtmap analyze . --summary` for a quick overview of your codebase health before diving into detailed analysis.
