//! # Verbosity-Aware Formatter
//!
//! Formats priority items with verbosity control using shared components.
//!
//! ## Shared Components
//!
//! - **Spec 202**: Uses `CoverageLevel` for consistent coverage labels
//! - **Spec 204**: Uses `item.detected_pattern` for pattern display
//! - **Spec 205**: Follows modular structure pattern
//!
//! ## Architecture (Stillwater Philosophy)
//!
//! Each module follows "Pure Core, Imperative Shell":
//! - Pure classification functions for decision logic
//! - Section formatters that compose pure functions for output
//!
//! ## Consistency Guarantees
//!
//! Coverage labels, severity classification, and pattern detection are
//! consistent across all formatters (terminal, markdown, verbosity).

pub mod body;
pub mod complexity;
pub mod context;
pub mod coverage;
pub mod git_history;
pub mod sections;

use crate::formatting::FormattingConfig;
use crate::priority::UnifiedDebtItem;

/// Format priority item with verbosity control
///
/// This is the main entry point for verbosity-aware formatting.
pub use body::format_priority_item_with_config;

/// Legacy function - assumes no coverage data
#[allow(dead_code)]
pub fn format_priority_item_with_verbosity(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    verbosity: u8,
) {
    format_priority_item_with_config(
        output,
        rank,
        item,
        verbosity,
        FormattingConfig::default(),
        false,
    )
}
