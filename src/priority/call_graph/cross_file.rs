//! Cross-file call resolution for handling method calls across modules

use super::types::{CallGraph, FunctionCall, FunctionId};
#[allow(unused_imports)]
use std::path::{Path, PathBuf};

impl CallGraph {
    /// Build a map of all functions by name
    #[allow(dead_code)]
    fn build_function_name_map(&self) -> std::collections::HashMap<String, Vec<FunctionId>> {
        let mut functions_by_name = std::collections::HashMap::new();
        for func_id in self.nodes.keys() {
            functions_by_name
                .entry(func_id.name.clone())
                .or_insert_with(Vec::new)
                .push(func_id.clone());
        }
        functions_by_name
    }

    /// Identify calls that need resolution (line 0 indicates unresolved)
    fn find_unresolved_calls(&self) -> Vec<FunctionCall> {
        self.edges
            .iter()
            .filter(|call| call.callee.line == 0)
            .cloned()
            .collect()
    }

    /// Pure function to resolve a cross-file call
    ///
    /// This function handles complex cases like:
    /// - Associated function calls (Type::method matching function stored as Type::method)
    /// - Qualified path resolution
    /// - Cross-module calls with type hints
    ///
    /// # Thread Safety
    ///
    /// This function is safe for concurrent execution because it:
    /// - Takes only immutable references
    /// - Returns new data without modifying inputs
    /// - Has no side effects or shared mutable state
    /// - Is `Send + Sync` and can be safely called from multiple threads
    fn resolve_call_with_advanced_matching(
        all_functions: &[FunctionId],
        callee_name: &str,
        caller_file: &PathBuf,
    ) -> Option<FunctionId> {
        use crate::analyzers::call_graph::call_resolution::CallResolver;

        // Delegate to the sophisticated CallResolver logic
        CallResolver::resolve_function_call(
            all_functions,
            callee_name,
            caller_file,
            false, // Don't force same-file preference for cross-file resolution
        )
    }

    /// Pure function to check if two function names could be the same call
    /// Handles various call patterns:
    /// - Exact match: "func" matches "func"
    /// - Associated function: "Type::method" matches "Type::method"
    /// - Method call resolution: "method" might match "Type::method" if we have type context
    #[allow(dead_code)]
    #[cfg(test)]
    pub fn is_cross_file_call_match(
        stored_function_name: &str,
        call_name: &str,
        type_context: Option<&str>,
    ) -> bool {
        // 1. Exact match
        if stored_function_name == call_name {
            return true;
        }

        // 2. Associated function call pattern
        // If call_name contains "::" it's likely an associated function call
        if call_name.contains("::") && stored_function_name == call_name {
            return true;
        }

        // 3. Method name matching with type context
        if let Some(type_name) = type_context {
            let expected_qualified_name = format!("{}::{}", type_name, call_name);
            if stored_function_name == expected_qualified_name {
                return true;
            }
        }

        // 4. Suffix matching for qualified paths
        // "module::Type::method" matches "Type::method"
        if stored_function_name.ends_with(&format!("::{}", call_name)) {
            return true;
        }

        // 5. Extract base name from stored function for method matching
        if let Some(pos) = stored_function_name.rfind("::") {
            let base_name = &stored_function_name[pos + 2..];
            if base_name == call_name {
                return true;
            }
        }

        false
    }

    /// Pure function to select the best matching function from candidates
    /// Applies preference rules:
    /// 1. Same file preference (when hint suggests it)
    /// 2. Least qualified name (simpler is better)
    /// 3. Exact matches over partial matches
    #[allow(dead_code)]
    #[cfg(test)]
    pub fn select_best_cross_file_match(
        candidates: Vec<FunctionId>,
        caller_file: &PathBuf,
        call_name: &str,
    ) -> Option<FunctionId> {
        if candidates.is_empty() {
            return None;
        }

        if candidates.len() == 1 {
            return candidates.into_iter().next();
        }

        // Prefer exact matches first
        let exact_matches: Vec<_> = candidates
            .iter()
            .filter(|func| func.name == call_name)
            .cloned()
            .collect();

        if !exact_matches.is_empty() {
            return Self::apply_file_and_qualification_preference(exact_matches, caller_file);
        }

        // Then prefer cross-file matches (different file, which is what we're resolving)
        let cross_file_matches: Vec<_> = candidates
            .iter()
            .filter(|func| &func.file != caller_file)
            .cloned()
            .collect();

        if !cross_file_matches.is_empty() {
            return Self::apply_file_and_qualification_preference(cross_file_matches, caller_file);
        }

        // Fallback to any match
        Self::apply_file_and_qualification_preference(candidates, caller_file)
    }

