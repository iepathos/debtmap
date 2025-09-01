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
