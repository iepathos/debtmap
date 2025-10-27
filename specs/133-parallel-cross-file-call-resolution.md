---
number: 133
title: Parallel Cross-File Call Resolution with Batching
category: optimization
priority: medium
status: draft
dependencies: [132]
created: 2025-10-26
---

# Specification 133: Parallel Cross-File Call Resolution with Batching

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 132 (Eliminate Redundant AST Parsing)

## Context

The `resolve_cross_file_calls()` method in `src/priority/call_graph/cross_file.rs` currently performs call resolution sequentially, processing each unresolved call one at a time and applying updates immediately. This approach has two performance limitations:

1. **Sequential Resolution**: Each call is resolved in a loop, missing opportunities for parallel processing
2. **Immediate Mutation**: Each resolution immediately mutates the graph's indexes and edges, preventing parallelization

Current implementation:
```rust
pub fn resolve_cross_file_calls(&mut self) {
    let all_functions: Vec<FunctionId> = self.get_all_functions().cloned().collect();
    let calls_to_resolve = self.find_unresolved_calls();

    for call in calls_to_resolve {  // Sequential
        if let Some(resolved_callee) = Self::resolve_call_with_advanced_matching(...) {
            self.apply_call_resolution(&call, &resolved_callee);  // Immediate mutation
        }
    }
}
```

**Performance Analysis**:
- For a 392-file codebase with ~1000-2000 unresolved calls
- Current: Sequential O(n) processing with scattered mutations
- Potential: Parallel resolution followed by bulk updates

The resolution logic in `resolve_call_with_advanced_matching()` is already a pure function that operates on immutable data (`all_functions` slice). This makes it an ideal candidate for parallelization using Rayon's parallel iterators.

**Why This Matters**:
While individual call resolutions are fast (~microseconds), processing thousands of them sequentially adds up. On multi-core systems, parallel resolution can provide 10-15% speedup by utilizing multiple CPU cores during the read-only resolution phase.

## Objective

Optimize cross-file call resolution by separating the process into two distinct phases: (1) parallel read-only resolution of calls, and (2) sequential bulk application of updates to the graph, achieving a 10-15% reduction in call graph construction time.

## Requirements

### Functional Requirements

1. **Parallel Resolution Phase**
   - Use Rayon's `par_iter()` to resolve calls concurrently
   - Leverage existing pure `resolve_call_with_advanced_matching()` function
   - Collect all successful resolutions into a vector of tuples: `(original_call, resolved_callee)`
   - Maintain deterministic resolution behavior (same results as sequential version)

2. **Bulk Update Phase**
   - Apply all resolutions sequentially after parallel phase completes
   - Update graph indexes and edges in batch
   - Maintain data structure consistency throughout updates
   - Preserve existing error handling and edge case behavior

3. **Functional Correctness**
   - Produce identical call graph results as current sequential implementation
   - Maintain all resolution heuristics and matching logic
   - Preserve handling of ambiguous matches and unresolvable calls
   - No changes to graph structure or data representation

### Non-Functional Requirements

1. **Performance**
   - 10-15% reduction in `resolve_cross_file_calls()` execution time
   - Scale efficiently with number of CPU cores (2-8 cores)
   - No performance regression for small codebases (<100 files)
   - Minimal overhead from parallel processing infrastructure

2. **Thread Safety**
   - Ensure `resolve_call_with_advanced_matching()` is safe for concurrent execution
   - No data races or concurrent access violations
   - Proper synchronization if needed for shared data structures

3. **Resource Efficiency**
   - Memory overhead: < 10MB for collecting resolutions vector
   - No excessive temporary allocations during parallel processing
   - Efficient use of CPU cores without over-subscription

## Acceptance Criteria

- [ ] `resolve_cross_file_calls()` uses parallel resolution for unresolved calls
- [ ] Resolution phase is read-only and thread-safe
- [ ] Update phase applies all resolutions sequentially
- [ ] All existing call graph tests pass without modification
- [ ] Call graph results are identical to sequential implementation
- [ ] Performance improvement of 10-15% measured with benchmarks
- [ ] No performance regression for small projects (<100 files)
- [ ] No new clippy warnings or thread-safety issues
- [ ] Memory usage increase is < 10MB
- [ ] Code maintains functional programming principles

