//! Cross-file call resolution for handling method calls across modules

use super::types::{CallGraph, FunctionCall, FunctionId};
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

    /// Advanced call resolution using sophisticated matching strategies
    /// This pure function handles complex cases like:
    /// - Associated function calls (Type::method matching function stored as Type::method)
    /// - Qualified path resolution
    /// - Cross-module calls with type hints
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

    /// Resolve cross-file function calls by matching function names across all files
    /// This is needed because method calls like `obj.method()` don't know the target file
    /// at parse time and default to the current file
    pub fn resolve_cross_file_calls(&mut self) {
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
