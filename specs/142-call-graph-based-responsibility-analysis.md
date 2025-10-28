---
number: 142
title: Call Graph-Based Responsibility Analysis
category: foundation
priority: critical
status: draft
dependencies: [141]
created: 2025-10-27
---

# Specification 142: Call Graph-Based Responsibility Analysis

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: Spec 141 (I/O and Side Effect Detection)

## Context

After detecting I/O and side effects (Spec 141), the next most valuable signal for responsibility classification comes from analyzing the **call graph**—the network of function calls within a codebase.

Call graph analysis reveals structural patterns that indicate responsibilities:

- **Orchestrator functions**: Call many other functions, coordinate workflows
- **Leaf functions**: Called by others, rarely call anything themselves
- **High coupling**: Functions heavily called suggest core abstractions
- **Utility clusters**: Groups of related functions that call each other

For example, a function that calls 10 different validation functions is clearly performing "Validation & Error Handling", regardless of its name. A function called by 20 other functions but calls nothing itself is a "Core Utility".

Current name-based heuristics can't detect these patterns. Call graph analysis adds ~30% weight to responsibility classification, bringing combined accuracy (with Spec 141) from ~70% to ~80%.

## Objective

Build a call graph for each analyzed file and use graph structure to classify function responsibilities. Detect orchestration patterns, coupling metrics, and functional clusters to enable accurate responsibility classification based on actual code relationships.

## Requirements

### Functional Requirements

**Call Graph Construction**:
- Build directed graph of function calls (caller → callee)
- Support intra-file calls (within same file)
- Support cross-file calls (track imports and module boundaries)
- Handle method calls, trait implementations, and closures
- Track call frequency and call sites
- Support all languages: Rust, Python, JavaScript, TypeScript

**Graph Metrics**:
- **Outdegree** (calls made): Number of functions this function calls
- **Indegree** (callers): Number of functions that call this function
- **Depth**: Distance from entry points (main, public APIs)
- **Centrality**: Importance in the call graph (betweenness, PageRank)
- **Clustering coefficient**: How tightly connected a function's neighbors are

**Pattern Detection**:
- **Orchestrators**: High outdegree, low indegree, coordinates multiple operations
- **Leaf nodes**: Zero outdegree, called by others but calls nothing
- **Hubs**: High indegree, frequently called utility/core functions
- **Bridges**: High betweenness centrality, connect different modules
- **Utilities**: High clustering coefficient, part of tight functional groups

**Responsibility Classification**:
- Orchestration & Coordination (high outdegree)
- Core Business Logic (high indegree + high centrality)
- Utility Functions (high indegree, low complexity)
- I/O Boundary (calls I/O functions from Spec 141)
- Pure Computation (no I/O calls, low coupling)
- Framework Integration (calls framework functions)

### Non-Functional Requirements

- **Performance**: Call graph construction <15% overhead on analysis time
- **Scalability**: Handle files with 500+ functions efficiently
- **Accuracy**: Correctly identify >85% of orchestrator vs leaf patterns
- **Memory**: Use incremental construction to limit memory growth

## Acceptance Criteria

- [ ] Call graph correctly captures function calls within a file
- [ ] Graph metrics (indegree, outdegree, centrality) are computed accurately
- [ ] Orchestrator pattern detection identifies coordination functions (outdegree > 5)
- [ ] Leaf node detection identifies pure utility functions (outdegree = 0)
- [ ] Hub detection identifies frequently-called core functions (indegree > 10)
- [ ] Responsibility classification uses graph metrics as primary signal
- [ ] Integration with Spec 141: I/O calls propagate through call graph
- [ ] Cross-file call tracking works for imported functions
- [ ] Performance overhead <15% on real codebases (debtmap, large projects)
- [ ] Test suite includes debtmap's own orchestrator patterns (e.g., src/orchestrator/)

## Technical Details

### Implementation Approach

**Phase 1: Call Graph Construction**

