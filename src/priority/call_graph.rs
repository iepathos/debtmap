use im::{HashMap, HashSet, Vector};
use std::path::PathBuf;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FunctionId {
    pub file: PathBuf,
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub caller: FunctionId,
    pub callee: FunctionId,
    pub call_type: CallType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallType {
    Direct,
    Delegate,
    Pipeline,
    Async,
    Callback,
}

#[derive(Debug, Clone)]
pub struct CallGraph {
    nodes: HashMap<FunctionId, FunctionNode>,
    edges: Vector<FunctionCall>,
    caller_index: HashMap<FunctionId, HashSet<FunctionId>>,
    callee_index: HashMap<FunctionId, HashSet<FunctionId>>,
}

#[derive(Debug, Clone)]
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

    pub fn get_callers(&self, func_id: &FunctionId) -> Vec<FunctionId> {
        self.caller_index
            .get(func_id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_dependency_count(&self, func_id: &FunctionId) -> usize {
        self.get_callers(func_id).len()
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
            if depth >= max_depth || visited.contains(&current) {
                continue;
            }

            visited.insert(current.clone());

            for caller in self.get_callers(&current) {
                if !visited.contains(&caller) {
                    to_visit.push_back((caller, depth + 1));
                }
            }
        }

        visited.remove(func_id);
        visited
    }

    pub fn detect_delegation_pattern(&self, func_id: &FunctionId) -> bool {
        if let Some(node) = self.nodes.get(func_id) {
            let callees = self.get_callees(func_id);

            // Simple delegation: low complexity, mostly calls other functions
            if node.complexity <= 2 && !callees.is_empty() {
                let avg_callee_complexity: f64 = callees
                    .iter()
                    .filter_map(|id| self.nodes.get(id))
                    .map(|n| n.complexity as f64)
                    .sum::<f64>()
                    / callees.len() as f64;

                // Delegates if callees are more complex
                return avg_callee_complexity > node.complexity as f64 * 2.0;
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
}
