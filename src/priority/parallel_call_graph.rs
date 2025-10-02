use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use dashmap::{DashMap, DashSet};
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Statistics for parallel call graph construction
#[derive(Debug, Default)]
pub struct ParallelStats {
    pub total_nodes: AtomicUsize,
    pub total_edges: AtomicUsize,
    pub files_processed: AtomicUsize,
    pub total_files: AtomicUsize,
}

impl ParallelStats {
    pub fn new(total_files: usize) -> Self {
        Self {
            total_nodes: AtomicUsize::new(0),
            total_edges: AtomicUsize::new(0),
            files_processed: AtomicUsize::new(0),
            total_files: AtomicUsize::new(total_files),
        }
    }

    pub fn increment_files(&self) {
        self.files_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_nodes(&self, count: usize) {
        self.total_nodes.fetch_add(count, Ordering::Relaxed);
    }

    pub fn add_edges(&self, count: usize) {
        self.total_edges.fetch_add(count, Ordering::Relaxed);
    }

    pub fn progress_ratio(&self) -> f64 {
        let processed = self.files_processed.load(Ordering::Relaxed) as f64;
        let total = self.total_files.load(Ordering::Relaxed) as f64;
        if total > 0.0 {
            processed / total
        } else {
            0.0
        }
    }
}

/// Thread-safe parallel call graph with concurrent data structures
pub struct ParallelCallGraph {
    nodes: Arc<DashMap<FunctionId, NodeInfo>>,
    edges: Arc<DashSet<FunctionCall>>,
    caller_index: Arc<DashMap<FunctionId, DashSet<FunctionId>>>,
    callee_index: Arc<DashMap<FunctionId, DashSet<FunctionId>>>,
    stats: Arc<ParallelStats>,
}

#[derive(Debug, Clone)]
struct NodeInfo {
    id: FunctionId,
    is_entry_point: bool,
    is_test: bool,
    complexity: u32,
    lines: usize,
}

impl ParallelCallGraph {
    pub fn new(total_files: usize) -> Self {
        Self {
            nodes: Arc::new(DashMap::new()),
            edges: Arc::new(DashSet::new()),
            caller_index: Arc::new(DashMap::new()),
            callee_index: Arc::new(DashMap::new()),
            stats: Arc::new(ParallelStats::new(total_files)),
        }
    }

    /// Add a function node concurrently
    pub fn add_function(
        &self,
        id: FunctionId,
        is_entry_point: bool,
        is_test: bool,
        complexity: u32,
        lines: usize,
    ) {
        let node_info = NodeInfo {
            id: id.clone(),
            is_entry_point,
            is_test,
            complexity,
            lines,
        };
        self.nodes.insert(id, node_info);
        self.stats.add_nodes(1);
    }

    /// Add a function call concurrently
    pub fn add_call(&self, caller: FunctionId, callee: FunctionId, call_type: CallType) {
        let call = FunctionCall {
            caller: caller.clone(),
            callee: callee.clone(),
            call_type,
        };

        if self.edges.insert(call) {
            // Update indices
            self.caller_index
                .entry(caller.clone())
                .or_default()
                .insert(callee.clone());

            self.callee_index.entry(callee).or_default().insert(caller);

            self.stats.add_edges(1);
        }
    }

    /// Merge another call graph concurrently
    pub fn merge_concurrent(&self, other: CallGraph) {
        // Parallelize node merging
        let nodes_vec: Vec<_> = other.get_all_functions().collect();
        nodes_vec.par_iter().for_each(|func_id| {
            if let Some((is_entry, is_test, complexity, lines)) = other.get_function_info(func_id) {
                self.add_function((*func_id).clone(), is_entry, is_test, complexity, lines);
            }
        });

        // Parallelize edge merging
        let calls_vec: Vec<_> = other.get_all_calls();
        calls_vec.par_iter().for_each(|call| {
            self.add_call(
                call.caller.clone(),
                call.callee.clone(),
                call.call_type.clone(),
            );
        });
    }

    /// Convert to regular CallGraph for compatibility
    pub fn to_call_graph(&self) -> CallGraph {
        let mut call_graph = CallGraph::new();

        // Add all nodes
        for entry in self.nodes.iter() {
            let node = entry.value();
            call_graph.add_function(
                node.id.clone(),
                node.is_entry_point,
                node.is_test,
                node.complexity,
                node.lines,
            );
        }

        // Add all edges
        for call in self.edges.iter() {
            call_graph.add_call(call.clone());
        }

        call_graph
    }

