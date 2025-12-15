//! Unified extraction module for single-pass file parsing.
//!
//! This module provides data types and utilities for extracting all analysis data
//! from source files in a single parse pass. This addresses the proc-macro2 SourceMap
//! overflow issue that occurs when parsing files repeatedly across different analysis phases.
//!
//! # Problem Statement
//!
//! The original analysis pipeline parsed files multiple times:
//! - `populate_io_operations` - per function (~20,000 parses for large codebases)
//! - `extract_variable_deps` - per function
//! - `populate_data_transformations` - per function
//! - Call graph building - all files
//! - Metrics extraction - all files
//! - God object detection - all files
//!
//! For a codebase with 20,000 functions across 2,000 files, this resulted in ~86,000
//! parses instead of 2,000, causing SourceMap overflow and 43x slower analysis.
//!
//! # Solution
//!
//! Parse each file exactly once and extract ALL needed data into `Send+Sync`-safe
//! structures that can be shared across all analysis phases.
//!
//! # Example
//!
//! ```ignore
//! use debtmap::extraction::{UnifiedFileExtractor, ExtractedFileData};
//! use std::path::Path;
//!
//! // Extract all data in one parse pass
//! let content = std::fs::read_to_string("src/main.rs")?;
//! let file_data = UnifiedFileExtractor::extract(Path::new("src/main.rs"), &content)?;
//!
//! // Use extracted data across multiple analysis phases
//! for func in &file_data.functions {
//!     let func_id = func.function_id(&file_data.path);
//!     println!("Function {} at line {}", func.name, func.line);
//! }
//! ```
//!
//! # Module Structure
//!
//! - `types` - Core data types for extracted data (spec 211)
//! - `extractor` - Single-pass file extraction logic (spec 212)
//! - `adapters` - Conversion to existing analysis types (spec 214)

pub mod adapters;
mod extractor;
mod types;

// Re-export all public types
pub use extractor::UnifiedFileExtractor;
pub use types::{
    CallSite, CallType, ExtractedFileData, ExtractedFunctionData, ExtractedImplData,
    ExtractedStructData, FieldInfo, ImportInfo, IoOperation, IoType, MethodInfo, PatternType,
    PurityAnalysisData, PurityLevel, TransformationPattern,
};
