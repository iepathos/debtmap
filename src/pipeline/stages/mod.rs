//! Pure analysis stages for the technical debt pipeline.
//!
//! Each module contains pure functions that transform data without performing I/O.
//! These functions are:
//! - Deterministic (same input → same output)
//! - Side-effect free (no logging, no file access, no network calls)
//! - Small and focused (< 20 lines per function)
//! - Easily testable (no mocking required)

pub mod aggregation;
pub mod call_graph;
pub mod debt;
pub mod filtering;
pub mod purity;
pub mod scoring;