    /// Pure function to apply file and qualification preferences
    #[allow(dead_code)]
    #[cfg(test)]
    pub fn apply_file_and_qualification_preference(
        candidates: Vec<FunctionId>,
        _caller_file: &Path,
    ) -> Option<FunctionId> {
        if candidates.is_empty() {
            return None;
        }

        if candidates.len() == 1 {
            return candidates.into_iter().next();
        }

        // Prefer less qualified names (simpler is better)
        let min_colons = candidates
            .iter()
            .map(|func| func.name.matches("::").count())
            .min()
            .unwrap_or(0);

        candidates
            .into_iter()
            .find(|func| func.name.matches("::").count() == min_colons)
    }

    /// Apply a resolved call to the graph's indexes and edges
    fn apply_call_resolution(
        &mut self,
        original_call: &FunctionCall,
        resolved_callee: &FunctionId,
    ) {
        // Remove old unresolved call from indexes
        if let Some(callee_set) = self.callee_index.get_mut(&original_call.caller) {
            callee_set.remove(&original_call.callee);
            callee_set.insert(resolved_callee.clone());
        }

        if let Some(caller_set) = self.caller_index.get_mut(&original_call.callee) {
            caller_set.remove(&original_call.caller);
        }

        // Add to the resolved callee's caller index
        self.caller_index
            .entry(resolved_callee.clone())
            .or_default()
            .insert(original_call.caller.clone());

        // Update the edge
        for edge in self.edges.iter_mut() {
            if edge.caller == original_call.caller && edge.callee == original_call.callee {
                edge.callee = resolved_callee.clone();
                break;
            }
        }
    }

    /// Resolve cross-file function calls using parallel processing
    ///
    /// This method processes unresolved calls in two phases:
    /// 1. **Parallel Resolution**: Uses Rayon to resolve calls concurrently
    ///    across multiple CPU cores, leveraging the pure functional nature
    ///    of the resolution logic.
    /// 2. **Sequential Updates**: Applies all resolutions to the graph
    ///    sequentially to maintain data structure consistency.
    ///
    /// # Performance
    ///
    /// Expected speedup: 10-15% on multi-core systems (4-8 cores).
    /// Scales linearly with number of unresolved calls and available cores.
    ///
    /// # Memory Usage
    ///
    /// Memory overhead is minimal and predictable:
    /// - Stores resolved call pairs in a `Vec<(FunctionCall, FunctionId)>`
    /// - For typical codebases with 1000-2000 unresolved calls:
    ///   - Each tuple: ~200-300 bytes (two FunctionId + one FunctionCall)
    ///   - Total overhead: ~200KB-600KB for 1000-2000 resolutions
    /// - Peak memory during phase 1 (parallel resolution)
    /// - Memory freed after phase 2 (sequential updates)
    /// - Well under the 10MB budget specified in requirements
    ///
    /// # Thread Safety
    ///
    /// The resolution phase is thread-safe because:
    /// - Resolution logic is pure (no side effects)
    /// - All input data is immutable during resolution
    /// - No shared mutable state between threads
    pub fn resolve_cross_file_calls(&mut self) {
        use rayon::prelude::*;

        let all_functions: Vec<FunctionId> = self.get_all_functions().cloned().collect();
        let calls_to_resolve = self.find_unresolved_calls();

        // Phase 1: Parallel resolution (read-only, no mutation)
        // This phase can utilize all CPU cores for independent resolutions
        let resolutions: Vec<(FunctionCall, FunctionId)> = calls_to_resolve
            .par_iter()
            .filter_map(|call| {
                // Pure function call - safe for parallel execution
                Self::resolve_call_with_advanced_matching(
                    &all_functions,
                    &call.callee.name,
                    &call.caller.file,
                )
                .map(|resolved_callee| {
                    // Return tuple of (original_call, resolved_callee)
                    (call.clone(), resolved_callee)
                })
            })
            .collect();

        // Phase 2: Sequential bulk update (mutation phase)
        // Apply all resolutions to the graph in sequence
        for (original_call, resolved_callee) in resolutions {
            self.apply_call_resolution(&original_call, &resolved_callee);
        }
    }