## Technical Details

### Implementation Approach

**File**: `src/priority/call_graph/cross_file.rs`

**Current Implementation**:
```rust
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
```

**Optimized Implementation**:
```rust
use rayon::prelude::*;

pub fn resolve_cross_file_calls(&mut self) {
    let all_functions: Vec<FunctionId> = self.get_all_functions().cloned().collect();
    let calls_to_resolve = self.find_unresolved_calls();

    // Phase 1: Parallel resolution (read-only, no mutation)
    // This phase can utilize all CPU cores for independent resolutions
    let resolutions: Vec<(FunctionCall, FunctionId)> = calls_to_resolve
        .par_iter()  // Parallel iteration with Rayon
        .filter_map(|call| {
            // Pure function call - safe for parallel execution
            Self::resolve_call_with_advanced_matching(
                &all_functions,
                &call.callee.name,
                &call.caller.file,
            ).map(|resolved_callee| {
                // Return tuple of (original_call, resolved_callee)
                (call.clone(), resolved_callee)
            })
        })
        .collect();

    // Phase 2: Sequential bulk update (mutation phase)
    // Apply all resolutions to the graph in sequence
    for (original_call, resolved_callee) in resolutions {
        self.apply_call_resolution(&original_call, &resolved_callee);
    }
}
```

### Architecture Changes

**Modified Components**:
- `src/priority/call_graph/cross_file.rs`: Single method optimization
- No changes to public API or call graph structure
- No changes to resolution logic or matching algorithms

**Data Flow**:
```
Before:
  for call in unresolved_calls {
      resolve → apply_mutation
      resolve → apply_mutation
      ...
  }

After:
  // Phase 1: Parallel
  [call1, call2, ...] → par_iter → [resolution1, resolution2, ...]

  // Phase 2: Sequential
  for resolution in resolutions {
      apply_mutation
  }
```

### Thread Safety Analysis

**Why Parallel Resolution is Safe**:

1. **Pure Function**: `resolve_call_with_advanced_matching()` is already a pure, static method
   - Takes immutable references as input
   - Returns new data without modifying arguments
   - No shared mutable state

2. **Read-Only Data**: All inputs are immutable during resolution phase
   - `all_functions`: Cloned vector, no shared mutation
   - `call.callee.name` and `call.caller.file`: Immutable string references
   - No access to `self` during parallel phase

3. **Independent Operations**: Each call resolution is independent
   - No dependencies between resolutions
   - No shared data structures being modified
   - Results are collected into new vector

**Verification**:
```rust
// Static method signature confirms thread safety
impl CallGraph {
    fn resolve_call_with_advanced_matching(
        all_functions: &[FunctionId],  // Shared read-only
        callee_name: &str,              // Shared read-only
        caller_file: &PathBuf,          // Shared read-only
    ) -> Option<FunctionId> {
        // Pure logic, no mutation
    }
}
```

### Performance Considerations

**Expected Speedup Calculation**:

Assume:
- 1000 unresolved calls
- 4 CPU cores available
- Resolution time: 50% of `resolve_cross_file_calls()` total time
- Update time: 50% of total time

Sequential:
- Resolution: 100ms (sequential)
- Updates: 100ms (sequential)
- Total: 200ms

Parallel:
- Resolution: 25ms (4x speedup on 4 cores, assuming ideal parallelism)
- Updates: 100ms (sequential, unchanged)
- Total: 125ms
- **Speedup: 37.5%** (best case)

Realistic (accounting for overhead):
- Resolution: 35ms (2.8x speedup)
- Updates: 100ms
- Total: 135ms
- **Speedup: 32.5%**

Conservative estimate considering batching overhead:
- **10-15% overall speedup** for `resolve_cross_file_calls()`

**Scaling Characteristics**:
- 2 cores: ~8% speedup
- 4 cores: ~12% speedup
- 8 cores: ~15% speedup (diminishing returns due to overhead)

### Memory Analysis

**Additional Memory Usage**:

1. **Resolutions Vector**:
   ```rust
   Vec<(FunctionCall, FunctionId)>
   ```
   - Size: Number of successful resolutions
   - Per-entry: ~200 bytes (FunctionCall + FunctionId structs)
   - For 1000 resolutions: ~200KB
   - For 2000 resolutions: ~400KB

