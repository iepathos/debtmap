//! Basic graph operations for adding and querying nodes and edges

use super::types::{CallGraph, CallType, FunctionCall, FunctionId, FunctionNode};
use im::{HashMap, HashSet, Vector};
use std::path::PathBuf;

impl CallGraph {
    pub fn merge(&mut self, other: CallGraph) {
        // Merge nodes
        for (id, node) in other.nodes {
            self.nodes.insert(id, node);
        }

        // Merge edges
        for call in other.edges {
            self.add_call(call);
        }
    }

    pub fn add_function(
        &mut self,
        id: FunctionId,
        is_entry_point: bool,
        is_test: bool,
        complexity: u32,
        lines: usize,
    ) {
        let node = FunctionNode {
            id: id.clone(),
            is_entry_point,
            is_test,
            complexity,
            _lines: lines,
        };
        self.nodes.insert(id, node);
    }

    pub fn add_call(&mut self, call: FunctionCall) {
        let caller = call.caller.clone();
        let callee = call.callee.clone();

        self.edges.push_back(call);

        self.callee_index
            .entry(caller.clone())
            .or_default()
            .insert(callee.clone());

        self.caller_index.entry(callee).or_default().insert(caller);
    }

    pub fn add_call_parts(&mut self, caller: FunctionId, callee: FunctionId, call_type: CallType) {
        self.add_call(FunctionCall {
            caller,
            callee,
            call_type,
        });
    }

