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
    let start_time = std::time::Instant::now();
    let mut combined_graph = CallGraph::new();
    let mut all_unresolved_calls = Vec::new();

    // Build PathResolver for import-aware resolution
    use crate::analyzers::call_graph::PathResolverBuilder;
    let mut path_resolver_builder = PathResolverBuilder::new();

    // Phase 1: Extract all functions, collect unresolved calls, and analyze imports
    let phase1_start = std::time::Instant::now();
    let total_files = files.len();
    let phase1_progress = crate::progress::ProgressManager::global()
        .map(|pm| {
            let pb = pm.create_bar(
                total_files as u64,
                "[analyze] {msg} {pos}/{len} files ({percent}%) - {eta}",
            );
            pb.set_message("Analyzing functions and imports");
            pb
        })
        .unwrap_or_else(indicatif::ProgressBar::hidden);

    for (file, path) in files {
        let mut extractor = CallGraphExtractor::new(path.clone());
        // Extract functions and collect unresolved calls
        extractor.extract_phase1(file);

        // Merge the functions into the combined graph
        combined_graph.merge(extractor.graph_builder.call_graph.clone());

        // Collect unresolved calls for later resolution
        all_unresolved_calls.extend(extractor.unresolved_calls.clone());

        // Analyze imports for this file
        path_resolver_builder = path_resolver_builder.analyze_file(path.clone(), file);

        phase1_progress.inc(1);
    }

    let phase1_duration = phase1_start.elapsed();
    log::info!(
        "Phase 1 completed in {:.2}s: {} files, {} functions, {} unresolved calls",
        phase1_duration.as_secs_f64(),
        total_files,
        combined_graph.get_all_functions().count(),
        all_unresolved_calls.len()
    );

    phase1_progress.finish_with_message(format!(
        "Analyzed {} files, {} unresolved calls ({}s)",
        total_files,
        all_unresolved_calls.len(),
        phase1_duration.as_secs()
    ));

    let resolver_build_start = std::time::Instant::now();
    let path_resolver = path_resolver_builder
        .build()
        .with_function_index(&combined_graph);
    let resolver_build_duration = resolver_build_start.elapsed();
    log::info!(
        "PathResolver built in {:.2}s",
        resolver_build_duration.as_secs_f64()
    );

    // Phase 2: Resolve all calls now that we have all functions and imports
    let phase2_start = std::time::Instant::now();
    let multi_file_path = PathBuf::from("multi-file");
    let mut resolved_calls = Vec::new();

    let total_unresolved = all_unresolved_calls.len();
    let phase2_progress = crate::progress::ProgressManager::global()
        .map(|pm| {
            let pb = pm.create_bar(
                total_unresolved as u64,
                "[resolve] {msg} {pos}/{len} calls ({percent}%) - {eta}",
            );
            pb.set_message("Resolving function calls");
            pb
        })
        .unwrap_or_else(indicatif::ProgressBar::hidden);

    let mut call_resolver_hits = 0;
    let mut path_resolver_hits = 0;
    let mut unresolved_count = 0;

    {
        let resolver_build_start = std::time::Instant::now();
        let resolver =
            crate::analyzers::call_graph::CallResolver::new(&combined_graph, &multi_file_path);
        log::info!(
            "CallResolver built in {:.2}s",
            resolver_build_start.elapsed().as_secs_f64()
        );

        for (idx, unresolved) in all_unresolved_calls.iter().enumerate() {
            // Try standard resolution first
            if let Some(callee) = resolver.resolve_call(unresolved) {
                resolved_calls.push(crate::priority::call_graph::FunctionCall {
                    caller: unresolved.caller.clone(),
                    callee,
                    call_type: unresolved.call_type.clone(),
                });
                call_resolver_hits += 1;
            } else {
                // If standard resolution fails, try using PathResolver
                // This handles both simple names (via imports) and qualified paths
                if let Some(callee) =
                    path_resolver.resolve_call(&unresolved.caller.file, &unresolved.callee_name)
                {
                    resolved_calls.push(crate::priority::call_graph::FunctionCall {
                        caller: unresolved.caller.clone(),
                        callee,
                        call_type: unresolved.call_type.clone(),
                    });
                    path_resolver_hits += 1;
                } else {
                    unresolved_count += 1;
                }
            }

            // Update progress every 100 calls to avoid overhead
            if idx % 100 == 0 || idx == total_unresolved - 1 {
                phase2_progress.set_position((idx + 1) as u64);
            }

            // Log progress every 1000 calls
            if idx > 0 && idx % 1000 == 0 {
                log::info!(
                    "Phase 2 progress: {}/{} calls ({:.1}%), CallResolver: {}, PathResolver: {}, Unresolved: {}",
                    idx,
                    total_unresolved,
                    (idx as f64 / total_unresolved as f64) * 100.0,
                    call_resolver_hits,
                    path_resolver_hits,
                    unresolved_count
                );
            }
        }
    }

    let phase2_duration = phase2_start.elapsed();
    log::info!(
        "Phase 2 completed in {:.2}s: CallResolver: {}, PathResolver: {}, Still unresolved: {}",
        phase2_duration.as_secs_f64(),
        call_resolver_hits,
        path_resolver_hits,
        unresolved_count
    );

    phase2_progress.finish_with_message(format!(
        "Resolved {}/{} calls ({}s)",
        resolved_calls.len(),
        total_unresolved,
        phase2_duration.as_secs()
    ));

    // Add all resolved calls
    for call in resolved_calls {
        combined_graph.add_call(call);
    }

    // Phase 3: Final cross-file resolution for any remaining unresolved calls
    // This has its own progress tracking inside resolve_cross_file_calls()
    let phase3_start = std::time::Instant::now();
    combined_graph.resolve_cross_file_calls();
    let phase3_duration = phase3_start.elapsed();
    log::info!("Phase 3 completed in {:.2}s", phase3_duration.as_secs_f64());

    let total_duration = start_time.elapsed();
    log::info!(
        "extract_call_graph_multi_file total time: {:.2}s (Phase1: {:.2}s, Phase2: {:.2}s, Phase3: {:.2}s)",
        total_duration.as_secs_f64(),
        phase1_duration.as_secs_f64(),
        phase2_duration.as_secs_f64(),
        phase3_duration.as_secs_f64()
    );

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
