//! Pure analysis phases for unified analysis.
//!
//! This module contains pure functions for each phase of the analysis pipeline.
//! These functions have no side effects and are easily testable.
//!
//! # Module Organization
//!
//! - [`call_graph`]: Pure call graph construction and enrichment
//! - [`file_analysis`]: Pure file-level metric aggregation
//! - [`god_object`]: Pure god object detection
//! - [`scoring`]: Pure debt scoring and prioritization
//! - [`coverage`]: Coverage data loading (I/O at boundaries)

pub mod call_graph;
pub mod coverage;
pub mod file_analysis;
pub mod god_object;
pub mod scoring;