```rust
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::{dijkstra, connected_components};

pub struct CallGraph {
    /// Directed graph: edge from caller to callee
    graph: DiGraph<FunctionId, CallEdge>,
    /// Fast lookup from function ID to graph node
    node_map: HashMap<FunctionId, NodeIndex>,
    /// Metadata about each function
    function_info: HashMap<FunctionId, FunctionMetadata>,
}

#[derive(Debug, Clone)]
pub struct CallEdge {
    pub call_site: SourceLocation,
    pub is_conditional: bool,  // Inside if/match branch
    pub is_loop: bool,         // Inside loop
}

#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    pub name: String,
    pub io_profile: IoProfile,  // From Spec 141
    pub complexity: u32,
    pub visibility: Visibility,
}

impl CallGraph {
    pub fn from_ast(ast: &FileAst) -> Self {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Create nodes for all functions
        for func in ast.functions() {
            let node = graph.add_node(func.id);
            node_map.insert(func.id, node);
        }

        // Create edges for all calls
        for func in ast.functions() {
            let caller_node = node_map[&func.id];

            for call in func.calls() {
                if let Some(&callee_node) = node_map.get(&call.target_id) {
                    graph.add_edge(
                        caller_node,
                        callee_node,
                        CallEdge {
                            call_site: call.location,
                            is_conditional: call.is_conditional,
                            is_loop: call.in_loop,
                        },
                    );
                }
            }
        }

        CallGraph { graph, node_map, function_info: HashMap::new() }
    }
}
```

**Phase 2: Graph Metrics**

```rust
#[derive(Debug, Clone)]
pub struct GraphMetrics {
    pub outdegree: usize,        // Functions called
    pub indegree: usize,         // Times called by others
    pub depth: usize,            // Distance from entry points
    pub betweenness: f64,        // Centrality metric
    pub clustering: f64,         // How connected neighbors are
}

impl CallGraph {
    pub fn compute_metrics(&self, function_id: FunctionId) -> GraphMetrics {
        let node = self.node_map[&function_id];

        let outdegree = self.graph
            .edges_directed(node, petgraph::Direction::Outgoing)
            .count();

        let indegree = self.graph
            .edges_directed(node, petgraph::Direction::Incoming)
            .count();

        let betweenness = self.compute_betweenness_centrality(node);
        let clustering = self.compute_clustering_coefficient(node);
        let depth = self.compute_depth_from_entry_points(node);

        GraphMetrics {
            outdegree,
            indegree,
            depth,
            betweenness,
            clustering,
        }
    }

    fn compute_betweenness_centrality(&self, node: NodeIndex) -> f64 {
        // Use Brandes algorithm for betweenness centrality
        // Measures how often this node appears on shortest paths
        // High betweenness = bridge between modules
        petgraph::algo::betweenness_centrality(&self.graph, false)
            .get(&node)
            .copied()
            .unwrap_or(0.0)
    }

    fn compute_clustering_coefficient(&self, node: NodeIndex) -> f64 {
        // Measures how connected a node's neighbors are
        // High clustering = part of tight functional group
        let neighbors: Vec<_> = self.graph
            .neighbors(node)
            .collect();

        if neighbors.len() < 2 {
            return 0.0;
        }

        let possible_edges = neighbors.len() * (neighbors.len() - 1);
        let actual_edges = neighbors.iter()
            .flat_map(|&n1| {
                neighbors.iter()
                    .filter(move |&&n2| n1 != n2 && self.graph.contains_edge(n1, n2))
            })
            .count();

        actual_edges as f64 / possible_edges as f64
    }

    fn compute_depth_from_entry_points(&self, node: NodeIndex) -> usize {
        // Find distance from entry points (main, public API functions)
        let entry_points = self.find_entry_points();

        entry_points.iter()
            .filter_map(|&entry| {
                dijkstra(&self.graph, entry, Some(node), |_| 1)
                    .get(&node)
                    .copied()
            })
            .min()
            .unwrap_or(usize::MAX)
    }

    fn find_entry_points(&self) -> Vec<NodeIndex> {
        // Entry points are:
        // 1. Functions with zero incoming calls (potential main)
        // 2. Public API functions (high visibility)
        self.graph.node_indices()
            .filter(|&node| {
                let indegree = self.graph
                    .edges_directed(node, petgraph::Direction::Incoming)
                    .count();

                let func_id = self.graph[node];
                let is_public = self.function_info
                    .get(&func_id)
                    .map(|info| matches!(info.visibility, Visibility::Public))
                    .unwrap_or(false);

                indegree == 0 || is_public
            })
            .collect()
    }
}
```

**Phase 3: Pattern Detection**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallGraphPattern {
    Orchestrator,      // High outdegree, coordinates operations
    LeafNode,          // Zero outdegree, pure utility
    Hub,               // High indegree, core abstraction
    Bridge,            // High betweenness, connects modules
    UtilityCluster,    // High clustering, part of tight group
    IoGateway,         // Calls I/O functions, boundary layer
}

