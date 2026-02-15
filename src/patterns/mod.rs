//! Error-prone code pattern detection.
//!
//! This module identifies code patterns that are likely to contain bugs or
//! cause maintenance issues, such as error handling anti-patterns and
//! common programming mistakes.

pub mod error_prone;

pub use error_prone::{check_error_prone_patterns, ErrorPronePattern, PatternType, Severity};
