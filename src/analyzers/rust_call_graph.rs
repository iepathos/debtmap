/// Call graph extraction for Rust code
///
/// This module provides backward compatibility by re-exporting types from the
/// refactored call_graph module. The functionality has been split into focused
/// submodules for better maintainability:
///
/// - `call_graph::macro_expansion`: Macro parsing and expansion
/// - `call_graph::call_resolution`: Function call resolution  
/// - `call_graph::graph_builder`: Graph construction
/// - `call_graph::trait_handling`: Trait and method resolution
use crate::analyzers::signature_extractor::SignatureExtractor;
use crate::analyzers::type_registry::GlobalTypeRegistry;
use crate::analyzers::function_registry::FunctionSignatureRegistry;
use crate::priority::call_graph::CallGraph;
use std::path::PathBuf;
use std::sync::Arc;

// Re-export everything from the call_graph module for backward compatibility
pub use crate::analyzers::call_graph::{
    CallGraphExtractor,
    MacroExpansionStats,
    MacroHandlingConfig,
    // Re-export other types if needed for compatibility
};

/// Extract call graph from a single Rust file
pub fn extract_call_graph(file: &syn::File, path: &PathBuf) -> CallGraph {
    let extractor = CallGraphExtractor::new(path.clone());
    extractor.extract(file)
}

/// Extract call graph from multiple Rust files
pub fn extract_call_graph_multi_file(files: &[(syn::File, PathBuf)]) -> CallGraph {
    let mut combined_graph = CallGraph::new();

    for (file, path) in files {
        let extractor = CallGraphExtractor::new(path.clone());
        let graph = extractor.extract(file);
        combined_graph.merge(graph);
    }

    combined_graph
}

/// Extract call graph with function signatures
pub fn extract_call_graph_with_signatures(
    file: &syn::File,
    path: &PathBuf,
    _type_registry: Arc<GlobalTypeRegistry>,
) -> (CallGraph, FunctionSignatureRegistry) {
    // Extract function signatures first
    let mut extractor = SignatureExtractor::new();
    extractor.extract_from_file(file);
    let function_registry = extractor.registry;
    
    // Extract call graph
    let call_graph = extract_call_graph(file, path);
    
    (call_graph, function_registry)
}

// For tests that might be importing directly
#[cfg(test)]
pub use crate::analyzers::call_graph::*;