2. **Temporary Allocations**:
   - Rayon thread pool: Minimal overhead (already created)
   - Parallel iterator state: ~1KB per thread
   - Total overhead: < 10MB even for large projects

**Memory Lifecycle**:
1. Collect all functions (already done in current implementation)
2. Find unresolved calls (already done in current implementation)
3. **NEW**: Allocate resolutions vector (~200KB-1MB)
4. Apply updates sequentially
5. Drop resolutions vector

### Potential Optimizations

**Future Improvements** (not in this spec):

1. **Parallel Updates**: Could potentially parallelize `apply_call_resolution()` if we:
   - Use concurrent data structures (DashMap)
   - Ensure thread-safe index updates
   - Complexity: High, benefit: Marginal (updates are fast)

2. **Lazy Resolution**: Only resolve calls as needed
   - Skip resolution for calls that are never queried
   - Complexity: Medium, benefit: Unknown

3. **Caching**: Cache resolution results across runs
   - Store resolution mappings in disk cache
   - Complexity: Medium, benefit: High for incremental analysis

## Dependencies

**Prerequisites**:
- **Spec 132**: Eliminate Redundant AST Parsing (recommended but not required)
  - Reduces overall call graph time, making this optimization more visible
  - Independent optimizations that compound for better total speedup

**Affected Components**:
- `src/priority/call_graph/cross_file.rs`: Primary implementation
- `src/priority/call_graph/types.rs`: No changes needed (already has required traits)

**External Dependencies**:
- `rayon`: Already in Cargo.toml for parallel processing
- No new dependencies required

## Testing Strategy

### Unit Tests

**Test parallel resolution produces same results**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_resolution_deterministic() {
        let mut graph = create_test_graph_with_unresolved_calls();

        // Clone graph for comparison
        let mut sequential_graph = graph.clone();

        // Resolve using parallel implementation
        graph.resolve_cross_file_calls();

        // Resolve using sequential implementation (old code preserved in test)
        sequential_graph.resolve_cross_file_calls_sequential();

        // Verify identical results
        assert_eq!(graph.edges, sequential_graph.edges);
        assert_eq!(graph.caller_index, sequential_graph.caller_index);
        assert_eq!(graph.callee_index, sequential_graph.callee_index);
    }

    #[test]
    fn test_parallel_resolution_handles_no_matches() {
        let mut graph = CallGraph::new();

        // Add unresolvable calls
        graph.edges.push(FunctionCall {
            caller: FunctionId::new(PathBuf::from("a.rs"), "func_a".into(), 1),
            callee: FunctionId::new(PathBuf::from("b.rs"), "nonexistent".into(), 0),
            call_type: CallType::Direct,
        });

        graph.resolve_cross_file_calls();

        // Should handle gracefully without panicking
        assert!(graph.find_unresolved_calls().len() >= 0);
    }

    #[test]
    fn test_parallel_resolution_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let graph = Arc::new(create_large_test_graph());

        // Run parallel resolution from multiple threads
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let graph_clone = Arc::clone(&graph);
                thread::spawn(move || {
                    let all_functions: Vec<_> = graph_clone.get_all_functions().cloned().collect();
                    let calls = graph_clone.find_unresolved_calls();

                    // Simulate parallel resolution
                    calls.par_iter().filter_map(|call| {
                        CallGraph::resolve_call_with_advanced_matching(
                            &all_functions,
                            &call.callee.name,
                            &call.caller.file,
                        )
                    }).count()
                })
            })
            .collect();

        // All threads should complete without panicking
        for handle in handles {
            handle.join().unwrap();
        }
    }
}
```

### Integration Tests

**Test with real codebase analysis**:
```rust
#[test]
fn test_full_call_graph_with_parallel_resolution() {
    let project_path = PathBuf::from("test_fixtures/rust_project");
    let mut graph = build_call_graph(&project_path).unwrap();

    let unresolved_before = graph.find_unresolved_calls().len();

    graph.resolve_cross_file_calls();

    let unresolved_after = graph.find_unresolved_calls().len();

    // Should resolve some calls
    assert!(unresolved_after < unresolved_before);

    // Graph should be valid
    assert!(graph.validate_consistency());
}
```

### Performance Tests

**Benchmark parallel vs sequential resolution**:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_resolve_cross_file_calls(c: &mut Criterion) {
    let mut group = c.benchmark_group("cross_file_resolution");

    // Setup: Create graph with many unresolved calls
    let graph = create_graph_with_1000_unresolved_calls();

    group.bench_function("parallel_resolution", |b| {
        b.iter(|| {
            let mut g = graph.clone();
            g.resolve_cross_file_calls();
            black_box(g);
        })
    });

    group.bench_function("sequential_resolution", |b| {
        b.iter(|| {
            let mut g = graph.clone();
            g.resolve_cross_file_calls_sequential();
            black_box(g);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_resolve_cross_file_calls);
criterion_main!(benches);
```

