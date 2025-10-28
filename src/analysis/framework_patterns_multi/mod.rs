//! Multi-Language Framework Pattern Detection
//!
//! This module provides comprehensive framework pattern detection across
//! Rust, Python, and JavaScript/TypeScript codebases. It uses TOML configuration
//! to define patterns for various frameworks and provides a unified API for
//! detecting framework-specific code.
//!
//! # Example
//!
//! ```ignore
//! use debtmap::analysis::framework_patterns_multi::{FrameworkDetector, Language};
//!
//! let detector = FrameworkDetector::from_config("framework_patterns.toml")?;
//! let matches = detector.detect_framework_patterns(&function_ast, &file_context);
//! ```

pub mod cli;
pub mod database;
pub mod detector;
pub mod patterns;
pub mod testing;
pub mod web;

pub use detector::FrameworkDetector;
pub use patterns::{
    FrameworkMatch, FrameworkPattern as MultiLangFrameworkPattern, Language, PatternMatcher,
};
