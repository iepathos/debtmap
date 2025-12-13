//! Data flow analysis for Rust code.
//!
//! This module provides control flow graph construction and data flow
//! analysis algorithms for analyzing variable definitions, uses, and
//! function purity.
//!
//! # Architecture Overview
//!
//! The analysis pipeline consists of three main phases:
//!
//! 1. **CFG Construction**: Parse Rust AST into a control flow graph
//! 2. **Reaching Definitions**: Track which definitions reach each program point
//! 3. **Def-Use Chains**: Build precise mappings between definitions and uses
//!
//! # Module Structure
//!
//! - [`call_classification`] - Database of known pure/impure functions
//! - [`types`] - Core CFG types (blocks, edges, variables)
//! - [`reaching_definitions`] - Data flow analysis algorithm
//! - [`cfg_builder`] - AST-to-CFG transformation
//!
//! # Design Decisions
//!
//! ## Intra-procedural Only
//!
//! The analysis is intentionally **intra-procedural** (within a single function).
//! Inter-procedural analysis (across functions) is significantly more complex and
//! has diminishing returns for technical debt detection.
//!
//! **Trade-off**: We accept some false positives (e.g., calling a pure helper function
//! might be flagged as impure) in exchange for:
//! - Faster analysis (< 10ms per function target)
//! - Simpler implementation
//! - No need for whole-program analysis
//!
//! ## Simplified CFG
//!
//! The CFG uses simplified variable extraction with temporary placeholders (e.g., `_temp0`)
//! for complex expressions. This is a pragmatic trade-off:
//!
//! **Trade-off**: We lose precise tracking of expressions like `x.y.z` in exchange for:
//! - Simpler CFG construction
//! - Faster analysis
//! - Good enough accuracy for debt detection
//!
//! # Example
//!
//! ```ignore
//! use debtmap::analysis::data_flow::{DataFlowAnalysis, ControlFlowGraph};
//! use syn::parse_quote;
//!
//! let block = parse_quote! {
//!     {
//!         let mut x = 1;
//!         x = x + 1;
//!         x
//!     }
//! };
//!
//! let cfg = ControlFlowGraph::from_block(&block);
//! let analysis = DataFlowAnalysis::analyze(&cfg);
//! ```
//!
//! # Performance Characteristics
//!
//! **Target**: < 10ms per function, < 20% overhead on total analysis time
//!
//! **Actual** (as of implementation):
//! - CFG construction: ~1-2ms per function (simple functions)
//! - Reaching definitions: ~0.5-1ms
//!
//! **Total**: ~1.5-3ms per function for typical code (well under 10ms target)

pub mod call_classification;
mod cfg_builder;
pub mod reaching_definitions;
pub mod types;

// Re-export public API from call_classification
pub use call_classification::{
    classify_call, is_known_impure, is_known_pure, CallPurity, UnknownCallBehavior,
    KNOWN_IMPURE_FUNCTIONS, KNOWN_PURE_FUNCTIONS,
};

// Re-export public API from types
pub use types::{
    BasicBlock, BinOp, BlockId, CaptureMode, CapturedVar, ControlFlowGraph, Definition, Edge,
    ExprKind, MatchArm, ProgramPoint, Rvalue, Statement, StatementIdx, Terminator, UnOp, Use,
    VarId,
};

// Re-export public API from reaching_definitions
pub use reaching_definitions::{DataFlowAnalysis, ReachingDefinitions};
