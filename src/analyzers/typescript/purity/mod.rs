//! TypeScript/JavaScript purity analysis
//!
//! This module provides purity detection for JavaScript and TypeScript functions,
//! similar to the Rust purity analyzer. It identifies:
//!
//! - **StrictlyPure**: No mutations whatsoever (pure mathematical functions)
//! - **LocallyPure**: Uses local mutations but no external side effects
//! - **ReadOnly**: Reads external state but doesn't modify it
//! - **Impure**: Modifies external state or performs I/O
//!
//! # Key Features
//!
//! - Browser I/O detection (console, fetch, DOM, localStorage, etc.)
//! - Node.js I/O detection (fs, http, process, etc.)
//! - Collection mutation detection (push, pop, splice, etc.)
//! - Scope tracking to distinguish local from external mutations
//! - Confidence scoring based on analysis completeness
//!
//! # Example
//!
//! ```ignore
//! use crate::analyzers::typescript::purity::TypeScriptPurityAnalyzer;
//!
//! let analysis = TypeScriptPurityAnalyzer::analyze(&body_node, source);
//! println!("Purity: {:?}, Confidence: {}", analysis.level, analysis.confidence);
//! ```

mod detector;
mod patterns;
mod scope;
mod types;

pub use detector::TypeScriptPurityAnalyzer;
pub use types::{JsImpurityReason, JsPurityAnalysis};
