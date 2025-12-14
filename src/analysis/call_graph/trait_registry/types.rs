//! Pure data types for trait registry
//!
//! These types represent trait definitions, implementations, and method calls
//! without any behavior or side effects.

use crate::priority::call_graph::FunctionId;
use im::Vector;

/// Information about a trait method
#[derive(Debug, Clone)]
pub struct TraitMethod {
    /// The trait this method belongs to
    pub trait_name: String,
    /// Method name
    pub method_name: String,
    /// Function ID for this method definition
    pub method_id: FunctionId,
    /// Whether this method has a default implementation
    pub has_default: bool,
}

/// Information about a trait implementation
#[derive(Debug, Clone)]
pub struct TraitImplementation {
    /// The trait being implemented
    pub trait_name: String,
    /// The type implementing the trait
    pub implementing_type: String,
    /// Method implementations
    pub method_implementations: Vector<TraitMethodImplementation>,
    /// Function ID for the impl block (if available)
    pub impl_id: Option<FunctionId>,
}

/// Information about a specific trait method implementation
#[derive(Debug, Clone)]
pub struct TraitMethodImplementation {
    /// Method name
    pub method_name: String,
    /// Function ID for this implementation
    pub method_id: FunctionId,
    /// Whether this overrides a default implementation
    pub overrides_default: bool,
}

/// Information about an unresolved trait method call
#[derive(Debug, Clone)]
pub struct TraitMethodCall {
    /// The caller of the trait method
    pub caller: FunctionId,
    /// The trait name (if determinable)
    pub trait_name: String,
    /// The method name being called
    pub method_name: String,
    /// The receiver type (if determinable)
    pub receiver_type: Option<String>,
    /// Line number of the call
    pub line: usize,
}

/// Statistics about trait usage in the codebase
#[derive(Debug, Clone)]
pub struct TraitStatistics {
    pub total_traits: usize,
    pub total_implementations: usize,
    pub total_unresolved_calls: usize,
}