    /// Get progress statistics
    pub fn stats(&self) -> &Arc<ParallelStats> {
        &self.stats
    }
}

/// Progress callback type
pub type ProgressCallback = Box<dyn Fn(usize, usize) + Send + Sync>;

/// Configuration for parallel call graph construction
#[derive(Default)]
pub struct ParallelConfig {
    /// Number of worker threads (0 = use all cores)
    pub num_threads: usize,
    /// Enable deterministic mode for reproducible results
    pub deterministic: bool,
    /// Progress callback
    pub progress_callback: Option<ProgressCallback>,
}

impl ParallelConfig {
    pub fn with_threads(mut self, num_threads: usize) -> Self {
        self.num_threads = num_threads;
        self
    }

    pub fn deterministic(mut self, enabled: bool) -> Self {
        self.deterministic = enabled;
        self
    }

    pub fn with_progress<F>(mut self, callback: F) -> Self
    where
        F: Fn(usize, usize) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_function_id(name: &str) -> FunctionId {
        FunctionId {
            file: "test.rs".into(),
            name: name.to_string(),
            line: 1,
        }
    }

    #[test]
    fn test_add_function_basic() {
        let graph = ParallelCallGraph::new(1);
        let func_id = create_test_function_id("test_func");

        graph.add_function(func_id.clone(), false, false, 5, 10);

        let call_graph = graph.to_call_graph();
        let info = call_graph.get_function_info(&func_id);
        assert!(info.is_some());
        let (is_entry, is_test, complexity, lines) = info.unwrap();
        assert!(!is_entry);
        assert!(!is_test);
        assert_eq!(complexity, 5);
        assert_eq!(lines, 10);
    }

    #[test]
    fn test_add_function_duplicate() {
        let graph = ParallelCallGraph::new(1);
        let func_id = create_test_function_id("test_func");

        // Add same function twice - should only have one entry
        graph.add_function(func_id.clone(), false, false, 5, 10);
        graph.add_function(func_id.clone(), true, true, 10, 20);

        let stats = graph.stats();
        assert_eq!(stats.total_nodes.load(Ordering::Relaxed), 2); // Both inserts increment counter
    }

    #[test]
    fn test_add_function_entry_point() {
        let graph = ParallelCallGraph::new(1);
        let func_id = create_test_function_id("main");

        graph.add_function(func_id.clone(), true, false, 3, 5);

        let call_graph = graph.to_call_graph();
        let info = call_graph.get_function_info(&func_id);
        assert!(info.is_some());
        let (is_entry, _, _, _) = info.unwrap();
        assert!(is_entry);
    }

    #[test]
    fn test_add_function_test_function() {
        let graph = ParallelCallGraph::new(1);
        let func_id = create_test_function_id("test_something");

        graph.add_function(func_id.clone(), false, true, 2, 8);

        let call_graph = graph.to_call_graph();
        let info = call_graph.get_function_info(&func_id);
        assert!(info.is_some());
        let (_, is_test, _, _) = info.unwrap();
        assert!(is_test);
    }

    #[test]
    fn test_add_call_basic() {
        let graph = ParallelCallGraph::new(1);
        let caller = create_test_function_id("caller");
        let callee = create_test_function_id("callee");

        graph.add_function(caller.clone(), false, false, 5, 10);
        graph.add_function(callee.clone(), false, false, 3, 6);
        graph.add_call(caller.clone(), callee.clone(), CallType::Direct);

        let call_graph = graph.to_call_graph();
        let callees = call_graph.get_callees(&caller);
        assert_eq!(callees.len(), 1);
        assert!(callees.contains(&callee));
    }

