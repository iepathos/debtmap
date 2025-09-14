//! Markdown writer for debt analysis reports
//!
//! This module has been refactored into smaller, focused sub-modules.
//! The main functionality is now organized as follows:
//! - Core writer implementation in `core`
//! - Enhanced writer trait in `enhanced`
//! - Formatting utilities in `formatters`
//! - Dead code analysis in `dead_code`
//! - Testing recommendations in `testing`
//! - Risk analysis in `risk`

mod core;
mod enhanced;
mod formatters;
mod dead_code;
mod testing;
mod risk;

// Re-export main public types and traits
pub use core::MarkdownWriter;
pub use enhanced::EnhancedMarkdownWriter;
