use crate::analyzers::function_registry::FunctionSignatureRegistry;
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
use crate::priority::call_graph::CallGraph;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// Re-export everything from the call_graph module for backward compatibility
pub use crate::analyzers::call_graph::{
    CallGraphExtractor,
    MacroExpansionStats,
    MacroHandlingConfig,
    // Re-export other types if needed for compatibility
};

/// Extract call graph from a single Rust file
pub fn extract_call_graph(file: &syn::File, path: &Path) -> CallGraph {
    let extractor = CallGraphExtractor::new(path.to_path_buf());
    extractor.extract(file)
}

/// Extract call graph from multiple Rust files
pub fn extract_call_graph_multi_file(files: &[(syn::File, PathBuf)]) -> CallGraph {
    let mut combined_graph = CallGraph::new();
    let mut all_unresolved_calls = Vec::new();

    // Phase 1: Extract all functions and collect unresolved calls from all files
    for (file, path) in files {
        let mut extractor = CallGraphExtractor::new(path.clone());
        // Extract functions and collect unresolved calls
        extractor.extract_phase1(file);

        // Merge the functions into the combined graph
        combined_graph.merge(extractor.graph_builder.call_graph.clone());

        // Collect unresolved calls for later resolution
        all_unresolved_calls.extend(extractor.unresolved_calls.clone());
    }

    // Phase 2: Resolve all calls now that we have all functions
    let multi_file_path = PathBuf::from("multi-file");
    let mut resolved_calls = Vec::new();

    {
        let resolver =
            crate::analyzers::call_graph::CallResolver::new(&combined_graph, &multi_file_path);

        for unresolved in &all_unresolved_calls {
            if let Some(callee) = resolver.resolve_call(unresolved) {
                resolved_calls.push(crate::priority::call_graph::FunctionCall {
                    caller: unresolved.caller.clone(),
                    callee,
                    call_type: unresolved.call_type.clone(),
                });
            }
        }
    }

    // Add all resolved calls
    for call in resolved_calls {
        combined_graph.add_call(call);
    }

    // Phase 3: Final cross-file resolution for any remaining unresolved calls
    combined_graph.resolve_cross_file_calls();

    combined_graph
}

/// Extract call graph with function signatures
pub fn extract_call_graph_with_signatures(
    file: &syn::File,
    path: &Path,
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