    #[test]
    fn test_add_call_duplicate() {
        let graph = ParallelCallGraph::new(1);
        let caller = create_test_function_id("caller");
        let callee = create_test_function_id("callee");

        // Add same call twice
        graph.add_call(caller.clone(), callee.clone(), CallType::Direct);
        graph.add_call(caller.clone(), callee.clone(), CallType::Direct);

        let stats = graph.stats();
        // Should only increment once due to DashSet deduplication
        assert_eq!(stats.total_edges.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_add_call_multiple_types() {
        let graph = ParallelCallGraph::new(1);
        let caller = create_test_function_id("caller");
        let callee1 = create_test_function_id("callee1");
        let callee2 = create_test_function_id("callee2");

        graph.add_call(caller.clone(), callee1.clone(), CallType::Direct);
        graph.add_call(caller.clone(), callee2.clone(), CallType::Delegate);

        let call_graph = graph.to_call_graph();
        let callees = call_graph.get_callees(&caller);
        assert_eq!(callees.len(), 2);
        assert!(callees.contains(&callee1));
        assert!(callees.contains(&callee2));
    }

    #[test]
    fn test_concurrent_add_function() {
        use std::sync::Arc;
        use std::thread;

        let graph = Arc::new(ParallelCallGraph::new(10));
        let mut handles = vec![];

        // Spawn multiple threads adding different functions
        for i in 0..10 {
            let graph_clone = Arc::clone(&graph);
            let handle = thread::spawn(move || {
                let func_id = create_test_function_id(&format!("func_{}", i));
                graph_clone.add_function(func_id, false, false, i as u32, i * 2);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let stats = graph.stats();
        assert_eq!(stats.total_nodes.load(Ordering::Relaxed), 10);
    }

    #[test]
    fn test_concurrent_add_call() {
        use std::sync::Arc;
        use std::thread;

        let graph = Arc::new(ParallelCallGraph::new(10));
        let caller = create_test_function_id("caller");
        graph.add_function(caller.clone(), false, false, 5, 10);

        let mut handles = vec![];

        // Spawn multiple threads adding different calls
        for i in 0..10 {
            let graph_clone = Arc::clone(&graph);
            let caller_clone = caller.clone();
            let handle = thread::spawn(move || {
                let callee = create_test_function_id(&format!("callee_{}", i));
                graph_clone.add_call(caller_clone, callee, CallType::Direct);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let stats = graph.stats();
        assert_eq!(stats.total_edges.load(Ordering::Relaxed), 10);
    }

    #[test]
    fn test_stats_progress_ratio() {
        let stats = ParallelStats::new(100);
        assert_eq!(stats.progress_ratio(), 0.0);

        stats.increment_files();
        stats.increment_files();
        assert_eq!(stats.progress_ratio(), 0.02);

        for _ in 0..48 {
            stats.increment_files();
        }
        assert_eq!(stats.progress_ratio(), 0.5);
    }

    #[test]
    fn test_stats_zero_files() {
        let stats = ParallelStats::new(0);
        assert_eq!(stats.progress_ratio(), 0.0);
    }

    #[test]
    fn test_merge_concurrent_empty() {
        let graph = ParallelCallGraph::new(1);
        let empty_graph = CallGraph::new();

        graph.merge_concurrent(empty_graph);

        let stats = graph.stats();
        assert_eq!(stats.total_nodes.load(Ordering::Relaxed), 0);
        assert_eq!(stats.total_edges.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_merge_concurrent_with_data() {
        let parallel_graph = ParallelCallGraph::new(2);
        let mut sequential_graph = CallGraph::new();

        let func1 = create_test_function_id("func1");
        let func2 = create_test_function_id("func2");
        sequential_graph.add_function(func1.clone(), false, false, 5, 10);
        sequential_graph.add_function(func2.clone(), false, false, 3, 6);
        sequential_graph.add_call(FunctionCall {
            caller: func1.clone(),
            callee: func2.clone(),
            call_type: CallType::Direct,
        });

        parallel_graph.merge_concurrent(sequential_graph);

        let result = parallel_graph.to_call_graph();
        assert!(result.get_function_info(&func1).is_some());
        assert!(result.get_function_info(&func2).is_some());
        assert_eq!(result.get_callees(&func1).len(), 1);
    }

    #[test]
    fn test_to_call_graph_preserves_data() {
        let graph = ParallelCallGraph::new(1);
        let func1 = create_test_function_id("func1");
        let func2 = create_test_function_id("func2");

        graph.add_function(func1.clone(), true, false, 5, 10);
        graph.add_function(func2.clone(), false, true, 3, 6);
        graph.add_call(func1.clone(), func2.clone(), CallType::Direct);

        let call_graph = graph.to_call_graph();

        // Verify func1
        let info1 = call_graph.get_function_info(&func1).unwrap();
        assert!(info1.0); // is_entry_point
        assert!(!info1.1); // is_test
        assert_eq!(info1.2, 5); // complexity
        assert_eq!(info1.3, 10); // lines

        // Verify func2
        let info2 = call_graph.get_function_info(&func2).unwrap();
        assert!(!info2.0); // is_entry_point
        assert!(info2.1); // is_test
        assert_eq!(info2.2, 3); // complexity
        assert_eq!(info2.3, 6); // lines

        // Verify call
        let callees = call_graph.get_callees(&func1);
        assert_eq!(callees.len(), 1);
        assert!(callees.contains(&func2));
    }

    #[test]
    fn test_parallel_config_builder() {
        let config = ParallelConfig::default()
            .with_threads(4)
            .deterministic(true);

        assert_eq!(config.num_threads, 4);
        assert!(config.deterministic);
    }

    #[test]
    fn test_parallel_config_with_progress() {
        let config = ParallelConfig::default().with_progress(|current, total| {
            println!("Progress: {}/{}", current, total);
        });

        assert!(config.progress_callback.is_some());
    }
}
