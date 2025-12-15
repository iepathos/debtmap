//! Extraction adapters for converting extracted data to analysis types.
//!
//! This module provides pure conversion functions that bridge extracted file data
//! to the existing analysis types used throughout the codebase.
//!
//! # Architecture
//!
//! The adapters layer sits between the unified extraction module and the various
//! analysis phases:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     Source Files                                 │
//! └─────────────────────┬───────────────────────────────────────────┘
//!                       │ Single Parse
//!                       ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │           UnifiedFileExtractor (spec 212)                        │
//! │           ExtractedFileData (spec 211)                           │
//! └─────────────────────┬───────────────────────────────────────────┘
//!                       │
//!        ┌──────────────┼──────────────┬──────────────┐
//!        ▼              ▼              ▼              ▼
//! ┌─────────────┐ ┌───────────┐ ┌───────────┐ ┌─────────────┐
//! │   Metrics   │ │CallGraph  │ │ DataFlow  │ │ GodObject   │
//! │   Adapter   │ │ Adapter   │ │ Adapter   │ │  Adapter    │
//! └──────┬──────┘ └─────┬─────┘ └─────┬─────┘ └──────┬──────┘
//!        │              │             │              │
//!        ▼              ▼             ▼              ▼
//! ┌─────────────┐ ┌───────────┐ ┌───────────┐ ┌─────────────┐
//! │Function     │ │CallGraph  │ │DataFlow   │ │GodObject    │
//! │Metrics,     │ │           │ │Graph      │ │Analysis     │
//! │FileMetrics  │ │           │ │           │ │             │
//! └─────────────┘ └───────────┘ └───────────┘ └─────────────┘
//! ```
//!
//! # Design Principles
//!
//! 1. **Pure Functions**: All adapters are pure functions with no I/O
//! 2. **O(n) Performance**: Conversions are linear in input size
//! 3. **No Parsing**: Adapters never re-parse source code
//! 4. **Testable**: Each adapter can be tested in isolation
//!
//! # Usage
//!
//! ```ignore
//! use debtmap::extraction::adapters;
//! use debtmap::extraction::UnifiedFileExtractor;
//! use std::collections::HashMap;
//!
//! // Extract all files first
//! let extracted: HashMap<PathBuf, ExtractedFileData> = files
//!     .iter()
//!     .filter_map(|path| {
//!         let content = std::fs::read_to_string(path).ok()?;
//!         UnifiedFileExtractor::extract(path, &content).ok()
//!     })
//!     .map(|data| (data.path.clone(), data))
//!     .collect();
//!
//! // Convert to metrics
//! let all_metrics = adapters::metrics::all_metrics_from_extracted(&extracted);
//!
//! // Build call graph
//! let call_graph = adapters::call_graph::build_call_graph(&extracted);
//!
//! // Populate data flow
//! let mut data_flow = DataFlowGraph::from_call_graph(call_graph.clone());
//! adapters::data_flow::populate_data_flow(&mut data_flow, &extracted);
//!
//! // Analyze god objects
//! let god_objects = adapters::god_object::analyze_all_files(&extracted);
//! ```
//!
//! # Modules
//!
//! - [`metrics`]: Convert to `FunctionMetrics` and `FileMetrics`
//! - [`call_graph`]: Build `CallGraph` from extracted calls
//! - [`data_flow`]: Populate `DataFlowGraph` with purity, I/O, and transformations
//! - [`god_object`]: Analyze for god object patterns

pub mod call_graph;
pub mod data_flow;
pub mod god_object;
pub mod metrics;

// Re-export commonly used functions for convenience
pub use call_graph::build_call_graph;
pub use data_flow::{populate_data_flow, PopulationStats};
pub use god_object::{analyze_god_object, GodObjectThresholds};
pub use metrics::{
    all_file_metrics_from_extracted, all_function_metrics, all_metrics_from_extracted,
    to_file_metrics, to_function_metrics,
};
