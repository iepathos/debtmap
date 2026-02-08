//! Technical debt detection
//!
//! Contains modules for detecting various types of technical debt.

pub mod collection;
pub mod complexity_items;
pub mod organization;
pub mod resource;

pub use collection::{collect_all_rust_debt_items, create_debt_items};
pub use complexity_items::{create_complexity_debt_item, extract_debt_items_with_enhanced};
pub use organization::{analyze_organization_patterns, pattern_to_message_context};
pub use resource::analyze_resource_patterns;
