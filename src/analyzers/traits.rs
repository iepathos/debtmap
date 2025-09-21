//! Local analyzer traits to decouple analyzer modules

use anyhow::Result;
use std::path::Path;

/// Trait for language-specific analyzers
pub trait LanguageAnalyzer: Send + Sync {
    /// Analyze a file and return metrics
    fn analyze_file(&self, path: &Path, content: &str) -> Result<crate::core::FileMetrics>;

    /// Get the language this analyzer handles
    fn language(&self) -> crate::core::Language;

    /// Check if this analyzer can handle a file
    fn can_handle(&self, path: &Path) -> bool {
        crate::core::Language::from_path(path) == self.language()
    }
}

/// Trait for call graph analysis
pub trait CallGraphAnalyzer: Send + Sync {
    /// Build a call graph for the given content
    fn build_call_graph(&self, content: &str, path: &Path) -> Result<CallGraph>;

    /// Find dependencies for a function
    fn find_dependencies(&self, function: &str, graph: &CallGraph) -> Vec<String>;
}

/// Simple call graph representation
#[derive(Debug, Clone)]
pub struct CallGraph {
    pub nodes: Vec<CallNode>,
    pub edges: Vec<CallEdge>,
}

#[derive(Debug, Clone)]
pub struct CallNode {
    pub name: String,
    pub file: std::path::PathBuf,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct CallEdge {
    pub from: String,
    pub to: String,
    pub call_type: CallType,
}

#[derive(Debug, Clone)]
pub enum CallType {
    Direct,
    Trait,
    Generic,
    Closure,
}

/// Trait for type tracking
pub trait TypeTracker: Send + Sync {
    /// Track type information for a symbol
    fn track_type(&mut self, symbol: &str, type_info: TypeInfo);

    /// Get type information for a symbol
    fn get_type(&self, symbol: &str) -> Option<&TypeInfo>;

    /// Clear all tracked types
    fn clear(&mut self);
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub kind: TypeKind,
    pub generic_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Struct,
    Enum,
    Trait,
    Function,
    Primitive,
    Generic,
}

/// Trait for test detection
pub trait TestDetector: Send + Sync {
    /// Check if a function is a test
    fn is_test_function(&self, name: &str, attributes: &[String]) -> bool;

    /// Check if a module is a test module
    fn is_test_module(&self, path: &Path) -> bool;

    /// Extract test metadata
    fn extract_test_metadata(&self, content: &str) -> TestMetadata;
}

#[derive(Debug, Clone, Default)]
pub struct TestMetadata {
    pub test_count: usize,
    pub assertion_count: usize,
    pub has_setup: bool,
    pub has_teardown: bool,
    pub test_frameworks: Vec<String>,
}

/// Trait for purity analysis
pub trait PurityAnalyzer: Send + Sync {
    /// Check if a function is pure
    fn is_pure(&self, function: &str, body: &str) -> (bool, f32);

    /// Identify side effects in code
    fn find_side_effects(&self, body: &str) -> Vec<SideEffect>;
}

#[derive(Debug, Clone)]
pub struct SideEffect {
    pub kind: SideEffectKind,
    pub location: usize,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum SideEffectKind {
    IO,
    Mutation,
    GlobalState,
    ExternalCall,
    Unsafe,
}