**Expected Results**:
```
parallel_resolution     time:   [85.0 ms 87.5 ms 90.0 ms]
sequential_resolution   time:   [95.0 ms 100.0 ms 105.0 ms]
                        change: [-15.0% -12.5% -10.0%] (improvement)
```

### Regression Tests

**Ensure no behavioral changes**:
- Run full test suite: `cargo test --all-features`
- Run call graph specific tests: `cargo test call_graph`
- Run integration tests: `cargo test --test '*'`
- Verify deterministic output on multiple runs

## Documentation Requirements

### Code Documentation

**Update method documentation**:
```rust
/// Resolve cross-file function calls using parallel processing
///
/// This method processes unresolved calls in two phases:
/// 1. **Parallel Resolution**: Uses Rayon to resolve calls concurrently
///    across multiple CPU cores, leveraging the pure functional nature
///    of the resolution logic.
/// 2. **Sequential Updates**: Applies all resolutions to the graph
///    sequentially to maintain data structure consistency.
///
/// # Performance
///
/// Expected speedup: 10-15% on multi-core systems (4-8 cores).
/// Scales linearly with number of unresolved calls and available cores.
///
/// # Thread Safety
///
/// The resolution phase is thread-safe because:
/// - Resolution logic is pure (no side effects)
/// - All input data is immutable during resolution
/// - No shared mutable state between threads
pub fn resolve_cross_file_calls(&mut self) {
    // Implementation...
}
```

**Document the pure function**:
```rust
/// Pure function to resolve a cross-file call
///
/// This function is safe for concurrent execution because it:
/// - Takes only immutable references
/// - Returns new data without modifying inputs
/// - Has no side effects or shared mutable state
///
/// # Thread Safety
///
/// This function is `Send + Sync` and can be safely called from
/// multiple threads concurrently with the same input data.
fn resolve_call_with_advanced_matching(
    all_functions: &[FunctionId],
    callee_name: &str,
    caller_file: &PathBuf,
) -> Option<FunctionId>
```

### Architecture Updates

Update `ARCHITECTURE.md`:
```markdown
## Call Graph Cross-File Resolution

The call graph uses a two-phase approach for resolving cross-file calls:

### Phase 1: Parallel Resolution
- Processes unresolved calls concurrently using Rayon
- Pure functional resolution logic enables safe parallelization
- Scales with number of CPU cores (2-8 cores)
- Collects all successful resolutions into a vector

### Phase 2: Sequential Updates
- Applies resolved calls to graph sequentially
- Updates caller/callee indexes and edges
- Maintains data structure consistency

**Performance**: This approach achieves 10-15% speedup compared to
sequential resolution by utilizing multiple CPU cores during the
read-only resolution phase.
```

### Performance Documentation

Update `book/src/parallel-processing.md`:
```markdown
### Cross-File Call Resolution

Debtmap uses parallel processing for cross-file call resolution:

- **Resolution Phase**: Pure functional resolution logic runs in parallel
  across CPU cores
- **Update Phase**: Graph mutations are applied sequentially for consistency

**Performance Impact**: 10-15% faster call graph construction on multi-core
systems (4-8 cores). Speedup scales with number of unresolved calls.

**Thread Safety**: Resolution logic is pure and side-effect free, making it
safe for concurrent execution without locks or synchronization.
```

## Implementation Notes

### Key Design Decisions

1. **Two-Phase Approach**:
   - **Why**: Separates read-only (parallelizable) from mutation (sequential)
   - **Trade-off**: Slight memory overhead for resolutions vector
   - **Benefit**: Clean separation of concerns, easy to reason about

