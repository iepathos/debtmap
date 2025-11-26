//! # Debtmap
//!
//! A code complexity and technical debt analyzer that identifies which code to refactor
//! for maximum cognitive debt reduction and which code to test for maximum risk reduction.
//!
//! ## Why Debtmap?
//!
//! Unlike traditional static analysis tools that simply flag complex code, debtmap answers two critical questions:
//!
//! 1. **"What should I refactor to reduce cognitive burden?"** - Identifies overly complex code that slows down development
//! 2. **"What should I test first to reduce the most risk?"** - Pinpoints untested complex code that threatens stability
//!
//! **Unique Capabilities:**
//!
//! - **Coverage-Risk Correlation** - Combines complexity metrics with test coverage to identify genuinely risky code (high complexity + low coverage = critical risk)
//! - **Reduced False Positives** - Uses entropy analysis and pattern detection to distinguish genuinely complex code from repetitive patterns, reducing false positives by up to 70%
//! - **Actionable Recommendations** - Provides specific guidance with quantified impact metrics instead of generic warnings
//! - **Multi-Factor Analysis** - Analyzes complexity, coverage, dependencies, and call graphs for comprehensive prioritization
//! - **Fast & Open Source** - Written in Rust for 10-100x faster analysis, MIT licensed
//!
//! ## Quick Start
//!
//! ### Basic File Analysis
//!
//! ```rust
//! use debtmap::{analyzers::get_analyzer, Language};
//!
//! // Get language-specific analyzer
//! let analyzer = get_analyzer(Language::Rust);
//!
//! // Parse source code
//! let content = r#"
//!     fn example() {
//!         if true {
//!             println!("hello");
//!         }
//!     }
//! "#;
//! let ast = analyzer.parse(&content, "example.rs".into()).unwrap();
//!
//! // Analyze complexity metrics
//! let metrics = analyzer.analyze(&ast);
//!
//! println!("Functions analyzed: {}", metrics.complexity.functions.len());
//! if !metrics.complexity.functions.is_empty() {
//!     let avg = metrics.complexity.functions.iter()
//!         .map(|f| f.cyclomatic as f64).sum::<f64>()
//!         / metrics.complexity.functions.len() as f64;
//!     println!("Average complexity: {:.2}", avg);
//! }
//! ```
//!
//! ### Code Smell Detection
//!
//! ```rust
//! use debtmap::debt::patterns::find_code_smells;
//! use std::path::Path;
//!
//! let content = r#"
//!     fn example() {
//!         // TODO: Fix this later
//!         let x = 1;
//!     }
//! "#;
//!
//! // Find TODOs, FIXMEs, and other code smells
//! let smells = find_code_smells(&content, Path::new("example.rs"));
//! for smell in smells {
//!     println!("{:?} at line {}", smell.debt_type, smell.line);
//! }
//! ```
//!
//! ### Coverage-Based Risk Analysis
//!
//! ```rust,ignore
//! use debtmap::{
//!     analyzers::get_analyzer,
//!     risk::{lcov::parse_lcov_file, RiskAnalyzer},
//!     Language,
//! };
//! use std::path::PathBuf;
//!
//! // Parse coverage data (skip if file doesn't exist)
//! let coverage_path = std::path::Path::new("target/coverage/lcov.info");
//! if !coverage_path.exists() {
//!     println!("Generate coverage with: cargo llvm-cov --lcov --output-path target/coverage/lcov.info");
//!     return;
//! }
//!
//! let coverage_data = parse_lcov_file(coverage_path).unwrap();
//!
//! // Analyze a file
//! let analyzer = get_analyzer(Language::Rust);
//! let content = std::fs::read_to_string("src/main.rs").unwrap();
//! let ast = analyzer.parse(&content, "src/main.rs".into()).unwrap();
//! let metrics = analyzer.analyze(&ast);
//!
//! // Calculate risk scores for each function
//! let risk_analyzer = RiskAnalyzer::default();
//! let file_path = std::path::Path::new("src/main.rs");
//! for func in &metrics.complexity.functions {
//!     let coverage = coverage_data.get_function_coverage(file_path, &func.name);
//!     let risk = risk_analyzer.analyze_function(
//!         PathBuf::from("src/main.rs"),
//!         func.name.clone(),
//!         (func.start_line, func.end_line),
//!         &func.complexity,
//!         coverage,
//!         false,
//!     );
//!     if risk.risk_score > 5.0 {
//!         println!("HIGH RISK: {} (score: {:.1})", risk.function_name, risk.risk_score);
//!     }
//! }
//! ```
//!
//! ## Features
//!
//! ### Multi-Language Support
//!
//! Debtmap analyzes code across multiple programming languages:
//!
//! - **Rust** - Full support with comprehensive AST analysis using [`syn`](https://docs.rs/syn)
//! - **Python** - Partial support via [`rustpython-parser`](https://docs.rs/rustpython-parser)
//! - **JavaScript/TypeScript** - Partial support via [`tree-sitter`](https://docs.rs/tree-sitter)
//!
//! ### Performance Characteristics
//!
//! - **Parallel Processing** - Uses [`rayon`](https://docs.rs/rayon) for CPU-intensive analysis across multiple files
//! - **Concurrent Data Structures** - Leverages [`dashmap`](https://docs.rs/dashmap) for lock-free concurrent access
//! - **Immutable Collections** - Uses [`im`](https://docs.rs/im) crate for persistent data structures
//! - **Performance** - 10-100x faster than Java/Python-based competitors
//!
//! ### Coverage Integration
//!
//! Debtmap works with any tool generating LCOV format:
//! - **Rust**: [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) (recommended), [`cargo-tarpaulin`](https://github.com/xd009642/tarpaulin)
//! - **Python**: `pytest-cov`, `coverage.py`
//! - **JavaScript**: `jest --coverage`, `nyc`
//!
//! ## Architecture
//!
//! Debtmap follows a functional architecture with clear separation of concerns:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     Input Layer (I/O)                        │
//! │  File Discovery → Content Reading → Coverage Parsing         │
//! └────────────────────────┬────────────────────────────────────┘
//!                          ↓
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Parser Layer (Pure)                       │
//! │  Language Detection → AST Generation → Symbol Extraction     │
//! └────────────────────────┬────────────────────────────────────┘
//!                          ↓
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   Analysis Layer (Pure)                      │
//! │  Complexity → Debt Detection → Risk Assessment → Dependency  │
//! └────────────────────────┬────────────────────────────────────┘
//!                          ↓
//! ┌─────────────────────────────────────────────────────────────┐
//! │                 Aggregation Layer (Functional)               │
//! │  Combine Results → Priority Scoring → Recommendation Gen    │
//! └────────────────────────┬────────────────────────────────────┘
//!                          ↓
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Output Layer (I/O)                        │
//! │  Format Selection → Report Generation → File Writing         │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Core Modules
//!
//! - **[`analyzers`]** - Language-specific parsers and AST analysis
//!   - [`analyzers::get_analyzer`] - Factory for language-specific analyzers
//!   - [`analyzers::analyze_file`] - High-level file analysis API
//!
//! - **[`debt`]** - Technical debt pattern detection
//!   - [`debt::patterns`] - Code smell detection (TODOs, magic numbers, etc.)
//!   - [`debt::smells`] - Function-level smell analysis (long methods, deep nesting)
//!   - [`debt::duplication`] - Duplicate code detection
//!   - [`debt::coupling`] - Module coupling analysis
//!
//! - **[`risk`]** - Risk assessment and prioritization
//!   - [`risk::RiskAnalyzer`] - Coverage-based risk scoring
//!   - [`risk::lcov::parse_lcov_file`] - LCOV format parser
//!   - [`risk::insights::generate_risk_insights`] - Actionable recommendation generation
//!
//! - **[`complexity`]** - Complexity metric calculations
//!   - Cyclomatic complexity (control flow branching)
//!   - Cognitive complexity (human comprehension difficulty)
//!   - Halstead metrics (vocabulary and volume)
//!
//! - **[`io`]** - Input/output formatting and handling
//!   - [`io::output`] - Multiple output formats (JSON, YAML, table)
//!   - [`io::output::OutputWriter`] - Trait for custom output formats
//!
//! - **[`analysis`]** - Advanced analysis algorithms
//!   - [`analysis::RustCallGraph`] - Function call graph construction
//!   - [`analysis::DeadCodeAnalysis`] - Unused code detection
//!   - [`analysis::FrameworkPatternDetector`] - Framework-specific patterns
//!
//! - **[`testing`]** - Test quality analysis
//!   - Test coverage correlation
//!   - Test effectiveness scoring
//!
//! ### Data Flow Principles
//!
//! Debtmap is built on functional programming principles:
//!
//! 1. **Pure Core** - All analysis logic is pure functions with no side effects
//! 2. **I/O at Boundaries** - File operations and network calls isolated to edges
//! 3. **Immutable Data** - Uses persistent data structures for safe concurrent access
//! 4. **Function Composition** - Complex behavior built from simple, testable units
//! 5. **Parallel Processing** - Embarrassingly parallel analysis across files
//!
//! ## CLI Usage
//!
//! For command-line usage, see the [CLI Reference](https://iepathos.github.io/debtmap/cli-reference.html).
//!
//! ```bash
//! # Basic analysis
//! debtmap analyze .
//!
//! # With coverage integration
//! cargo llvm-cov --lcov --output-path target/coverage/lcov.info
//! debtmap analyze . --lcov target/coverage/lcov.info
//!
//! # Generate JSON report
//! debtmap analyze . --format json --output report.json
//! ```
//!
//! ## Examples
//!
//! ### Custom Complexity Thresholds
//!
//! ```rust,no_run
//! use debtmap::{analyzers::get_analyzer, Language};
//!
//! let analyzer = get_analyzer(Language::Rust);
//! let content = std::fs::read_to_string("src/main.rs").unwrap();
//! let ast = analyzer.parse(&content, "src/main.rs".into()).unwrap();
//! let metrics = analyzer.analyze(&ast);
//!
//! // Filter functions by custom complexity threshold
//! let high_complexity_threshold = 10;
//! let complex_functions: Vec<_> = metrics.complexity.functions.iter()
//!     .filter(|f| f.cyclomatic > high_complexity_threshold)
//!     .collect();
//!
//! println!("Found {} highly complex functions", complex_functions.len());
//! for func in complex_functions {
//!     println!("  {} (complexity: {})", func.name, func.cyclomatic);
//! }
//! ```
//!
//! ### Detecting Circular Dependencies
//!
//! ```rust,no_run
//! use debtmap::debt::circular::analyze_module_dependencies;
//! use debtmap::core::Dependency;
//! use std::path::PathBuf;
//!
//! // Example: Analyze module dependencies from parsed files
//! // In practice, you would gather dependencies during file parsing
//! let files: Vec<(PathBuf, Vec<Dependency>)> = vec![
//!     (PathBuf::from("src/main.rs"), vec![]),
//!     (PathBuf::from("src/lib.rs"), vec![]),
//! ];
//!
//! let _dependency_graph = analyze_module_dependencies(&files);
//! // The dependency graph can be used to detect circular dependencies
//! // and analyze module coupling
//! ```
//!
//! ### Generating Risk Insights
//!
//! ```rust,ignore
//! use debtmap::{
//!     analyzers::get_analyzer,
//!     risk::{lcov::parse_lcov_file, RiskAnalyzer, insights::generate_risk_insights},
//!     Language,
//! };
//! use std::path::PathBuf;
//! use im::Vector;
//!
//! // Parse coverage and analyze file
//! let coverage = parse_lcov_file(std::path::Path::new("target/coverage/lcov.info")).unwrap();
//! let analyzer = get_analyzer(Language::Rust);
//! let content = std::fs::read_to_string("src/main.rs").unwrap();
//! let ast = analyzer.parse(&content, "src/main.rs".into()).unwrap();
//! let metrics = analyzer.analyze(&ast);
//!
//! // Calculate risks for all functions
//! let risk_analyzer = RiskAnalyzer::default();
//! let mut risks = Vector::new();
//! let file_path = std::path::Path::new("src/main.rs");
//! for func in &metrics.complexity.functions {
//!     let coverage_pct = coverage.get_function_coverage(file_path, &func.name);
//!     let risk = risk_analyzer.analyze_function(
//!         PathBuf::from("src/main.rs"),
//!         func.name.clone(),
//!         (func.start_line, func.end_line),
//!         &func.complexity,
//!         coverage_pct,
//!         false,
//!     );
//!     risks.push_back(risk);
//! }
//!
//! // Generate actionable insights
//! let insights = generate_risk_insights(risks, &risk_analyzer);
//!
//! // Display top recommendations
//! for rec in insights.risk_reduction_opportunities.iter().take(5) {
//!     println!("{}", rec.recommendation);
//! }
//! ```
//!
//! ## Resources
//!
//! - **Documentation**: [iepathos.github.io/debtmap](https://iepathos.github.io/debtmap/)
//! - **Repository**: [github.com/iepathos/debtmap](https://github.com/iepathos/debtmap)
//! - **Crate**: [crates.io/crates/debtmap](https://crates.io/crates/debtmap)
//! - **Issues**: [github.com/iepathos/debtmap/issues](https://github.com/iepathos/debtmap/issues)
//!
//! ## License
//!
//! Debtmap is licensed under the [MIT License](https://github.com/iepathos/debtmap/blob/master/LICENSE).

