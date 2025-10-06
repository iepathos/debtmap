//! Metrics calculation module
//!
//! Provides unified metrics calculation across all analysis modes.

pub mod loc_counter;

pub use loc_counter::{LocCount, LocCounter, LocCountingConfig, ProjectLocCount};
