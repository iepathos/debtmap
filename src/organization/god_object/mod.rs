//! God Object Detection Modules
//!
//! This module contains specialized components for detecting and analyzing god objects:
//! - `ast_visitor`: AST traversal and data collection
//! - `metrics`: Scoring and metric calculations
//! - `classifier`: Pattern detection and classification
//! - `recommender`: Recommendation generation

pub mod ast_visitor;
pub mod metrics;

pub use ast_visitor::{FunctionWeight, Responsibility, TypeAnalysis, TypeVisitor};
