//! Comparison utilities for analyzing differences between analysis results.
//!
//! This module provides functionality for comparing debtmap analysis results,
//! matching source locations between runs, and parsing implementation plans
//! to track progress on debt reduction.
//!
//! Key components:
//! - **Comparator**: Compare two analysis results to find improvements/regressions
//! - **Location matcher**: Match debt items across analysis runs by source location
//! - **Plan parser**: Parse and track implementation plans for debt reduction
//!
//! The comparison module is essential for validating that code changes
//! actually improve technical debt metrics.

pub mod comparator;
pub mod location_matcher;
pub mod plan_parser;
pub mod types;

pub use comparator::Comparator;
pub use location_matcher::{LocationMatcher, LocationPattern, MatchResult, MatchStrategy};
pub use plan_parser::PlanParser;
pub use types::*;
