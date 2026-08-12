//! Analysis phases for unified analysis.
//!
//! This module contains focused functions for each phase of the analysis pipeline.
//! Transformations remain pure where practical. I/O belongs at orchestration
//! boundaries, which provide immutable facts to these phases.
//!
//! # Module Organization
//!
//! - [`call_graph`]: Pure call graph construction and enrichment
//! - [`file_analysis`]: Pure file-level metric aggregation
//! - [`god_object`]: Pure god object detection
//! - [`preparation`]: Shared scoring preparation
//! - [`scoring`]: Pure debt scoring and prioritization
//! - [`coverage`]: Coverage data loading (I/O at boundaries)

pub mod call_graph;
pub mod coverage;
pub mod file_analysis;
pub mod god_object;
pub mod preparation;
pub mod scoring;