2. **Use of `clone()` for FunctionCall**:
   - **Why**: `par_iter()` requires owned data for `filter_map`
   - **Trade-off**: Small allocation overhead per resolution
   - **Alternative**: Could use indexes instead, but adds complexity

3. **No Changes to Resolution Logic**:
   - **Why**: Minimize risk, focus on parallelization
   - **Benefit**: Easy to verify correctness (same algorithm)
   - **Future**: Could optimize resolution logic independently

### Potential Gotchas

1. **Rayon Thread Pool Configuration**:
   - Rayon uses global thread pool by default
   - May compete with other parallel operations
   - **Solution**: Rely on Rayon's work-stealing for efficiency

2. **Memory Pressure**:
   - Resolutions vector could be large for massive projects
   - **Solution**: Acceptable for typical projects; could batch in future

3. **Determinism**:
   - Parallel iteration must produce deterministic results
   - **Guarantee**: Resolution logic is deterministic for same inputs
   - **Verification**: Test with multiple runs

4. **Small Project Overhead**:
   - Parallel overhead may exceed benefit for <100 files
   - **Mitigation**: Rayon is efficient even for small datasets
   - **Measurement**: Benchmark to verify no regression

### Testing Considerations

**Preserve Sequential Implementation**:
For testing, keep a sequential version:
```rust
#[cfg(test)]
impl CallGraph {
    /// Sequential resolution for testing and comparison
    pub fn resolve_cross_file_calls_sequential(&mut self) {
        // Original implementation preserved for tests
    }
}
```

**Stress Testing**:
- Test with 10,000+ unresolved calls
- Verify memory usage stays bounded
- Check for thread starvation or deadlocks

## Migration and Compatibility

### Breaking Changes

**None** - This is an internal optimization with identical behavior.

### Backward Compatibility

- All existing tests pass without modification
- No changes to call graph output or structure
- No changes to public API

### Performance Characteristics

**Before**:
- Sequential processing: O(n) time
- No parallelism utilized

**After**:
- Parallel resolution: O(n/cores) time (theoretical)
- Sequential updates: O(n) time
- Overall: 10-15% improvement in practice

### Rollback Plan

If performance issues are discovered:
1. Remove `par_iter()`, replace with `.iter()`
2. No data structure changes needed
3. No migrations required

Simple rollback:
```rust
// Change from:
let resolutions: Vec<_> = calls_to_resolve.par_iter()...

// Back to:
let resolutions: Vec<_> = calls_to_resolve.iter()...
```

## Success Metrics

### Performance Metrics

- [ ] Cross-file resolution time reduced by 10-15%
- [ ] For 392-file project: Save 0.5-1.0 seconds
- [ ] Scales efficiently with 2-8 cores
- [ ] No regression for small projects (<100 files)

### Quality Metrics

- [ ] All tests pass
- [ ] No new clippy warnings
- [ ] Thread safety verified (no data races)
- [ ] Deterministic results across runs
- [ ] Memory usage < 10MB additional

### Validation

**Benchmark before/after**:
```bash
# Run benchmark
cargo bench --bench call_graph_bench -- resolve_cross_file

# Expected output:
# Before: 100.0 ms ± 5 ms
# After:  87.5 ms ± 5 ms (12.5% improvement)
```

**Integration test timing**:
```bash
# Measure full analysis time
time debtmap analyze . --lcov target/coverage/lcov.info

# Check "Extracting cross-file call relationships" phase
# Expected: 0.5-1.0 second reduction
```

## Interaction with Other Specs

### Spec 132: Eliminate Redundant AST Parsing

**Relationship**: Complementary optimizations
- Spec 132: Reduces parsing overhead (40-50% of total time)
- Spec 133: Reduces resolution overhead (10-15% of remaining time)

**Combined Impact**:
- Spec 132 alone: 40-50% speedup
- Spec 133 alone: 10-15% speedup
- Both together: ~50-60% total speedup (compounding improvements)

**Implementation Order**:
1. Implement Spec 132 first (larger impact)
2. Measure baseline with new parsing approach
3. Implement Spec 133 on top of Spec 132
4. Measure combined speedup

**No direct dependencies**: Can be implemented independently, but testing is easier if done sequentially.