impl CallGraph {
    pub fn detect_pattern(&self, function_id: FunctionId) -> Vec<CallGraphPattern> {
        let metrics = self.compute_metrics(function_id);
        let io_profile = self.function_info
            .get(&function_id)
            .map(|info| &info.io_profile);

        let mut patterns = Vec::new();

        // Orchestrator: High outdegree, low indegree
        if metrics.outdegree >= 5 && metrics.indegree <= 3 {
            patterns.push(CallGraphPattern::Orchestrator);
        }

        // Leaf node: Never calls anything
        if metrics.outdegree == 0 {
            patterns.push(CallGraphPattern::LeafNode);
        }

        // Hub: Frequently called
        if metrics.indegree >= 10 {
            patterns.push(CallGraphPattern::Hub);
        }

        // Bridge: High betweenness centrality
        if metrics.betweenness > 0.5 {
            patterns.push(CallGraphPattern::Bridge);
        }

        // Utility cluster: Part of tight group
        if metrics.clustering > 0.6 && metrics.indegree >= 3 {
            patterns.push(CallGraphPattern::UtilityCluster);
        }

        // I/O Gateway: Calls I/O functions
        if let Some(profile) = io_profile {
            if !profile.is_pure || self.calls_io_functions(function_id) {
                patterns.push(CallGraphPattern::IoGateway);
            }
        }

        patterns
    }

    fn calls_io_functions(&self, function_id: FunctionId) -> bool {
        let node = self.node_map[&function_id];

        self.graph
            .neighbors(node)
            .any(|callee_node| {
                let callee_id = self.graph[callee_node];
                self.function_info
                    .get(&callee_id)
                    .map(|info| !info.io_profile.is_pure)
                    .unwrap_or(false)
            })
    }
}
```

**Phase 4: Responsibility Classification**

```rust
pub fn classify_responsibility_from_call_graph(
    call_graph: &CallGraph,
    function_id: FunctionId,
) -> ResponsibilityClassification {
    let patterns = call_graph.detect_pattern(function_id);
    let metrics = call_graph.compute_metrics(function_id);

    // Priority order: Most specific patterns first
    if patterns.contains(&CallGraphPattern::Orchestrator) {
        return ResponsibilityClassification {
            primary: "Orchestration & Coordination",
            confidence: 0.85,
            evidence: format!(
                "Calls {} functions, orchestrating complex workflow",
                metrics.outdegree
            ),
        };
    }

    if patterns.contains(&CallGraphPattern::IoGateway) {
        return ResponsibilityClassification {
            primary: "I/O & External Communication",
            confidence: 0.80,
            evidence: "Acts as gateway to I/O operations",
        };
    }

    if patterns.contains(&CallGraphPattern::Hub) {
        return ResponsibilityClassification {
            primary: "Core Business Logic",
            confidence: 0.75,
            evidence: format!(
                "Called by {} functions, central to module",
                metrics.indegree
            ),
        };
    }

    if patterns.contains(&CallGraphPattern::LeafNode) {
        return ResponsibilityClassification {
            primary: "Utility & Helper Functions",
            confidence: 0.70,
            evidence: "Pure function with no external calls",
        };
    }

    if patterns.contains(&CallGraphPattern::UtilityCluster) {
        return ResponsibilityClassification {
            primary: "Domain-Specific Utilities",
            confidence: 0.65,
            evidence: "Part of tightly-connected functional group",
        };
    }

    // Default fallback
    ResponsibilityClassification {
        primary: "General Logic",
        confidence: 0.40,
        evidence: "No strong call graph pattern detected",
    }
}
```

### Architecture Changes

**New Module**: `src/analysis/call_graph.rs`
- Call graph construction from AST
- Graph metrics computation (indegree, outdegree, centrality)
- Pattern detection algorithms

**New Module**: `src/analysis/graph_metrics/`
- `centrality.rs` - Betweenness, closeness, PageRank algorithms
- `clustering.rs` - Clustering coefficient, community detection
- `patterns.rs` - Pattern matching on graph structure

**Integration Point**: `src/organization/god_object_analysis.rs`
- Use call graph patterns as primary signal for responsibility classification
- Combine with I/O detection (Spec 141) for comprehensive analysis
- Fall back to name heuristics only when graph signals are weak

**Dependencies**:
```toml
[dependencies]
petgraph = "0.6"  # Graph data structure and algorithms
```

### Data Structures

```rust
#[derive(Debug, Clone)]
pub struct ResponsibilityClassification {
    pub primary: &'static str,
    pub confidence: f64,  // 0.0 to 1.0
    pub evidence: String,
}

