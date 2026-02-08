//! Core types for purity analysis
//!
//! This module contains all the data structures used by the purity detector
//! for analyzing function purity in Rust code.

use crate::analysis::data_flow::DataFlowAnalysis;
use crate::core::PurityLevel;

/// A mutation of a local variable or owned parameter
#[derive(Debug, Clone)]
pub struct LocalMutation {
    pub target: String,
}

/// A mutation of a closure-captured variable
#[derive(Debug, Clone)]
pub struct UpvalueMutation {
    pub captured_var: String,
}

/// Classification of mutation scope
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationScope {
    /// Mutation of local variable or owned parameter
    Local,
    /// Mutation of closure-captured variable
    Upvalue,
    /// Mutation of external state (fields, statics, etc.)
    External,
}

/// Classification of unsafe operations (Spec 161)
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum UnsafeOp {
    // Pure operations - no side effects
    Transmute,
    RawPointerRead,
    UnionFieldRead,
    PointerArithmetic,

    // Impure operations - side effects
    FFICall,
    RawPointerWrite,
    MutableStatic,
    UnionFieldWrite,
}

/// Classification of path purity for constant detection (Spec 259)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPurity {
    /// Definitely a constant, no purity impact
    Constant,
    /// Likely a constant (e.g., SCREAMING_CASE), reduce confidence slightly
    ProbablyConstant,
    /// Unknown path, conservative: assume external state access
    Unknown,
}

/// Result of purity analysis for a function
#[derive(Debug, Clone)]
pub struct PurityAnalysis {
    pub is_pure: bool,
    pub purity_level: PurityLevel,
    pub reasons: Vec<ImpurityReason>,
    pub confidence: f32,
    pub data_flow_info: Option<DataFlowAnalysis>,
    /// Live local mutations (after filtering out dead stores)
    /// This is useful for "almost pure" analysis - functions with only 1-2 live mutations
    /// are good refactoring candidates
    pub live_mutations: Vec<LocalMutation>,
    /// Total mutations detected (before filtering dead stores)
    pub total_mutations: usize,
    /// Variable name mapping for translating VarIds (from CFG)
    pub var_names: Vec<String>,
}

/// Reasons why a function is not pure
#[derive(Debug, Clone, PartialEq)]
pub enum ImpurityReason {
    SideEffects,
    MutableParameters,
    IOOperations,
    UnsafeCode,
    ModifiesExternalState,
    AccessesExternalState,
}

impl ImpurityReason {
    pub fn description(&self) -> &str {
        match self {
            Self::SideEffects => "Function has side effects",
            Self::MutableParameters => "Function takes mutable parameters",
            Self::IOOperations => "Function performs I/O operations",
            Self::UnsafeCode => "Function contains unsafe code",
            Self::ModifiesExternalState => "Function modifies external state",
            Self::AccessesExternalState => "Function accesses external state",
        }
    }

    /// Get a concise, user-friendly description for TUI display
    pub fn display_description(&self) -> &'static str {
        match self {
            Self::SideEffects => "Has side effects",
            Self::MutableParameters => "Takes &mut self or &mut T",
            Self::IOOperations => "I/O operations (print, file, network)",
            Self::UnsafeCode => "Contains unsafe block",
            Self::ModifiesExternalState => "Modifies external state",
            Self::AccessesExternalState => "Reads external state",
        }
    }
}
