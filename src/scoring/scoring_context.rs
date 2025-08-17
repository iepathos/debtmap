use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::risk::lcov::LcovData;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ScoringContext {
    pub call_graph: CallGraph,
    pub coverage_map: Option<LcovData>,
    pub git_history: Option<GitHistory>,
    pub hot_paths: HashSet<FunctionId>,
    pub test_files: HashSet<PathBuf>,
    pub entry_points: Vec<FunctionId>,
    pub call_frequencies: HashMap<FunctionId, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHistory {
    pub change_counts: HashMap<PathBuf, usize>,
    pub bug_fix_counts: HashMap<PathBuf, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub total: f64,
    pub components: HashMap<String, f64>,
    pub explanation: String,
    pub confidence: f64,
}

impl ScoreBreakdown {
    pub fn new(total: f64) -> Self {
        Self {
            total,
            components: HashMap::new(),
            explanation: String::new(),
            confidence: 1.0,
        }
    }

    pub fn add_component(&mut self, name: &str, value: f64) {
        self.components.insert(name.to_string(), value);
    }

    pub fn with_explanation(mut self, explanation: String) -> Self {
        self.explanation = explanation;
        self
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }
}

impl ScoringContext {
    pub fn new(call_graph: CallGraph) -> Self {
        let entry_points = Self::identify_entry_points(&call_graph);
        let hot_paths = Self::identify_hot_paths(&call_graph, &entry_points);
        let call_frequencies = Self::calculate_call_frequencies(&call_graph);
        
        Self {
            call_graph,
            coverage_map: None,
            git_history: None,
            hot_paths,
            test_files: HashSet::new(),
            entry_points,
            call_frequencies,
        }
    }

    pub fn with_coverage(mut self, coverage: LcovData) -> Self {
        self.coverage_map = Some(coverage);
        self
    }

    pub fn with_git_history(mut self, history: GitHistory) -> Self {
        self.git_history = Some(history);
        self
    }

    pub fn with_test_files(mut self, test_files: HashSet<PathBuf>) -> Self {
        self.test_files = test_files;
        self
    }

    fn identify_entry_points(call_graph: &CallGraph) -> Vec<FunctionId> {
        let mut entry_points = Vec::new();
        
        for function in call_graph.find_all_functions() {
            // Main functions
            if function.name == "main" || function.name.ends_with("::main") {
                entry_points.push(function.clone());
                continue;
            }
            
            // API handlers
            if function.name.contains("handler") 
                || function.name.contains("endpoint")
                || function.name.contains("route") {
                entry_points.push(function.clone());
                continue;
            }
            
            // CLI commands
            if function.name.contains("cmd_") 
                || function.name.contains("command") 
                || function.name == "run" {
                entry_points.push(function.clone());
                continue;
            }
            
            // Public API functions with no callers
            if call_graph.get_callers(&function).is_empty() {
                // Check if it's likely a public API
                if function.name.starts_with("new") 
                    || function.name.starts_with("create")
                    || function.name.starts_with("from") {
                    entry_points.push(function.clone());
                }
            }
        }
        
        entry_points
    }

    fn identify_hot_paths(call_graph: &CallGraph, entry_points: &[FunctionId]) -> HashSet<FunctionId> {
        let mut hot_paths = HashSet::new();
        
        // Functions directly called by entry points are hot
        for entry in entry_points {
            hot_paths.insert(entry.clone());
            
            // Add direct callees of entry points
            for callee in call_graph.get_callees(entry) {
                hot_paths.insert(callee.clone());
            }
        }
        
        // Functions called by many others are hot
        for function in call_graph.find_all_functions() {
            let caller_count = call_graph.get_callers(&function).len();
            if caller_count >= 5 {  // Threshold for "many callers"
                hot_paths.insert(function.clone());
            }
        }
        
        hot_paths
    }

    fn calculate_call_frequencies(call_graph: &CallGraph) -> HashMap<FunctionId, usize> {
        let mut frequencies = HashMap::new();
        
        // Count how many times each function is called
        for function in call_graph.find_all_functions() {
            let caller_count = call_graph.get_callers(&function).len();
            frequencies.insert(function.clone(), caller_count);
        }
        
        frequencies
    }

    pub fn distance_from_entry(&self, function: &FunctionId) -> Option<usize> {
        use std::collections::VecDeque;
        
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        // Start from all entry points
        for entry in &self.entry_points {
            queue.push_back((entry.clone(), 0));
            visited.insert(entry.clone());
        }
        
        while let Some((current, distance)) = queue.pop_front() {
            if &current == function {
                return Some(distance);
            }
            
            for callee in self.call_graph.get_callees(&current) {
                if !visited.contains(&callee) {
                    visited.insert(callee.clone());
                    queue.push_back((callee, distance + 1));
                }
            }
        }
        
        None
    }
}