#[derive(Debug, Clone)]
pub struct FunctionId {
    pub file_path: PathBuf,
    pub function_name: String,
    pub line_number: usize,
}
```

## Dependencies

- **Prerequisites**:
  - Spec 141 (I/O Detection) - provides I/O profiles used in call graph analysis
- **Affected Components**:
  - `src/organization/god_object_analysis.rs` - responsibility classification
  - `src/analyzers/` - AST parsers must extract function calls
  - `Cargo.toml` - add petgraph dependency
- **External Dependencies**:
  - `petgraph` crate for graph algorithms

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orchestrator_detection() {
        let code = r#"
        fn process_user_request(req: Request) -> Response {
            validate_input(&req);
            authenticate_user(&req);
            authorize_action(&req);
            execute_business_logic(&req);
            format_response(&req);
            log_request(&req);
        }
        "#;

        let ast = parse_rust(code);
        let call_graph = CallGraph::from_ast(&ast);
        let patterns = call_graph.detect_pattern(ast.functions[0].id);

        assert!(patterns.contains(&CallGraphPattern::Orchestrator));
    }

    #[test]
    fn leaf_node_detection() {
        let code = r#"
        fn calculate_tax(amount: f64) -> f64 {
            amount * 0.15
        }
        "#;

        let ast = parse_rust(code);
        let call_graph = CallGraph::from_ast(&ast);
        let patterns = call_graph.detect_pattern(ast.functions[0].id);

        assert!(patterns.contains(&CallGraphPattern::LeafNode));
    }

    #[test]
    fn hub_detection() {
        let code = r#"
        fn parse_json(s: &str) -> Value { ... }  // Called by many functions

        fn load_config() { let c = parse_json(...); }
        fn process_api() { let d = parse_json(...); }
        fn handle_request() { let r = parse_json(...); }
        // ... 10+ more callers
        "#;

        let ast = parse_rust(code);
        let call_graph = CallGraph::from_ast(&ast);
        let patterns = call_graph.detect_pattern(/* parse_json ID */);

        assert!(patterns.contains(&CallGraphPattern::Hub));
    }

    #[test]
    fn io_gateway_detection() {
        let code = r#"
        fn read_config_file() -> Config {
            let content = std::fs::read_to_string("config.toml");  // I/O
            parse_toml(&content)
        }
        "#;

        let ast = parse_rust(code);
        let io_analyzer = RustIoAnalyzer::new();
        let mut call_graph = CallGraph::from_ast(&ast);

        // Attach I/O profiles from Spec 141
        for func in ast.functions() {
            let io_profile = io_analyzer.analyze_function(&func);
            call_graph.attach_io_profile(func.id, io_profile);
        }

        let patterns = call_graph.detect_pattern(ast.functions[0].id);
        assert!(patterns.contains(&CallGraphPattern::IoGateway));
    }
}
```

### Integration Tests

```rust
#[test]
fn analyze_debtmap_orchestrator() {
    // Test on debtmap's actual orchestrator code
    let ast = parse_file("src/orchestrator/workflow_orchestrator.rs");
    let call_graph = CallGraph::from_ast(&ast);

    // Should detect orchestration patterns
    let orchestrators: Vec<_> = ast.functions()
        .filter(|f| {
            call_graph
                .detect_pattern(f.id)
                .contains(&CallGraphPattern::Orchestrator)
        })
        .collect();

    assert!(!orchestrators.is_empty());
}

#[test]
fn cross_file_call_tracking() {
    let files = vec![
        parse_file("src/main.rs"),
        parse_file("src/config.rs"),
        parse_file("src/analyzer.rs"),
    ];

    let call_graph = CallGraph::from_multiple_files(&files);

    // Verify cross-file calls are tracked
    assert!(call_graph.has_cross_file_calls());
}
```

### Performance Tests

```rust
#[test]
fn call_graph_performance() {
    let large_file = parse_file("src/priority/formatter.rs");  // 2889 lines

    let start = Instant::now();
    let call_graph = CallGraph::from_ast(&large_file);
    let construction_time = start.elapsed();

    let start = Instant::now();
    for func in large_file.functions() {
        let _ = call_graph.compute_metrics(func.id);
    }
    let metrics_time = start.elapsed();

    // Construction + metrics should be <15% overhead
    assert!(construction_time < Duration::from_millis(50));
    assert!(metrics_time < Duration::from_millis(100));
}
```

