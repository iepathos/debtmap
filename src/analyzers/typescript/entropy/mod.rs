//! Entropy-based complexity analysis for TypeScript/JavaScript
//!
//! This module provides entropy analysis for JavaScript/TypeScript code,
//! enabling cognitive complexity dampening for repetitive patterns.
//!
//! # Architecture
//!
//! - `token`: Token extraction from tree-sitter AST
//! - `patterns`: Pattern detection for JS/TS idioms
//! - `branches`: Branch similarity analysis
//! - `analyzer`: Core `LanguageEntropyAnalyzer` implementation

pub mod analyzer;
pub mod branches;
pub mod patterns;
pub mod token;

pub use analyzer::{calculate_entropy, JsEntropyAnalyzer};
