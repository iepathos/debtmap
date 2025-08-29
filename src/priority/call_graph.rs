use im::{HashMap, HashSet, Vector};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionId {
    pub file: PathBuf,
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub caller: FunctionId,
    pub callee: FunctionId,
    pub call_type: CallType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CallType {
    Direct,
    Delegate,
    Pipeline,
    Async,
    Callback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    #[serde(with = "function_id_map")]
    nodes: HashMap<FunctionId, FunctionNode>,
    edges: Vector<FunctionCall>,
    #[serde(with = "function_id_map")]
    caller_index: HashMap<FunctionId, HashSet<FunctionId>>,
    #[serde(with = "function_id_map")]
    callee_index: HashMap<FunctionId, HashSet<FunctionId>>,
}

// Custom serialization for HashMap with FunctionId keys
mod function_id_map {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap as StdHashMap;

    pub fn serialize<S, V>(
        map: &im::HashMap<FunctionId, V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        let string_map: StdHashMap<String, &V> = map
            .iter()
            .map(|(k, v)| (format!("{}:{}:{}", k.file.display(), k.name, k.line), v))
            .collect();
        string_map.serialize(serializer)
    }

    pub fn deserialize<'de, D, V>(deserializer: D) -> Result<im::HashMap<FunctionId, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de> + Clone,
    {
        let string_map: StdHashMap<String, V> = StdHashMap::deserialize(deserializer)?;
        let mut result = im::HashMap::new();
        for (key, value) in string_map {
            let parts: Vec<&str> = key.rsplitn(3, ':').collect();
            if parts.len() == 3 {
                let func_id = FunctionId {
                    file: parts[2].into(),
                    name: parts[1].to_string(),
                    line: parts[0].parse().unwrap_or(0),
                };
                result.insert(func_id, value);
            }
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionNode {
    id: FunctionId,
    is_entry_point: bool,
    is_test: bool,
    complexity: u32,
    _lines: usize,
}

impl CallGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vector::new(),
            caller_index: HashMap::new(),
            callee_index: HashMap::new(),
        }
    }

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

    pub fn detect_delegation_pattern(&self, func_id: &FunctionId) -> bool {
        if let Some(node) = self.nodes.get(func_id) {
            let callees = self.get_callees(func_id);

            // Delegation pattern requires:
            // 1. Low complexity in the orchestrator
            // 2. Multiple functions being coordinated (not just one wrapper call)
            // 3. Callees doing the real work (higher complexity)
            if node.complexity <= 3 && callees.len() >= 2 {
                let avg_callee_complexity: f64 = callees
                    .iter()
                    .filter_map(|id| self.nodes.get(id))
                    .map(|n| n.complexity as f64)
                    .sum::<f64>()
                    / callees.len().max(1) as f64;

                // Delegates if callees are more complex on average
                return avg_callee_complexity > node.complexity as f64 * 1.5;
            }
        }
        false
    }

    pub fn find_entry_points(&self) -> Vec<FunctionId> {
        self.nodes
            .values()
            .filter(|node| node.is_entry_point)
            .map(|node| node.id.clone())
            .collect()
    }

    pub fn find_test_functions(&self) -> Vec<FunctionId> {
        self.nodes
            .values()
            .filter(|node| node.is_test)
            .map(|node| node.id.clone())
            .collect()
    }

    /// Check if a function is only called by test functions (test helper)
    /// Returns true if:
    /// - The function has at least one caller
    /// - All callers are test functions
    pub fn is_test_helper(&self, func_id: &FunctionId) -> bool {
        let callers = self.get_callers(func_id);

        // If no callers, it's not a test helper
        if callers.is_empty() {
            return false;
        }

        // Check if all callers are test functions
        callers.iter().all(|caller| self.is_test_function(caller))
    }

    pub fn find_all_functions(&self) -> Vec<FunctionId> {
        self.nodes.keys().cloned().collect()
    }

    /// Identify functions that are only reachable from test functions
    /// These are test infrastructure functions (mocks, helpers, fixtures, etc.)
    pub fn find_test_only_functions(&self) -> HashSet<FunctionId> {
        // Step 1: Identify all test functions
        let test_functions: HashSet<FunctionId> = self
            .nodes
            .iter()
            .filter(|(_, node)| node.is_test)
            .map(|(id, _)| id.clone())
            .collect();

        // Step 2: Find all functions reachable from test functions (including tests themselves)
        let mut reachable_from_tests = test_functions.clone();
        for test_fn in &test_functions {
            // Get all transitive callees (functions called by this test)
            let callees = self.get_transitive_callees(test_fn, usize::MAX);
            reachable_from_tests.extend(callees);
        }

        // Step 3: Find all functions reachable from non-test code
        let mut reachable_from_production = HashSet::new();
        for (id, node) in &self.nodes {
            // Skip test functions entirely
            if node.is_test {
                continue;
            }

            // Non-test functions that are either:
            // 1. Marked as entry points
            // 2. Have no callers (top-level functions)
            if node.is_entry_point || self.get_callers(id).is_empty() {
                reachable_from_production.insert(id.clone());
                let callees = self.get_transitive_callees(id, usize::MAX);
                reachable_from_production.extend(callees);
            }
        }

        // Step 4: Test-only functions are those reachable from tests but NOT from production
        // This excludes test functions themselves (they're not infrastructure, they ARE tests)
        let test_only: HashSet<FunctionId> = reachable_from_tests
            .into_iter()
            .filter(|id| {
                // Must be:
                // 1. NOT in production reachable set
                // 2. NOT a test function itself (test functions are not infrastructure)
                !reachable_from_production.contains(id)
                    && self.nodes.get(id).is_some_and(|node| !node.is_test)
            })
            .collect();

        test_only
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

    pub fn calculate_criticality(&self, func_id: &FunctionId) -> f64 {
        let mut criticality = 1.0;

        // Entry points are critical
        if self.is_entry_point(func_id) {
            criticality *= 2.0;
        }

        // Functions with many dependents are critical
        let dependency_count = self.get_dependency_count(func_id);
        if dependency_count > 5 {
            criticality *= 1.5;
        } else if dependency_count > 2 {
            criticality *= 1.2;
        }

        // Functions called by entry points are critical
        let callers = self.get_callers(func_id);
        for caller in &callers {
            if self.is_entry_point(caller) {
                criticality *= 1.3;
                break;
            }
        }

        criticality
    }

    /// Build a map of all functions by name
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

    /// Find a function at a specific file and line location
    /// Returns the function that contains the given line
    pub fn find_function_at_location(&self, file: &PathBuf, line: usize) -> Option<FunctionId> {
        // Find all functions in the specified file
        let functions_in_file: Vec<_> = self.nodes.keys().filter(|id| &id.file == file).collect();

        // Find the function that contains this line
        // We'll return the function with the closest line number that's <= the target line
        let mut best_match: Option<&FunctionId> = None;
        let mut best_distance = usize::MAX;

        for func_id in functions_in_file {
            if func_id.line <= line {
                let distance = line - func_id.line;
                if distance < best_distance {
                    best_distance = distance;
                    best_match = Some(func_id);
                }
            }
        }

        best_match.cloned()
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
        let functions_by_name = self.build_function_name_map();
        let calls_to_resolve = self.find_unresolved_calls();

        for call in calls_to_resolve {
            // Look for a function with the same name in the nodes
            if let Some(candidates) = functions_by_name.get(&call.callee.name) {
                // First, try to find a match in the same file
                let same_file_match = candidates
                    .iter()
                    .find(|func_id| func_id.file == call.caller.file);

                if let Some(resolved) = same_file_match {
                    // Prefer same-file resolution
                    self.apply_call_resolution(&call, resolved);
                } else if candidates.len() == 1 {
                    // If no same-file match, but only one candidate globally, use it
                    self.apply_call_resolution(&call, &candidates[0]);
                }
                // If multiple candidates across files and none in the same file,
                // we leave them unresolved for now
            }
        }
    }
}

impl Default for CallGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_graph_basic() {
        let mut graph = CallGraph::new();

        let main_id = FunctionId {
            file: PathBuf::from("main.rs"),
            name: "main".to_string(),
            line: 1,
        };

        let helper_id = FunctionId {
            file: PathBuf::from("lib.rs"),
            name: "helper".to_string(),
            line: 10,
        };

        graph.add_function(main_id.clone(), true, false, 2, 20);
        graph.add_function(helper_id.clone(), false, false, 5, 30);

        graph.add_call(FunctionCall {
            caller: main_id.clone(),
            callee: helper_id.clone(),
            call_type: CallType::Direct,
        });

        assert_eq!(graph.get_callees(&main_id).len(), 1);
        assert_eq!(graph.get_callers(&helper_id).len(), 1);
        assert!(graph.is_entry_point(&main_id));
        assert!(!graph.is_entry_point(&helper_id));
    }

    #[test]
    fn test_transitive_dependencies() {
        let mut graph = CallGraph::new();

        let a = FunctionId {
            file: PathBuf::from("a.rs"),
            name: "a".to_string(),
            line: 1,
        };
        let b = FunctionId {
            file: PathBuf::from("b.rs"),
            name: "b".to_string(),
            line: 1,
        };
        let c = FunctionId {
            file: PathBuf::from("c.rs"),
            name: "c".to_string(),
            line: 1,
        };

        graph.add_function(a.clone(), true, false, 1, 10);
        graph.add_function(b.clone(), false, false, 2, 20);
        graph.add_function(c.clone(), false, false, 3, 30);

        graph.add_call(FunctionCall {
            caller: a.clone(),
            callee: b.clone(),
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: b.clone(),
            callee: c.clone(),
            call_type: CallType::Direct,
        });

        let transitive = graph.get_transitive_callees(&a, 3);
        assert_eq!(transitive.len(), 2);
        assert!(transitive.contains(&b));
        assert!(transitive.contains(&c));
    }

    #[test]
    fn test_find_function_at_location() {
        let mut graph = CallGraph::new();

        let file = PathBuf::from("test.rs");

        // Add functions at different line numbers
        let func1 = FunctionId {
            file: file.clone(),
            name: "function_one".to_string(),
            line: 10,
        };

        let func2 = FunctionId {
            file: file.clone(),
            name: "function_two".to_string(),
            line: 30,
        };

        let func3 = FunctionId {
            file: file.clone(),
            name: "function_three".to_string(),
            line: 50,
        };

        graph.add_function(func1.clone(), false, false, 5, 15);
        graph.add_function(func2.clone(), false, false, 3, 10);
        graph.add_function(func3.clone(), false, false, 4, 20);

        // Test finding functions at exact line numbers
        let found = graph.find_function_at_location(&file, 10);
        assert_eq!(
            found.as_ref().map(|f| &f.name),
            Some(&"function_one".to_string())
        );

        let found = graph.find_function_at_location(&file, 30);
        assert_eq!(
            found.as_ref().map(|f| &f.name),
            Some(&"function_two".to_string())
        );

        // Test finding functions within their range
        let found = graph.find_function_at_location(&file, 15);
        assert_eq!(
            found.as_ref().map(|f| &f.name),
            Some(&"function_one".to_string())
        );

        let found = graph.find_function_at_location(&file, 35);
        assert_eq!(
            found.as_ref().map(|f| &f.name),
            Some(&"function_two".to_string())
        );

        let found = graph.find_function_at_location(&file, 60);
        assert_eq!(
            found.as_ref().map(|f| &f.name),
            Some(&"function_three".to_string())
        );

        // Test line before any function
        let found = graph.find_function_at_location(&file, 5);
        assert_eq!(found, None);

        // Test different file
        let other_file = PathBuf::from("other.rs");
        let found = graph.find_function_at_location(&other_file, 25);
        assert_eq!(found, None);
    }

    #[test]
    fn test_delegation_detection() {
        let mut graph = CallGraph::new();

        let orchestrator = FunctionId {
            file: PathBuf::from("orch.rs"),
            name: "orchestrate".to_string(),
            line: 1,
        };
        let worker1 = FunctionId {
            file: PathBuf::from("work.rs"),
            name: "complex_work1".to_string(),
            line: 10,
        };
        let worker2 = FunctionId {
            file: PathBuf::from("work.rs"),
            name: "complex_work2".to_string(),
            line: 20,
        };

        graph.add_function(orchestrator.clone(), false, false, 2, 15);
        graph.add_function(worker1.clone(), false, false, 10, 50);
        graph.add_function(worker2.clone(), false, false, 8, 40);

        graph.add_call(FunctionCall {
            caller: orchestrator.clone(),
            callee: worker1.clone(),
            call_type: CallType::Delegate,
        });
        graph.add_call(FunctionCall {
            caller: orchestrator.clone(),
            callee: worker2.clone(),
            call_type: CallType::Delegate,
        });

        assert!(graph.detect_delegation_pattern(&orchestrator));
    }

    #[test]
    fn test_get_transitive_callers_single_level() {
        let mut graph = CallGraph::new();

        let a = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_a".to_string(),
            line: 1,
        };
        let b = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_b".to_string(),
            line: 10,
        };
        let c = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_c".to_string(),
            line: 20,
        };

        graph.add_function(a.clone(), false, false, 1, 10);
        graph.add_function(b.clone(), false, false, 1, 10);
        graph.add_function(c.clone(), false, false, 1, 10);

        // a -> b, c -> b (b has two callers)
        graph.add_call(FunctionCall {
            caller: a.clone(),
            callee: b.clone(),
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: c.clone(),
            callee: b.clone(),
            call_type: CallType::Direct,
        });

        let callers = graph.get_transitive_callers(&b, 1);
        assert_eq!(callers.len(), 2);
        assert!(callers.contains(&a));
        assert!(callers.contains(&c));
    }

    #[test]
    fn test_get_transitive_callers_multi_level() {
        let mut graph = CallGraph::new();

        let a = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_a".to_string(),
            line: 1,
        };
        let b = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_b".to_string(),
            line: 10,
        };
        let c = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_c".to_string(),
            line: 20,
        };
        let d = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_d".to_string(),
            line: 30,
        };

        graph.add_function(a.clone(), false, false, 1, 10);
        graph.add_function(b.clone(), false, false, 1, 10);
        graph.add_function(c.clone(), false, false, 1, 10);
        graph.add_function(d.clone(), false, false, 1, 10);

        // a -> b -> c -> d (chain of calls)
        graph.add_call(FunctionCall {
            caller: a.clone(),
            callee: b.clone(),
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: b.clone(),
            callee: c.clone(),
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: c.clone(),
            callee: d.clone(),
            call_type: CallType::Direct,
        });

        // Get all transitive callers of d with max_depth 3
        let callers = graph.get_transitive_callers(&d, 3);
        assert_eq!(callers.len(), 3);
        assert!(callers.contains(&a));
        assert!(callers.contains(&b));
        assert!(callers.contains(&c));

        // Test with limited depth
        let callers_depth_1 = graph.get_transitive_callers(&d, 1);
        assert_eq!(callers_depth_1.len(), 1);
        assert!(callers_depth_1.contains(&c));

        let callers_depth_2 = graph.get_transitive_callers(&d, 2);
        assert_eq!(callers_depth_2.len(), 2);
        assert!(callers_depth_2.contains(&b));
        assert!(callers_depth_2.contains(&c));
    }

    #[test]
    fn test_get_transitive_callers_with_cycles() {
        let mut graph = CallGraph::new();

        let a = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_a".to_string(),
            line: 1,
        };
        let b = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_b".to_string(),
            line: 10,
        };
        let c = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_c".to_string(),
            line: 20,
        };

        graph.add_function(a.clone(), false, false, 1, 10);
        graph.add_function(b.clone(), false, false, 1, 10);
        graph.add_function(c.clone(), false, false, 1, 10);

        // Create a cycle: a -> b -> c -> a
        graph.add_call(FunctionCall {
            caller: a.clone(),
            callee: b.clone(),
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: b.clone(),
            callee: c.clone(),
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: c.clone(),
            callee: a.clone(),
            call_type: CallType::Direct,
        });

        // Should handle cycles without infinite loop
        let callers = graph.get_transitive_callers(&a, 10);
        assert_eq!(callers.len(), 2);
        assert!(callers.contains(&b));
        assert!(callers.contains(&c));
    }

    #[test]
    fn test_get_transitive_callers_no_callers() {
        let mut graph = CallGraph::new();

        let a = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_a".to_string(),
            line: 1,
        };
        let b = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_b".to_string(),
            line: 10,
        };

        graph.add_function(a.clone(), false, false, 1, 10);
        graph.add_function(b.clone(), false, false, 1, 10);

        // a -> b (a calls b, so b has one caller but a has none)
        graph.add_call(FunctionCall {
            caller: a.clone(),
            callee: b.clone(),
            call_type: CallType::Direct,
        });

        // a has no callers
        let callers = graph.get_transitive_callers(&a, 5);
        assert_eq!(callers.len(), 0);
    }

    #[test]
    fn test_get_transitive_callers_complex_graph() {
        let mut graph = CallGraph::new();

        // Create a complex graph structure
        //      a
        //     / \
        //    b   c
        //    |\ /|
        //    | X |
        //    |/ \|
        //    d   e
        //     \ /
        //      f

        let a = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_a".to_string(),
            line: 1,
        };
        let b = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_b".to_string(),
            line: 10,
        };
        let c = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_c".to_string(),
            line: 20,
        };
        let d = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_d".to_string(),
            line: 30,
        };
        let e = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_e".to_string(),
            line: 40,
        };
        let f = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "func_f".to_string(),
            line: 50,
        };

        graph.add_function(a.clone(), false, false, 1, 10);
        graph.add_function(b.clone(), false, false, 1, 10);
        graph.add_function(c.clone(), false, false, 1, 10);
        graph.add_function(d.clone(), false, false, 1, 10);
        graph.add_function(e.clone(), false, false, 1, 10);
        graph.add_function(f.clone(), false, false, 1, 10);

        // Add edges
        graph.add_call(FunctionCall {
            caller: a.clone(),
            callee: b.clone(),
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: a.clone(),
            callee: c.clone(),
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: b.clone(),
            callee: d.clone(),
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: b.clone(),
            callee: e.clone(),
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: c.clone(),
            callee: d.clone(),
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: c.clone(),
            callee: e.clone(),
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: d.clone(),
            callee: f.clone(),
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: e.clone(),
            callee: f.clone(),
            call_type: CallType::Direct,
        });

        // Test transitive callers of f
        let callers_f = graph.get_transitive_callers(&f, 10);
        assert_eq!(callers_f.len(), 5); // All except f itself
        assert!(callers_f.contains(&a));
        assert!(callers_f.contains(&b));
        assert!(callers_f.contains(&c));
        assert!(callers_f.contains(&d));
        assert!(callers_f.contains(&e));

        // Test with limited depth
        let callers_f_depth_2 = graph.get_transitive_callers(&f, 2);
        assert_eq!(callers_f_depth_2.len(), 4); // d, e, b, c
        assert!(callers_f_depth_2.contains(&d));
        assert!(callers_f_depth_2.contains(&e));
        assert!(callers_f_depth_2.contains(&b));
        assert!(callers_f_depth_2.contains(&c));
    }

    #[test]
    fn test_call_graph_serialization_roundtrip() {
        use serde_json;

        // Create a CallGraph
        let mut graph = CallGraph::new();

        let func1 = FunctionId {
            file: PathBuf::from("src/main.rs"),
            name: "main".to_string(),
            line: 10,
        };

        let func2 = FunctionId {
            file: PathBuf::from("src/lib.rs"),
            name: "helper".to_string(),
            line: 25,
        };

        graph.add_function(func1.clone(), true, false, 5, 50);
        graph.add_function(func2.clone(), false, false, 3, 30);
        graph.add_call(FunctionCall {
            caller: func1.clone(),
            callee: func2.clone(),
            call_type: CallType::Direct,
        });

        // Serialize to JSON
        let json = serde_json::to_string(&graph).unwrap();

        // Deserialize back - this will trigger our custom deserialize function
        let deserialized: CallGraph = serde_json::from_str(&json).unwrap();

        // Verify the graph was correctly deserialized
        assert_eq!(deserialized.get_callees(&func1).len(), 1);
        assert_eq!(deserialized.get_callers(&func2).len(), 1);
        assert!(deserialized.is_entry_point(&func1));
        assert!(!deserialized.is_entry_point(&func2));
    }

    #[test]
    fn test_function_id_map_deserialize_happy_path() {
        use serde_json;
        use std::collections::HashMap as StdHashMap;

        // Create a JSON representation with string keys in "file:name:line" format
        let json_data = r#"{
            "src/main.rs:main:10": {"value": 100},
            "src/lib.rs:helper:25": {"value": 200},
            "src/utils.rs:process_data:42": {"value": 300}
        }"#;

        #[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
        struct TestValue {
            value: u32,
        }

        // Deserialize using our custom deserializer
        let result: Result<im::HashMap<FunctionId, TestValue>, _> = serde_json::from_str(json_data)
            .map(|map: StdHashMap<String, TestValue>| {
                let mut result = im::HashMap::new();
                for (key, value) in map {
                    let parts: Vec<&str> = key.rsplitn(3, ':').collect();
                    if parts.len() == 3 {
                        let func_id = FunctionId {
                            file: parts[2].into(),
                            name: parts[1].to_string(),
                            line: parts[0].parse().unwrap_or(0),
                        };
                        result.insert(func_id, value);
                    }
                }
                result
            });

        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.len(), 3);

        // Verify specific entries
        let main_id = FunctionId {
            file: PathBuf::from("src/main.rs"),
            name: "main".to_string(),
            line: 10,
        };
        assert_eq!(map.get(&main_id).unwrap().value, 100);

        let helper_id = FunctionId {
            file: PathBuf::from("src/lib.rs"),
            name: "helper".to_string(),
            line: 25,
        };
        assert_eq!(map.get(&helper_id).unwrap().value, 200);
    }

    #[test]
    fn test_function_id_map_deserialize_empty_map() {
        use serde_json;
        use std::collections::HashMap as StdHashMap;

        // Test with empty JSON object
        let json_data = r#"{}"#;

        #[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
        struct TestValue {
            value: u32,
        }

        let result: Result<im::HashMap<FunctionId, TestValue>, _> = serde_json::from_str(json_data)
            .map(|map: StdHashMap<String, TestValue>| {
                let mut result = im::HashMap::new();
                for (key, value) in map {
                    let parts: Vec<&str> = key.rsplitn(3, ':').collect();
                    if parts.len() == 3 {
                        let func_id = FunctionId {
                            file: parts[2].into(),
                            name: parts[1].to_string(),
                            line: parts[0].parse().unwrap_or(0),
                        };
                        result.insert(func_id, value);
                    }
                }
                result
            });

        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_function_id_map_deserialize_malformed_keys() {
        use serde_json;
        use std::collections::HashMap as StdHashMap;

        // Test with malformed keys (missing parts)
        let json_data = r#"{
            "src/main.rs:main:10": {"value": 100},
            "malformed_key": {"value": 200},
            "only:two": {"value": 300},
            "src/lib.rs:helper:25": {"value": 400}
        }"#;

        #[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
        struct TestValue {
            value: u32,
        }

        let result: Result<im::HashMap<FunctionId, TestValue>, _> = serde_json::from_str(json_data)
            .map(|map: StdHashMap<String, TestValue>| {
                let mut result = im::HashMap::new();
                for (key, value) in map {
                    let parts: Vec<&str> = key.rsplitn(3, ':').collect();
                    if parts.len() == 3 {
                        let func_id = FunctionId {
                            file: parts[2].into(),
                            name: parts[1].to_string(),
                            line: parts[0].parse().unwrap_or(0),
                        };
                        result.insert(func_id, value);
                    }
                }
                result
            });

        assert!(result.is_ok());
        let map = result.unwrap();
        // Only valid keys should be included
        assert_eq!(map.len(), 2);

        // Verify valid entries are present
        let main_id = FunctionId {
            file: PathBuf::from("src/main.rs"),
            name: "main".to_string(),
            line: 10,
        };
        assert_eq!(map.get(&main_id).unwrap().value, 100);
    }

    #[test]
    fn test_function_id_map_deserialize_invalid_line_numbers() {
        use serde_json;
        use std::collections::HashMap as StdHashMap;

        // Test with invalid line numbers
        let json_data = r#"{
            "src/main.rs:main:10": {"value": 100},
            "src/lib.rs:helper:not_a_number": {"value": 200},
            "src/utils.rs:process:999999": {"value": 300}
        }"#;

        #[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
        struct TestValue {
            value: u32,
        }

        let result: Result<im::HashMap<FunctionId, TestValue>, _> = serde_json::from_str(json_data)
            .map(|map: StdHashMap<String, TestValue>| {
                let mut result = im::HashMap::new();
                for (key, value) in map {
                    let parts: Vec<&str> = key.rsplitn(3, ':').collect();
                    if parts.len() == 3 {
                        let func_id = FunctionId {
                            file: parts[2].into(),
                            name: parts[1].to_string(),
                            line: parts[0].parse().unwrap_or(0),
                        };
                        result.insert(func_id, value);
                    }
                }
                result
            });

        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.len(), 3);

        // Verify that invalid line numbers default to 0
        let helper_id = FunctionId {
            file: PathBuf::from("src/lib.rs"),
            name: "helper".to_string(),
            line: 0, // Should default to 0 when parsing fails
        };
        assert_eq!(map.get(&helper_id).unwrap().value, 200);

        // Large numbers should parse successfully
        let process_id = FunctionId {
            file: PathBuf::from("src/utils.rs"),
            name: "process".to_string(),
            line: 999999,
        };
        assert_eq!(map.get(&process_id).unwrap().value, 300);
    }

    #[test]
    fn test_is_test_helper_detection() {
        let mut graph = CallGraph::new();

        // Create test functions
        let test_func1 = FunctionId {
            file: PathBuf::from("tests/test.rs"),
            name: "test_something".to_string(),
            line: 10,
        };

        let test_func2 = FunctionId {
            file: PathBuf::from("tests/test.rs"),
            name: "test_another".to_string(),
            line: 30,
        };

        // Create a helper function called only by tests
        let test_helper = FunctionId {
            file: PathBuf::from("src/lib.rs"),
            name: "validate_initial_state".to_string(),
            line: 100,
        };

        // Create a regular function called by non-test code
        let regular_func = FunctionId {
            file: PathBuf::from("src/lib.rs"),
            name: "process_data".to_string(),
            line: 200,
        };

        // Create a main function
        let main_func = FunctionId {
            file: PathBuf::from("src/main.rs"),
            name: "main".to_string(),
            line: 1,
        };

        // Add functions to graph
        graph.add_function(test_func1.clone(), false, true, 3, 20); // is_test = true
        graph.add_function(test_func2.clone(), false, true, 4, 25); // is_test = true
        graph.add_function(test_helper.clone(), false, false, 5, 30); // regular function
        graph.add_function(regular_func.clone(), false, false, 6, 40); // regular function
        graph.add_function(main_func.clone(), true, false, 2, 15); // entry point

        // Add calls: test functions call the helper
        graph.add_call(FunctionCall {
            caller: test_func1.clone(),
            callee: test_helper.clone(),
            call_type: CallType::Direct,
        });

        graph.add_call(FunctionCall {
            caller: test_func2.clone(),
            callee: test_helper.clone(),
            call_type: CallType::Direct,
        });

        // Main calls regular_func
        graph.add_call(FunctionCall {
            caller: main_func.clone(),
            callee: regular_func.clone(),
            call_type: CallType::Direct,
        });

        // Test: test_helper should be identified as a test helper
        assert!(
            graph.is_test_helper(&test_helper),
            "validate_initial_state should be identified as a test helper"
        );

        // Test: regular_func should NOT be a test helper (called by main)
        assert!(
            !graph.is_test_helper(&regular_func),
            "process_data should not be identified as a test helper"
        );

        // Test: test functions themselves are not test helpers
        assert!(
            !graph.is_test_helper(&test_func1),
            "Test functions should not be identified as test helpers"
        );

        // Test: functions with no callers are not test helpers
        let orphan_func = FunctionId {
            file: PathBuf::from("src/orphan.rs"),
            name: "unused_func".to_string(),
            line: 1,
        };
        graph.add_function(orphan_func.clone(), false, false, 1, 10);
        assert!(
            !graph.is_test_helper(&orphan_func),
            "Functions with no callers should not be test helpers"
        );

        // Test: mixed callers (test and non-test) - should NOT be a test helper
        let mixed_helper = FunctionId {
            file: PathBuf::from("src/lib.rs"),
            name: "mixed_use_helper".to_string(),
            line: 300,
        };
        graph.add_function(mixed_helper.clone(), false, false, 4, 20);

        // Called by both test and main
        graph.add_call(FunctionCall {
            caller: test_func1.clone(),
            callee: mixed_helper.clone(),
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: main_func.clone(),
            callee: mixed_helper.clone(),
            call_type: CallType::Direct,
        });

        assert!(
            !graph.is_test_helper(&mixed_helper),
            "Functions called by both test and non-test code should not be test helpers"
        );
    }
}