## Documentation Requirements

### Code Documentation

- Rustdoc for all public APIs
- Examples of call graph construction and pattern detection
- Documentation of graph algorithms used (betweenness, clustering)

### User Documentation

Update README.md:
```markdown
## Call Graph Analysis

Debtmap builds a call graph to understand function relationships:

**Detected Patterns**:
- **Orchestrators**: Coordinate multiple operations (5+ calls)
- **Hubs**: Frequently called core utilities (10+ callers)
- **Leaf Nodes**: Pure functions with no external calls
- **I/O Gateways**: Bridge between business logic and I/O

**Metrics Computed**:
- Indegree: How many functions call this one
- Outdegree: How many functions this one calls
- Betweenness Centrality: Importance in module connectivity
- Clustering: Participation in functional groups
```

### Architecture Updates

Update ARCHITECTURE.md:
```markdown
## Call Graph Analysis (Spec 142)

1. **Construction**: Build directed graph of function calls
2. **Metrics**: Compute indegree, outdegree, centrality
3. **Pattern Detection**: Identify orchestrators, hubs, leaf nodes
4. **Integration with I/O**: Propagate I/O profiles through call chain
5. **Responsibility Classification**: Use graph structure as primary signal
```

## Implementation Notes

### Handling Cross-File Calls

For accurate call graph construction, track imports and module boundaries:

```rust
impl CallGraph {
    pub fn from_multiple_files(files: &[FileAst]) -> Self {
        let mut graph = Self::new();

        // First pass: Create nodes for all functions
        for file in files {
            for func in file.functions() {
                graph.add_function(func);
            }
        }

        // Second pass: Resolve cross-file calls
        for file in files {
            for func in file.functions() {
                for call in func.calls() {
                    // Resolve call target across files
                    if let Some(target_id) = graph.resolve_call(&call, &file.imports) {
                        graph.add_call_edge(func.id, target_id);
                    }
                }
            }
        }

        graph
    }
}
```

### Optimization: Incremental Construction

For large projects, use incremental call graph updates:

```rust
pub struct IncrementalCallGraph {
    graph: CallGraph,
    dirty_nodes: HashSet<FunctionId>,
}

impl IncrementalCallGraph {
    pub fn update_function(&mut self, function: &FunctionAst) {
        // Mark affected nodes as dirty
        self.dirty_nodes.insert(function.id);

        // Recompute only dirty subgraph
        self.recompute_dirty_metrics();
    }
}
```

## Migration and Compatibility

### Breaking Changes

None - this is additive functionality.

### Integration with Spec 141

Call graph analysis builds on I/O detection:

```rust
// I/O profiles propagate through call graph
fn propagate_io_profiles(call_graph: &mut CallGraph) {
    for function_id in call_graph.topological_order() {
        let direct_io = /* from Spec 141 */;
        let indirect_io = call_graph
            .callees(function_id)
            .map(|callee| call_graph.io_profile(callee))
            .fold(IoProfile::empty(), |acc, profile| acc.merge(profile));

        call_graph.set_io_profile(
            function_id,
            direct_io.merge(indirect_io)
        );
    }
}
```

## Expected Impact

### Accuracy Improvement

- **Spec 141 alone**: ~65-70% accuracy
- **Spec 141 + Spec 142**: ~80% accuracy
- **Improvement from call graph**: +10-15 percentage points

### Examples

**Before (name-based + I/O)**:
```rust
fn handle_request(req: Request) -> Response {
    validate(req);
    process(req);
    format_response(req)
}
// Classified as "Request Handling" (name-based)
```

**After (call graph)**:
```rust
fn handle_request(req: Request) -> Response {
    validate(req);      // ← Outdegree = 3
    process(req);       // ← Orchestration pattern
    format_response(req)
}
// Correctly classified as "Orchestration & Coordination"
```

### Foundation for Multi-Signal (Spec 145)

This provides the second-most-important signal:
- I/O detection (Spec 141): 40% weight
- **Call graph analysis (Spec 142): 30% weight** ← This spec
- Type signatures (Spec 147): 15% weight
- Side effects (Spec 141): 10% weight
- Name heuristics: 5% weight
- **Combined accuracy**: ~85%
