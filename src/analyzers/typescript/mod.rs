//! TypeScript/JavaScript source code analysis
//!
//! This module provides comprehensive analysis of JavaScript and TypeScript source code,
//! including:
//!
//! - Function complexity metrics (cyclomatic, cognitive)
//! - Technical debt detection
//! - Pattern recognition (async/await, promises, callbacks, functional)
//! - Dependency extraction (imports, exports, require)
//!
//! # Example
//!
//! ```ignore
//! use debtmap::analyzers::typescript::TypeScriptAnalyzer;
//! use debtmap::analyzers::Analyzer;
//!
//! let analyzer = TypeScriptAnalyzer::new();
//! let ast = analyzer.parse(source_code, path)?;
//! let metrics = analyzer.analyze(&ast);
//! ```

pub mod analyzer;
pub mod debt;
pub mod dependencies;
pub mod metrics;
pub mod orchestration;
pub mod parser;
pub mod patterns;
pub mod types;
pub mod visitor;

// Re-export main types
pub use analyzer::TypeScriptAnalyzer;
pub use orchestration::analyze_typescript_file;
pub use types::{
    AsyncPattern, FunctionKind, JsDebtPattern, JsFunctionMetrics, TypeScriptPatternResult,
};
