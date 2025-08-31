---
number: 70
title: Parallel Call Graph Construction
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-01-31
---

# Specification 70: Parallel Call Graph Construction

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current call graph construction in debtmap is a significant performance bottleneck, taking up approximately 40-50% of the total analysis time. The implementation performs three sequential passes over all project files:

1. Initial call graph extraction from individual files
2. Multi-file cross-module resolution
3. Enhanced analysis for trait dispatch, function pointers, and framework patterns

With 282 Rust files taking ~38 seconds total, the call graph construction alone accounts for ~15-19 seconds. This sequential processing fails to leverage modern multi-core processors effectively, despite the README claiming "Parallel processing with Rayon for analyzing massive codebases in seconds."

## Objective

Parallelize the call graph construction process to reduce execution time by 60-70% through effective use of Rayon's parallel iterators and concurrent data structures, while maintaining the accuracy and completeness of the current sequential implementation.

## Requirements

### Functional Requirements

- Maintain exact same call graph output as sequential implementation
- Support all existing call graph features:
  - Basic function calls
  - Trait dispatch analysis
  - Function pointer tracking
  - Cross-module resolution
  - Framework pattern detection
- Preserve deterministic results across runs
- Handle partial failures gracefully without data corruption

### Non-Functional Requirements

- Achieve 60-70% reduction in call graph construction time
- Scale linearly with available CPU cores up to 8 cores
- Maintain memory usage within 1.5x of sequential implementation
- Support cancellation without leaving partial state
- Provide progress feedback during long operations

## Acceptance Criteria

- [ ] Call graph construction uses Rayon parallel iterators for all three passes
- [ ] Benchmark shows 60-70% time reduction on 4+ core machines
- [ ] All existing tests pass without modification
- [ ] New parallel-specific tests verify correctness under concurrent load
- [ ] Memory usage stays within 1.5x of sequential baseline
- [ ] Results are deterministic (same output for same input)
- [ ] Progress indicator shows current file being processed
- [ ] Documentation updated to reflect parallel architecture

## Technical Details

### Implementation Approach

#### Phase 1: Parallel First Pass (Individual File Analysis)
```rust
fn build_initial_call_graph_parallel(files: &[PathBuf]) -> CallGraph {
    let file_graphs: Vec<_> = files
        .par_iter()
        .filter_map(|path| {
            analyze_rust_file_for_call_graph(path)
                .map_err(|e| log::debug!("Failed to analyze {}: {}", path.display(), e))
                .ok()
        })
        .collect();
    
    // Parallel reduction to merge graphs
    file_graphs
        .into_par_iter()
        .reduce(CallGraph::new, |mut acc, graph| {
            acc.merge(graph);
            acc
        })
}
```

#### Phase 2: Parallel Cross-Module Resolution
```rust
fn resolve_cross_module_parallel(
    call_graph: &mut CallGraph,
    workspace_files: &HashMap<PathBuf, ParsedFile>
) -> Result<()> {
    // Group files by module for better cache locality
    let module_groups = group_files_by_module(workspace_files);
    
    // Process module groups in parallel
    let resolved_calls: Vec<_> = module_groups
        .par_iter()
        .flat_map(|(module, files)| {
            resolve_module_calls(files, call_graph)
        })
        .collect();
    
    // Apply resolved calls to main graph
    for (caller, callee) in resolved_calls {
        call_graph.add_call(caller, callee);
    }
    
    Ok(())
}
```

#### Phase 3: Concurrent Enhanced Analysis
```rust
fn enhanced_analysis_parallel(
    builder: &mut EnhancedCallGraphBuilder,
    workspace_files: &HashMap<PathBuf, ParsedFile>
) -> Result<()> {
    // Use Arc<Mutex> for shared builder state
    let builder = Arc::new(Mutex::new(builder));
    
    workspace_files
        .par_iter()
        .try_for_each(|(path, parsed)| {
            let mut local_calls = Vec::new();
            
            // Analyze without holding lock
            analyze_traits(&parsed, &mut local_calls)?;
            analyze_function_pointers(&parsed, &mut local_calls)?;
            analyze_framework_patterns(&parsed, &mut local_calls)?;
            
            // Batch update to minimize lock contention
            if !local_calls.is_empty() {
                let mut builder = builder.lock().unwrap();
                builder.add_calls(path, local_calls);
            }
            
            Ok::<_, anyhow::Error>(())
        })?;
    
    Ok(())
}
```

### Architecture Changes

