---
number: 206
title: Convert Recursive Graph Traversal to Iterative
category: optimization
priority: critical
status: draft
dependencies: []
created: 2025-12-09
---

# Specification 206: Convert Recursive Graph Traversal to Iterative

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Debtmap experiences stack overflow errors when analyzing large codebases. The error occurs at approximately 83% progress during "stage 6/6" (debt scoring):

```
thread 'main' has overflowed its stack
fatal runtime error: stack overflow, aborting
```

Investigation identified multiple recursive DFS functions in the call graph module that can cause stack overflow on large graphs:

### Affected Functions in `src/priority/call_graph/graph_operations.rs`

1. **`has_cycle_dfs`** (lines 385-406) - Recursive cycle detection for `is_recursive()`
2. **`topo_sort_dfs`** (lines 425-441) - Recursive topological sort helper

### Previously Fixed (in `src/analysis/dsm.rs`)

3. **`dfs_finish`** - Already converted to iterative (Kosaraju's algorithm)
4. **`dfs_collect`** - Already converted to iterative (SCC collection)

### Root Cause

Recursive DFS on a call graph with N nodes uses O(N) stack frames. With default 8MB stack size and ~1KB per frame, graphs exceeding ~8,000 nodes in a single call chain can overflow.

Real codebases like debtmap itself have:
- ~3,000+ functions
- Complex interdependencies creating deep call chains
- Potential for linear chains in module hierarchies

## Objective

Convert all recursive graph traversal functions in `graph_operations.rs` to iterative implementations using explicit stacks, eliminating stack overflow risk while preserving correctness.

**Success Metric**: Debtmap can analyze itself (`debtmap analyze .`) without stack overflow.

## Requirements

### Functional Requirements

1. **FR1**: `has_cycle_dfs` must be converted to iterative implementation
2. **FR2**: `topo_sort_dfs` must be converted to iterative implementation
3. **FR3**: All existing tests must continue to pass
4. **FR4**: Behavioral equivalence - same results for all inputs

### Non-Functional Requirements

1. **NFR1**: No performance regression on small graphs (< 1000 nodes)
2. **NFR2**: Memory usage should be proportional to graph size, not call depth
3. **NFR3**: Code clarity - iterative versions should be well-documented

## Acceptance Criteria

- [ ] AC1: `has_cycle_dfs` converted to iterative with explicit stack
- [ ] AC2: `topo_sort_dfs` converted to iterative with explicit stack
- [ ] AC3: All 12 existing tests in `graph_operations.rs` pass
- [ ] AC4: `cargo test` passes for entire project
- [ ] AC5: Self-analysis works: `debtmap analyze . --context` completes without stack overflow
- [ ] AC6: No clippy warnings introduced
- [ ] AC7: Code is formatted with `cargo fmt`

## Technical Details

### Implementation Approach

#### Converting `has_cycle_dfs` to Iterative

The current recursive implementation:

```rust
fn has_cycle_dfs(
    &self,
    func_id: &FunctionId,
    visited: &mut HashSet<FunctionId>,
    rec_stack: &mut HashSet<FunctionId>,
) -> bool {
    if !visited.contains(func_id) {
        visited.insert(func_id.clone());
        rec_stack.insert(func_id.clone());

        for callee in self.get_callees(func_id) {
            if rec_stack.contains(&callee) {
                return true;  // Cycle found
            }
            if !visited.contains(&callee) && self.has_cycle_dfs(&callee, visited, rec_stack) {
                return true;
            }
        }
    }
    rec_stack.remove(func_id);
    false
}
```

**Iterative approach using state machine**:

```rust
fn has_cycle_dfs_iterative(
    &self,
    start: &FunctionId,
    visited: &mut HashSet<FunctionId>,
    rec_stack: &mut HashSet<FunctionId>,
) -> bool {
    // Stack entries: (node, neighbor_index, is_backtracking)
    let mut stack: Vec<(FunctionId, usize, bool)> = vec![(start.clone(), 0, false)];

    while let Some((node, neighbor_idx, backtracking)) = stack.pop() {
        if backtracking {
            rec_stack.remove(&node);
            continue;
        }

        if visited.contains(&node) {
            continue;
        }

        visited.insert(node.clone());
        rec_stack.insert(node.clone());

        // Schedule backtracking (remove from rec_stack) after processing children
        stack.push((node.clone(), 0, true));

        let callees: Vec<_> = self.get_callees(&node);
        for callee in callees.into_iter().rev() {
            if rec_stack.contains(&callee) {
                return true; // Cycle found
            }
            if !visited.contains(&callee) {
                stack.push((callee, 0, false));
            }
        }
    }

    false
}
```

#### Converting `topo_sort_dfs` to Iterative

The current recursive implementation:

```rust
fn topo_sort_dfs(
    &self,
    func_id: &FunctionId,
    visited: &mut HashSet<FunctionId>,
    result: &mut Vector<FunctionId>,
) -> Result<(), String> {
    visited.insert(func_id.clone());

    for callee in self.get_callees(func_id) {
        if !visited.contains(&callee) {
            self.topo_sort_dfs(&callee, visited, result)?;
        }
    }

    result.push_back(func_id.clone());
    Ok(())
}
```

**Iterative approach**:

```rust
fn topo_sort_dfs_iterative(
    &self,
    start: &FunctionId,
    visited: &mut HashSet<FunctionId>,
    result: &mut Vector<FunctionId>,
) {
    // Stack entries: (node, is_post_order)
    let mut stack: Vec<(FunctionId, bool)> = vec![(start.clone(), false)];

    while let Some((node, is_post_order)) = stack.pop() {
        if is_post_order {
            result.push_back(node);
            continue;
        }

        if visited.contains(&node) {
            continue;
        }

        visited.insert(node.clone());

        // Schedule post-order processing (add to result after children)
        stack.push((node.clone(), true));

        // Add unvisited children
        for callee in self.get_callees(&node) {
            if !visited.contains(&callee) {
                stack.push((callee, false));
            }
        }
    }
}
```

### Key Insight: Two-Phase Stack Entries

Both conversions use a common pattern:
1. **Pre-order phase**: Visit node, mark visited, schedule children
2. **Post-order phase**: Perform cleanup (remove from rec_stack) or output (add to result)

This is encoded using a boolean flag or enum in the stack entry.

### Architecture Changes

No architectural changes required. This is a pure algorithmic refactor within existing module.

### Data Structures

New stack entry types (can be local to functions):

```rust
// For cycle detection
enum CycleDfsState {
    Enter(FunctionId),
    Exit(FunctionId),
}

// For topological sort
enum TopoState {
    Visit(FunctionId),
    Finish(FunctionId),
}
```

### APIs and Interfaces

No public API changes. Internal helper functions remain private.

## Dependencies

- **Prerequisites**: None
- **Affected Components**: `src/priority/call_graph/graph_operations.rs`
- **External Dependencies**: None (uses std collections)

## Testing Strategy

### Unit Tests

Existing tests in `graph_operations.rs::tests` module:
- `test_is_recursive_direct` - Direct self-call
- `test_is_recursive_indirect` - Two-node cycle
- `test_topological_sort_simple` - Three-node linear chain

These must continue to pass.

### Integration Tests

Spec 207 will add large-graph stress tests. See that spec for details.

### Performance Tests

Verify no regression on existing stress tests:
```bash
cargo test stress_test_1000_files -- --ignored
cargo test stress_test_highly_connected_graph -- --ignored
```

### User Acceptance

```bash
# Must complete without stack overflow
debtmap analyze . --lcov target/coverage/lcov.info --context
```

## Documentation Requirements

- **Code Documentation**: Add doc comments explaining iterative approach
- **User Documentation**: None required (internal change)
- **Architecture Updates**: None required

## Implementation Notes

### Pattern: Simulating Recursion with Explicit Stack

When converting recursive DFS to iterative:

1. **Identify recursive call points** - Where function calls itself
2. **Track state between calls** - What needs to be remembered
3. **Handle pre-order vs post-order** - When to process node relative to children
4. **Use stack entries with state** - Encode whether entering or exiting node

### Common Pitfall: Forgetting Post-Order

For `has_cycle_dfs`, the `rec_stack.remove(func_id)` happens AFTER all children are processed. This must be scheduled on the stack, not done immediately.

### Performance Consideration

Vec-based stack is typically faster than VecDeque for DFS due to cache locality. Use `Vec::with_capacity(N)` when graph size is known.

## Migration and Compatibility

No migration required. This is a transparent internal optimization.

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Behavioral regression | High | Comprehensive unit tests, self-analysis validation |
| Performance regression | Medium | Benchmark before/after on large graphs |
| Code complexity increase | Low | Well-documented iterative patterns |

## Related Specifications

- **Spec 207**: Large Graph Stress Tests (validates this fix)