    pub fn get_callees(&self, func_id: &FunctionId) -> Vec<FunctionId> {
        self.callee_index
            .get(func_id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn get_callers(&self, func_id: &FunctionId) -> Vec<FunctionId> {
        self.caller_index
            .get(func_id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_dependency_count(&self, func_id: &FunctionId) -> usize {
        self.get_callers(func_id).len()
    }

    /// Get all functions in the graph
    pub fn get_all_functions(&self) -> impl Iterator<Item = &FunctionId> {
        self.nodes.keys()
    }

    /// Get function info
    pub fn get_function_info(&self, func_id: &FunctionId) -> Option<(bool, bool, u32, usize)> {
        self.nodes.get(func_id).map(|node| {
            (
                node.is_entry_point,
                node.is_test,
                node.complexity,
                node._lines,
            )
        })
    }

    /// Mark a function as being reachable through trait dispatch
    /// This helps reduce false positives in dead code detection
    pub fn mark_as_trait_dispatch(&mut self, func_id: FunctionId) {
        // Ensure the function exists in the graph
        if !self.nodes.contains_key(&func_id) {
            self.nodes.insert(
                func_id.clone(),
                FunctionNode {
                    id: func_id.clone(),
                    is_entry_point: false,
                    is_test: false,
                    complexity: 0,
                    _lines: 0,
                },
            );
        }

        // Mark it as an entry point to prevent dead code false positives
        if let Some(node) = self.nodes.get_mut(&func_id) {
            node.is_entry_point = true;
        }
    }

    pub fn is_entry_point(&self, func_id: &FunctionId) -> bool {
        self.nodes
            .get(func_id)
            .map(|n| n.is_entry_point)
            .unwrap_or(false)
    }

    pub fn is_test_function(&self, func_id: &FunctionId) -> bool {
        self.nodes.get(func_id).map(|n| n.is_test).unwrap_or(false)
    }

    // String-based convenience methods for critical path analysis

    /// Add an edge between two functions by name (used by critical path analyzer)
    pub fn add_edge_by_name(&mut self, from: String, to: String, file: PathBuf) {
        // Create simplified FunctionIds for string-based access
        let from_id = FunctionId {
            file: file.clone(),
            name: from,
            line: 0, // Use 0 for string-based lookups
        };
        let to_id = FunctionId {
            file: file.clone(),
            name: to,
            line: 0,
        };

        // Ensure both nodes exist
        if !self.nodes.contains_key(&from_id) {
            self.add_function(from_id.clone(), false, false, 0, 0);
        }
        if !self.nodes.contains_key(&to_id) {
            self.add_function(to_id.clone(), false, false, 0, 0);
        }

        // Add the call
        self.add_call(FunctionCall {
            caller: from_id,
            callee: to_id,
            call_type: CallType::Direct,
        });
    }

    /// Get callees by function name (returns function names)
    pub fn get_callees_by_name(&self, function: &str) -> Vec<String> {
        // Find all nodes with this function name
        self.nodes
            .keys()
            .filter(|id| id.name == function)
            .flat_map(|id| self.get_callees(id))
            .map(|id| id.name.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Get callers by function name (returns function names)
    pub fn get_callers_by_name(&self, function: &str) -> Vec<String> {
        // Find all nodes with this function name
        self.nodes
            .keys()
            .filter(|id| id.name == function)
            .flat_map(|id| self.get_callers(id))
            .map(|id| id.name.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn get_transitive_callees(
        &self,
        func_id: &FunctionId,
        max_depth: usize,
    ) -> HashSet<FunctionId> {
        let mut visited = HashSet::new();
        let mut to_visit = Vector::new();
        to_visit.push_back((func_id.clone(), 0));

        while let Some((current, depth)) = to_visit.pop_front() {
            if depth >= max_depth || visited.contains(&current) {
                continue;
            }

            visited.insert(current.clone());

            for callee in self.get_callees(&current) {
                if !visited.contains(&callee) {
                    to_visit.push_back((callee, depth + 1));
                }
            }
        }

        visited.remove(func_id);
        visited
    }

    pub fn get_transitive_callers(
        &self,
        func_id: &FunctionId,
        max_depth: usize,
    ) -> HashSet<FunctionId> {
        let mut visited = HashSet::new();
        let mut to_visit = Vector::new();
        to_visit.push_back((func_id.clone(), 0));

        while let Some((current, depth)) = to_visit.pop_front() {
            if visited.contains(&current) {
                continue;
            }

            visited.insert(current.clone());

            if depth < max_depth {
                for caller in self.get_callers(&current) {
                    if !visited.contains(&caller) {
                        to_visit.push_back((caller, depth + 1));
                    }
                }
            }
        }

        visited.remove(func_id);
        visited
    }

    pub fn find_entry_points(&self) -> Vec<FunctionId> {
        self.nodes
            .values()
            .filter(|node| node.is_entry_point)
            .map(|node| node.id.clone())
            .collect()
    }

    pub fn find_all_functions(&self) -> Vec<FunctionId> {
        self.nodes.keys().cloned().collect()
    }

    pub fn get_function_calls(&self, func_id: &FunctionId) -> Vec<FunctionCall> {
        self.edges
            .iter()
            .filter(|call| &call.caller == func_id)
            .cloned()
            .collect()
    }

    /// Get all function calls in the graph (for testing and debugging)
    pub fn get_all_calls(&self) -> Vec<FunctionCall> {
        self.edges.iter().cloned().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Find a function at a specific file and line location
    /// Returns the function that contains the given line
    pub fn find_function_at_location(&self, file: &PathBuf, line: usize) -> Option<FunctionId> {
        let functions_in_file = Self::functions_in_file(&self.nodes, file);
        Self::find_best_line_match(&functions_in_file, line)
    }

    /// Pure function to filter functions by file
    pub fn functions_in_file<'a>(
        nodes: &'a HashMap<FunctionId, FunctionNode>,
        file: &PathBuf,
    ) -> Vec<&'a FunctionId> {
        nodes.keys().filter(|id| &id.file == file).collect()
    }

    /// Pure function to find the best matching function by line proximity
    pub fn find_best_line_match(
        functions: &[&FunctionId],
        target_line: usize,
    ) -> Option<FunctionId> {
        functions
            .iter()
            .filter(|func_id| func_id.line <= target_line)
            .min_by_key(|func_id| target_line - func_id.line)
            .map(|&func_id| func_id.clone())
    }
}
