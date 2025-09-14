//! Markdown writer module for debt analysis reports
//!
//! This module provides markdown output functionality split into focused sub-modules:
//! - `core`: Core MarkdownWriter struct and OutputWriter trait implementation
//! - `enhanced`: EnhancedMarkdownWriter trait and implementation
//! - `formatters`: Pure formatting functions for debt types, visibility, etc.
//! - `dead_code`: Dead code analysis and formatting functionality
//! - `testing`: Testing recommendation functions
//! - `risk`: Risk analysis output functions

pub mod core;
pub mod dead_code;
pub mod enhanced;
pub mod formatters;
pub mod risk;
pub mod testing;

// Re-export main public types
pub use core::MarkdownWriter;
pub use enhanced::EnhancedMarkdownWriter;