    /// Sequential resolution for testing and benchmarking
    ///
    /// This method provides a non-parallel baseline for comparison with the
    /// parallel `resolve_cross_file_calls()` method. It's primarily used for:
    /// - Verifying correctness of parallel implementation (determinism tests)
    /// - Performance benchmarking and comparison
    /// - Debugging and development
    ///
    /// In production, prefer `resolve_cross_file_calls()` for better performance.
    pub fn resolve_cross_file_calls_sequential(&mut self) {
        let all_functions: Vec<FunctionId> = self.get_all_functions().cloned().collect();
        let calls_to_resolve = self.find_unresolved_calls();

        for call in calls_to_resolve {
            if let Some(resolved_callee) = Self::resolve_call_with_advanced_matching(
                &all_functions,
                &call.callee.name,
                &call.caller.file,
            ) {
                self.apply_call_resolution(&call, &resolved_callee);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::call_graph::CallType;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::thread;

    /// Helper to create a test graph with unresolved cross-file calls
    fn create_test_graph_with_unresolved_calls() -> CallGraph {
        let mut graph = CallGraph::new();

        // Add functions in different files
        for i in 0..10 {
            let func_id = FunctionId::new(
                PathBuf::from(format!("file_{}.rs", i)),
                format!("function_{}", i),
                i * 10,
            );
            graph.add_function(func_id, i == 0, false, 5, 50);
        }

        // Add unresolved cross-file calls (line = 0 indicates unresolved)
        for i in 0..5 {
            let caller = FunctionId::new(
                PathBuf::from(format!("file_{}.rs", i)),
                format!("function_{}", i),
                i * 10,
            );
            let callee = FunctionId {
                file: PathBuf::from("unknown.rs"),
                name: format!("function_{}", i + 1),
                line: 0, // Unresolved
                module_path: String::new(),
            };
            graph.add_call(FunctionCall {
                caller,
                callee,
                call_type: CallType::Direct,
            });
        }

        graph
    }

    /// Test that parallel and sequential resolution produce identical results
    /// This ensures correctness of the parallel implementation
    #[test]
    fn test_parallel_sequential_determinism() {
        let graph1 = create_test_graph_with_unresolved_calls();
        let graph2 = graph1.clone();

        let mut parallel_graph = graph1;
        let mut sequential_graph = graph2;

        // Resolve using both methods
        parallel_graph.resolve_cross_file_calls();
        sequential_graph.resolve_cross_file_calls_sequential();

        // Verify edges are identical
        assert_eq!(
            parallel_graph.edges.len(),
            sequential_graph.edges.len(),
            "Edge count mismatch between parallel and sequential"
        );

        let parallel_edges: HashSet<_> = parallel_graph.edges.iter().collect();
        let sequential_edges: HashSet<_> = sequential_graph.edges.iter().collect();

        assert_eq!(
            parallel_edges, sequential_edges,
            "Edge sets differ between parallel and sequential resolution"
        );

        // Verify caller_index is identical
        assert_eq!(
            parallel_graph.caller_index.len(),
            sequential_graph.caller_index.len(),
            "Caller index size mismatch"
        );

        for (func_id, parallel_callers) in &parallel_graph.caller_index {
            let sequential_callers = sequential_graph
                .caller_index
                .get(func_id)
                .expect("Function missing in sequential caller_index");
            assert_eq!(
                parallel_callers, sequential_callers,
                "Caller sets differ for function {:?}",
                func_id
            );
        }

        // Verify callee_index is identical
        assert_eq!(
            parallel_graph.callee_index.len(),
            sequential_graph.callee_index.len(),
            "Callee index size mismatch"
        );

        for (func_id, parallel_callees) in &parallel_graph.callee_index {
            let sequential_callees = sequential_graph
                .callee_index
                .get(func_id)
                .expect("Function missing in sequential callee_index");
            assert_eq!(
                parallel_callees, sequential_callees,
                "Callee sets differ for function {:?}",
                func_id
            );
        }
    }

    /// Test multiple runs of parallel resolution produce identical results
    #[test]
    fn test_parallel_resolution_determinism() {
        let base_graph = create_test_graph_with_unresolved_calls();

        let mut graph1 = base_graph.clone();
        let mut graph2 = base_graph.clone();
        let mut graph3 = base_graph;

        graph1.resolve_cross_file_calls();
        graph2.resolve_cross_file_calls();
        graph3.resolve_cross_file_calls();

        // All three should produce identical results
        let edges1: HashSet<_> = graph1.edges.iter().collect();
        let edges2: HashSet<_> = graph2.edges.iter().collect();
        let edges3: HashSet<_> = graph3.edges.iter().collect();

        assert_eq!(edges1, edges2, "First and second parallel runs differ");
        assert_eq!(edges2, edges3, "Second and third parallel runs differ");
    }

    /// Test thread safety - concurrent execution should not panic or race
    #[test]
    fn test_concurrent_resolution_thread_safety() {
        let num_threads = 8;
        let mut handles = vec![];

        // Spawn multiple threads calling resolve_call_with_advanced_matching concurrently
        for thread_id in 0..num_threads {
            let handle = thread::spawn(move || {
                // Create test data
                let all_functions: Vec<FunctionId> = (0..50)
                    .map(|i| {
                        FunctionId::new(
                            PathBuf::from(format!("file_{}.rs", i % 10)),
                            format!("function_{}", i),
                            i * 10,
                        )
                    })
                    .collect();

                let caller_file = PathBuf::from(format!("caller_{}.rs", thread_id));

                // Perform multiple resolutions
                let mut results = vec![];
                for i in 0..100 {
                    let callee_name = format!("function_{}", i % 50);
                    let result = CallGraph::resolve_call_with_advanced_matching(
                        &all_functions,
                        &callee_name,
                        &caller_file,
                    );
                    results.push(result);
                }

                results
            });
            handles.push(handle);
        }

        // Wait for all threads and verify no panics occurred
        for handle in handles {
            handle
                .join()
                .expect("Thread panicked during concurrent resolution");
        }
    }

    /// Test thread safety with shared graph data
    #[test]
    fn test_concurrent_resolution_with_shared_data() {
        let graph = Arc::new(create_test_graph_with_unresolved_calls());
        let all_functions: Vec<FunctionId> = graph.get_all_functions().cloned().collect();
        let all_functions = Arc::new(all_functions);

        let num_threads = 8;
        let mut handles = vec![];

        for thread_id in 0..num_threads {
            let functions = Arc::clone(&all_functions);
            let handle = thread::spawn(move || {
                let caller_file = PathBuf::from(format!("file_{}.rs", thread_id % 10));

                // Perform resolutions with shared data
                let mut results = vec![];
                for i in 0..50 {
                    let callee_name = format!("function_{}", i % 10);
                    let result = CallGraph::resolve_call_with_advanced_matching(
                        &functions,
                        &callee_name,
                        &caller_file,
                    );
                    results.push(result);
                }

                results
            });
            handles.push(handle);
        }

        // Verify all threads complete successfully
        for handle in handles {
            handle.join().expect("Thread panicked with shared data");
        }
    }

    /// Test that unresolved calls are properly identified
    #[test]
    fn test_find_unresolved_calls() {
        let graph = create_test_graph_with_unresolved_calls();
        let unresolved = graph.find_unresolved_calls();

        // Should find 5 unresolved calls (line = 0)
        assert_eq!(unresolved.len(), 5, "Expected 5 unresolved calls");

        for call in unresolved {
            assert_eq!(call.callee.line, 0, "Unresolved call should have line = 0");
        }
    }

    /// Test resolution with no unresolved calls
    #[test]
    fn test_resolution_with_no_unresolved_calls() {
        let mut graph = CallGraph::new();

        // Add only resolved calls
        for i in 0..5 {
            let caller = FunctionId::new(
                PathBuf::from(format!("file_{}.rs", i)),
                format!("func_{}", i),
                i * 10,
            );
            let callee = FunctionId::new(
                PathBuf::from(format!("file_{}.rs", i + 1)),
                format!("func_{}", i + 1),
                (i + 1) * 10,
            );

            graph.add_function(caller.clone(), false, false, 5, 50);
            graph.add_function(callee.clone(), false, false, 5, 50);
            graph.add_call(FunctionCall {
                caller,
                callee,
                call_type: CallType::Direct,
            });
        }

        let edges_before = graph.edges.len();

        // Resolution should do nothing
        graph.resolve_cross_file_calls();

        assert_eq!(
            graph.edges.len(),
            edges_before,
            "Edge count should not change"
        );
    }

    /// Test large-scale resolution for performance characteristics
    #[test]
    fn test_large_scale_resolution() {
        let mut graph = CallGraph::new();

        // Create a larger graph (100 files, 10 functions each)
        for file_idx in 0..100 {
            for func_idx in 0..10 {
                let func_id = FunctionId::new(
                    PathBuf::from(format!("file_{}.rs", file_idx)),
                    format!("function_{}_{}", file_idx, func_idx),
                    func_idx * 10,
                );
                graph.add_function(func_id, false, false, 5, 50);
            }
        }

        // Add many unresolved calls
        for i in 0..500 {
            let caller = FunctionId::new(
                PathBuf::from(format!("file_{}.rs", i % 100)),
                format!("function_{}_{}", i % 100, i % 10),
                (i % 10) * 10,
            );
            let callee = FunctionId {
                file: PathBuf::from("unknown.rs"),
                name: format!("function_{}_{}", (i + 1) % 100, (i + 1) % 10),
                line: 0,
                module_path: String::new(),
            };
            graph.add_call(FunctionCall {
                caller,
                callee,
                call_type: CallType::Direct,
            });
        }

        // This should complete without panic and resolve most calls
        graph.resolve_cross_file_calls();

        // Verify some resolutions occurred
        let remaining_unresolved = graph.find_unresolved_calls();
        assert!(
            remaining_unresolved.len() < 500,
            "Expected some calls to be resolved"
        );
    }
}
