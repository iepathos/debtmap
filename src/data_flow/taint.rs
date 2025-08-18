use super::graph::{DataFlowGraph, DataFlowNode, EdgeKind, NodeId};
use super::sinks::SinkDetector;
use super::sources::{OperationType, SourceDetector};
use super::validation::ValidationDetector;
use crate::security::types::{InputSource, SinkOperation};
use std::collections::{HashMap, HashSet, VecDeque};

/// Represents the taint state of nodes in the graph
#[derive(Debug, Clone)]
pub struct TaintState {
    /// Nodes that are tainted (carry untrusted data)
    pub tainted_nodes: HashSet<NodeId>,
    /// Mapping of tainted nodes to their source
    pub sources: HashMap<NodeId, InputSource>,
    /// Nodes that have been validated/sanitized
    pub validated_nodes: HashSet<NodeId>,
}

impl TaintState {
    pub fn new() -> Self {
        Self {
            tainted_nodes: HashSet::new(),
            sources: HashMap::new(),
            validated_nodes: HashSet::new(),
        }
    }

    /// Mark a node as tainted
    pub fn taint(&mut self, node: NodeId, source: InputSource) {
        self.tainted_nodes.insert(node.clone());
        self.sources.insert(node, source);
    }

    /// Mark a node as validated
    pub fn validate(&mut self, node: NodeId) {
        self.validated_nodes.insert(node);
        // Validation doesn't remove taint, it just marks it as validated
    }

    /// Check if a node is tainted
    pub fn is_tainted(&self, node: &NodeId) -> bool {
        self.tainted_nodes.contains(node)
    }

    /// Check if a node is validated
    pub fn is_validated(&self, node: &NodeId) -> bool {
        self.validated_nodes.contains(node)
    }

    /// Get the source of taint for a node
    pub fn get_source(&self, node: &NodeId) -> Option<&InputSource> {
        self.sources.get(node)
    }
}

/// Result of taint analysis
#[derive(Debug)]
pub struct TaintAnalysis {
    pub state: TaintState,
    pub taint_paths: Vec<TaintPath>,
    pub statistics: AnalysisStatistics,
}

/// A path from source to sink
#[derive(Debug, Clone)]
pub struct TaintPath {
    pub source: InputSource,
    pub source_node: NodeId,
    pub sink: SinkOperation,
    pub sink_node: NodeId,
    pub path: Vec<NodeId>,
    pub has_validation: bool,
}

/// Statistics about the analysis
#[derive(Debug, Default)]
pub struct AnalysisStatistics {
    pub total_sources: usize,
    pub total_sinks: usize,
    pub tainted_nodes: usize,
    pub validated_nodes: usize,
    pub vulnerable_paths: usize,
    pub safe_paths: usize,
}

/// Performs taint analysis on data flow graphs
pub struct TaintAnalyzer {
    max_path_length: usize,
}

impl TaintAnalyzer {
    pub fn new() -> Self {
        Self {
            max_path_length: 20, // Limit path length to avoid infinite loops
        }
    }

    /// Analyze taint propagation through the graph
    pub fn analyze(
        &self,
        graph: &DataFlowGraph,
        source_detector: &SourceDetector,
        sink_detector: &SinkDetector,
        validation_detector: &ValidationDetector,
    ) -> TaintAnalysis {
        let mut state = TaintState::new();

        // Phase 1: Identify and mark source nodes
        let sources = self.identify_sources(graph, source_detector);
        for (node_id, source_type) in &sources {
            state.taint(node_id.clone(), *source_type);
        }

        // Phase 2: Propagate taint through the graph
        self.propagate_taint(graph, &mut state, validation_detector);

        // Phase 3: Find paths from sources to sinks
        let sinks = self.identify_sinks(graph, sink_detector);
        let taint_paths = self.find_taint_paths(graph, &state, &sources, &sinks);

        // Calculate statistics
        let statistics = AnalysisStatistics {
            total_sources: sources.len(),
            total_sinks: sinks.len(),
            tainted_nodes: state.tainted_nodes.len(),
            validated_nodes: state.validated_nodes.len(),
            vulnerable_paths: taint_paths.iter().filter(|p| !p.has_validation).count(),
            safe_paths: taint_paths.iter().filter(|p| p.has_validation).count(),
        };

        TaintAnalysis {
            state,
            taint_paths,
            statistics,
        }
    }

    /// Identify source nodes in the graph
    fn identify_sources(
        &self,
        graph: &DataFlowGraph,
        detector: &SourceDetector,
    ) -> HashMap<NodeId, InputSource> {
        let mut sources = HashMap::new();

        for (node_id, node) in graph.nodes() {
            // Check if this is a source node
            if let DataFlowNode::Source { kind, .. } = node {
                // Verify it's an actual read operation, not just pattern checking
                let context = format!("{:?}", node);
                if detector.classify_operation("", &context) == OperationType::Read {
                    sources.insert(node_id.clone(), *kind);
                }
            }

            // Also check parameters marked as potentially tainted
            if let DataFlowNode::Parameter { function, .. } = node {
                // Public API functions might receive untrusted input
                if self.is_public_api_function(function) {
                    sources.insert(node_id.clone(), InputSource::UserInput);
                }
            }
        }

        sources
    }