// Export modules for library usage
pub mod analysis;
pub mod analysis_utils;
pub mod analyzers;
pub mod builders;
pub mod cli;
pub mod commands;
pub mod common;
pub mod comparison;
pub mod complexity;
pub mod config;
pub mod context;
pub mod core;
pub mod data_flow;
pub mod database;
pub mod debt;
pub mod example_debt;
pub mod extraction_patterns;
pub mod formatting;
pub mod io;
pub mod metrics;
pub mod organization;
pub mod output;
pub mod patterns;
pub mod priority;
pub mod progress;
pub mod refactoring;
pub mod resource;
pub mod risk;
pub mod testing;
pub mod transformers;
pub mod utils;

#[cfg(test)]
mod example_complex_function;
#[cfg(test)]
mod example_refactor;

// Re-export commonly used types
pub use crate::core::{
    AnalysisResults, CircularDependency, ComplexityMetrics, ComplexityReport, ComplexitySummary,
    DebtItem, DebtType, Dependency, DependencyKind, DependencyReport, DuplicationBlock,
    DuplicationLocation, FileMetrics, FunctionMetrics, Language, ModuleDependency, Priority,
    TechnicalDebtReport,
};

pub use crate::debt::{
    circular::{analyze_module_dependencies, DependencyGraph},
    coupling::{calculate_coupling_metrics, identify_coupling_issues, CouplingMetrics},
    duplication::detect_duplication,
    patterns::{
        detect_duplicate_strings, find_code_smells, find_code_smells_with_suppression,
        find_todos_and_fixmes, find_todos_and_fixmes_with_suppression,
    },
    smells::{
        analyze_function_smells, analyze_module_smells, detect_deep_nesting, detect_long_method,
        detect_long_parameter_list, CodeSmell, SmellType,
    },
    suppression::{parse_suppression_comments, SuppressionContext, SuppressionStats},
};

pub use crate::core::metrics::{
    calculate_average_complexity, count_high_complexity, find_max_complexity,
};

pub use crate::io::output::{create_writer, OutputFormat, OutputWriter};

pub use crate::analyzers::{analyze_file, get_analyzer, Analyzer};

pub use crate::risk::{
    insights::generate_risk_insights, lcov::parse_lcov_file, FunctionRisk, RiskAnalyzer,
    RiskCategory, RiskInsight, TestingRecommendation,
};

pub use crate::analysis::{
    AnalysisConfig, CrossModuleTracker, DeadCodeAnalysis, FrameworkPatternDetector,
    FunctionPointerTracker, RustCallGraph, RustCallGraphBuilder, TraitRegistry,
};