1. **Thread-Safe Call Graph Structure**
   - Replace `HashMap` with `DashMap` for concurrent access
   - Use `Arc<RwLock>` for shared state that needs mutation
   - Implement lock-free data structures where possible

2. **Work Distribution Strategy**
   - Use work-stealing queues for dynamic load balancing
   - Group related files to improve cache locality
   - Implement chunking to reduce synchronization overhead

3. **Progress Monitoring**
   - Add atomic counter for processed files
   - Implement progress callback system
   - Support for graceful cancellation

### Data Structures

```rust
pub struct ParallelCallGraph {
    nodes: DashMap<FunctionId, NodeInfo>,
    edges: DashMap<FunctionId, DashSet<FunctionId>>,
    stats: Arc<AtomicStats>,
    progress: Arc<AtomicUsize>,
}

struct AtomicStats {
    total_nodes: AtomicUsize,
    total_edges: AtomicUsize,
    files_processed: AtomicUsize,
}

impl ParallelCallGraph {
    pub fn merge_concurrent(&self, other: CallGraph) {
        // Lock-free merge operation
        other.nodes.par_iter().for_each(|(id, info)| {
            self.nodes.insert(id.clone(), info.clone());
        });
        
        other.edges.par_iter().for_each(|(from, to_set)| {
            let entry = self.edges.entry(from.clone()).or_default();
            for to in to_set.iter() {
                entry.insert(to.clone());
            }
        });
    }
}
```

### APIs and Interfaces

```rust
pub trait ParallelCallGraphBuilder {
    /// Build call graph with parallel processing
    fn build_parallel(&self, files: &[PathBuf]) -> Result<CallGraph>;
    
    /// Set number of worker threads (0 = use all cores)
    fn with_threads(self, num_threads: usize) -> Self;
    
    /// Set progress callback
    fn with_progress<F>(self, callback: F) -> Self
    where
        F: Fn(usize, usize) + Send + Sync + 'static;
    
    /// Enable deterministic mode (slower but reproducible)
    fn deterministic(self, enabled: bool) -> Self;
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/main.rs` - Call graph construction calls
  - `src/analyzers/rust_call_graph.rs` - Core extraction logic
  - `src/priority/call_graph.rs` - Graph data structure
- **External Dependencies**:
  - `rayon` - Already in dependencies
  - `dashmap` - New concurrent HashMap (add to Cargo.toml)
  - `crossbeam` - For concurrent data structures

## Testing Strategy

- **Unit Tests**:
  - Test parallel graph merging produces same result as sequential
  - Verify thread safety with concurrent modifications
  - Test cancellation and error handling
  
- **Integration Tests**:
  - Compare parallel vs sequential output on real codebases
  - Benchmark performance across different file counts
  - Test with various thread counts (1, 2, 4, 8, 16)
  
- **Performance Tests**:
  - Measure speedup factor vs core count
  - Monitor memory usage under load
  - Profile lock contention and overhead
  
- **Stress Tests**:
  - Large codebases (1000+ files)
  - Deeply nested call chains
  - Circular dependencies
  - Concurrent analysis of same modules

## Documentation Requirements

- **Code Documentation**:
  - Document thread safety guarantees
  - Explain synchronization points
  - Add examples of parallel usage
  
- **User Documentation**:
  - Update README performance claims with benchmarks
  - Add `--jobs` flag documentation
  - Include troubleshooting for parallel issues
  
- **Architecture Updates**:
  - Update ARCHITECTURE.md with parallel processing flow
  - Document concurrent data structures used
  - Add sequence diagrams for parallel phases

## Implementation Notes

### Performance Considerations
- Use `par_iter()` for collections > 100 items
- Batch small operations to reduce overhead
- Prefer lock-free data structures where possible
- Use thread-local storage for temporary data

### Potential Pitfalls
- Watch for false sharing in concurrent data structures
- Ensure deterministic ordering when required
- Handle thread panics gracefully
- Avoid over-parallelization of small workloads

### Optimization Opportunities
- Pre-sort files by size for better load balancing
- Cache parsed ASTs between passes
- Use SIMD for string comparisons where applicable
- Implement parallel graph algorithms (e.g., parallel BFS/DFS)

## Migration and Compatibility

During the prototype phase, breaking changes are allowed. The parallel implementation will:

- Replace the existing sequential implementation entirely
- May change internal graph representation
- Could alter the order of items in output (while maintaining correctness)
- Might require minimum Rust version bump for better async support

Users can opt for deterministic mode if they need reproducible results across runs, trading some performance for consistency.