    /// Identify sink nodes in the graph
    fn identify_sinks(
        &self,
        graph: &DataFlowGraph,
        detector: &SinkDetector,
    ) -> HashMap<NodeId, SinkOperation> {
        let mut sinks = HashMap::new();

        for (node_id, node) in graph.nodes() {
            if let DataFlowNode::Sink { kind, .. } = node {
                if detector.is_dangerous_sink(node) {
                    sinks.insert(node_id.clone(), *kind);
                }
            }
        }

        sinks
    }

    /// Propagate taint through the graph
    fn propagate_taint(
        &self,
        graph: &DataFlowGraph,
        state: &mut TaintState,
        validation_detector: &ValidationDetector,
    ) {
        // Use a queue for breadth-first propagation
        let mut queue: VecDeque<NodeId> = state.tainted_nodes.iter().cloned().collect();
        let mut visited = HashSet::new();

        while let Some(current) = queue.pop_front() {
            if !visited.insert(current.clone()) {
                continue; // Already processed
            }

            // Get the source type for this tainted node
            let source = state.get_source(&current).copied();

            // Check if this node is a validator
            if let Some(node) = graph.get_node(&current) {
                if validation_detector.is_validation_node(node) {
                    state.validate(current.clone());
                }
            }

            // Propagate to connected nodes
            for edge in graph.outgoing_edges(&current) {
                let should_propagate = match &edge.kind {
                    EdgeKind::Assignment => true,
                    EdgeKind::Parameter { .. } => true,
                    EdgeKind::Return => true,
                    EdgeKind::MethodCall { .. } => true,
                    EdgeKind::FieldAccess { .. } => true,
                    EdgeKind::IndexAccess => true,
                    EdgeKind::Transform => true,
                    EdgeKind::ControlFlow => false, // Don't propagate through control flow
                    EdgeKind::Validation => {
                        // Mark as validated but still propagate
                        state.validate(edge.to.clone());
                        true
                    }
                };

                if should_propagate && !state.is_tainted(&edge.to) {
                    if let Some(src) = source {
                        state.taint(edge.to.clone(), src);
                        queue.push_back(edge.to.clone());
                    }
                }
            }
        }
    }

    /// Find paths from sources to sinks
    fn find_taint_paths(
        &self,
        graph: &DataFlowGraph,
        state: &TaintState,
        sources: &HashMap<NodeId, InputSource>,
        sinks: &HashMap<NodeId, SinkOperation>,
    ) -> Vec<TaintPath> {
        let mut paths = Vec::new();

        for (source_node, source_type) in sources {
            for (sink_node, sink_type) in sinks {
                // Use BFS to find path
                if let Some(path) = self.find_path_bfs(graph, source_node, sink_node) {
                    // Check if any node in the path is validated
                    let has_validation = path.iter().any(|node| state.is_validated(node));

                    paths.push(TaintPath {
                        source: *source_type,
                        source_node: source_node.clone(),
                        sink: *sink_type,
                        sink_node: sink_node.clone(),
                        path,
                        has_validation,
                    });
                }
            }
        }

        paths
    }

    /// Find a path between two nodes using BFS
    fn find_path_bfs(
        &self,
        graph: &DataFlowGraph,
        start: &NodeId,
        end: &NodeId,
    ) -> Option<Vec<NodeId>> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent_map: HashMap<NodeId, NodeId> = HashMap::new();

        queue.push_back(start.clone());
        visited.insert(start.clone());

        while let Some(current) = queue.pop_front() {
            if &current == end {
                // Reconstruct path
                let mut path = Vec::new();
                let mut node = end.clone();

                while &node != start {
                    path.push(node.clone());
                    if let Some(parent) = parent_map.get(&node) {
                        node = parent.clone();
                    } else {
                        break;
                    }
                }
                path.push(start.clone());
                path.reverse();

                return Some(path);
            }

            // Check path length limit
            if path_length(&parent_map, &current) >= self.max_path_length {
                continue;
            }

            for edge in graph.outgoing_edges(&current) {
                if !visited.contains(&edge.to) {
                    visited.insert(edge.to.clone());
                    parent_map.insert(edge.to.clone(), current.clone());
                    queue.push_back(edge.to.clone());
                }
            }
        }

        None
    }

    /// Check if a function is a public API
    fn is_public_api_function(&self, function_name: &str) -> bool {
        // Heuristics for public API functions
        function_name == "main"
            || function_name.starts_with("handle_")
            || function_name.starts_with("serve_")
            || function_name.starts_with("process_")
            || function_name.ends_with("_handler")
            || function_name.ends_with("_endpoint")
    }
}

/// Calculate path length from a node to the start
fn path_length(parent_map: &HashMap<NodeId, NodeId>, node: &NodeId) -> usize {
    let mut length = 0;
    let mut current = node.clone();

    while let Some(parent) = parent_map.get(&current) {
        length += 1;
        current = parent.clone();

        if length > 100 {
            break; // Prevent infinite loops
        }
    }

    length
}

impl Default for TaintAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
