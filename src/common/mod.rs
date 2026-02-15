//! Common utilities shared across the debtmap codebase.
//!
//! This module provides utility types and functions used by multiple modules,
//! including source code location tracking and text manipulation helpers.
//!
//! Key components:
//! - **Source locations**: Track file, line, and column positions with confidence levels
//! - **Text utilities**: String manipulation helpers like capitalization

pub mod source_location;
pub mod text;

pub use source_location::{LocationConfidence, SourceLocation, UnifiedLocationExtractor};
pub use text::capitalize_first;
