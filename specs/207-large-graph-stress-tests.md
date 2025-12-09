---
number: 207
title: Large Graph Stress Tests for Call Graph Operations
category: testing
priority: high
status: draft
dependencies: [206]
created: 2025-12-09
---

# Specification 207: Large Graph Stress Tests for Call Graph Operations

**Category**: testing
**Priority**: high
**Status**: draft
**Dependencies**: Spec 206 (Convert Recursive Graph Traversal to Iterative)

## Context

### Current Test Coverage Gaps

Existing tests for call graph operations only use tiny graphs (2-3 nodes):

| Test | Nodes | Coverage |
|------|-------|----------|
| `test_is_recursive_direct` | 1 | Direct self-call only |
| `test_is_recursive_indirect` | 2 | Two-node cycle only |
| `test_topological_sort_simple` | 3 | Linear chain only |

These tests **cannot detect stack overflow** because they never stress the recursive depth.

### Stress Tests Exist But Don't Cover Graph Algorithms

`tests/stress_test_large_projects.rs` has good stress tests but:
- All marked `#[ignore]` (don't run in CI)
- Test parallel analysis phases, not graph algorithms
- `stress_test_highly_connected_graph` creates 10k nodes with 200k edges but **never calls `is_recursive()` or `topological_sort()`**

### What Causes Stack Overflow

Recursive graph traversal functions overflow when:
1. **Deep linear chains**: A -> B -> C -> ... -> Z (depth = N)
2. **Large SCCs**: Strongly connected components with 100+ nodes
3. **Star patterns with recursive edges**: Hub calling many leaves that call back

The DSM module already fixed this for `dfs_finish`/`dfs_collect`. Spec 206 fixes `has_cycle_dfs`/`topo_sort_dfs`. This spec adds tests to **prevent regression**.

## Objective

Add stress tests that specifically exercise graph traversal algorithms at scale, ensuring:

1. **Detection of stack overflow** - Tests that would fail before Spec 206 fix
2. **Regression prevention** - Run in CI to catch future issues
3. **Performance validation** - Ensure O(N) behavior, not O(N^2)

**Success Metric**: Tests catch stack overflow if recursive implementations are reintroduced.

## Requirements

### Functional Requirements

1. **FR1**: Test `is_recursive()` on graphs with 10,000+ nodes
2. **FR2**: Test `topological_sort()` on graphs with 10,000+ nodes
3. **FR3**: Test deep linear chains (approaching default stack limit)
4. **FR4**: Test large strongly connected components
5. **FR5**: At least one test runs in CI (not `#[ignore]`)

### Non-Functional Requirements

1. **NFR1**: Fast test execution (< 5 seconds for CI test)
2. **NFR2**: Deterministic results (no flaky tests)
3. **NFR3**: Clear failure messages indicating stack overflow

## Acceptance Criteria

- [ ] AC1: `test_is_recursive_large_linear_chain` - 5,000 node linear chain
- [ ] AC2: `test_is_recursive_large_scc` - 1,000 node fully connected component
- [ ] AC3: `test_topological_sort_large_graph` - 10,000 node DAG
- [ ] AC4: `test_topological_sort_deep_chain` - 5,000 depth chain
- [ ] AC5: At least one test runs in CI without `#[ignore]`
- [ ] AC6: Tests pass with iterative implementation (Spec 206)
- [ ] AC7: Tests document expected behavior for edge cases

## Technical Details

### Implementation Approach

Create new test file: `tests/call_graph_stress_test.rs`

#### Test 1: Large Linear Chain for `is_recursive`

```rust
/// Test is_recursive on a 5,000 node linear chain
/// This would cause stack overflow with recursive implementation
#[test]
fn test_is_recursive_large_linear_chain() {
    let mut graph = CallGraph::new();
    let chain_length = 5_000;

    // Create chain: A -> B -> C -> ... (no cycle)
    let nodes: Vec<FunctionId> = (0..chain_length)
        .map(|i| FunctionId::new(
            PathBuf::from("test.rs"),
            format!("func_{}", i),
            i * 10,
        ))
        .collect();

    for node in &nodes {
        graph.add_function(node.clone(), false, false, 1, 10);
    }

    for i in 0..chain_length - 1 {
        graph.add_call_parts(
            nodes[i].clone(),
            nodes[i + 1].clone(),
            CallType::Direct,
        );
    }

    // No cycles, so none should be recursive
    // This traverses entire graph depth - would overflow with recursion
    assert!(!graph.is_recursive(&nodes[0]));
    assert!(!graph.is_recursive(&nodes[chain_length / 2]));
}
```

#### Test 2: Large SCC for `is_recursive`

```rust
/// Test is_recursive on a 1,000 node fully connected component
/// Every node can reach every other node
#[test]
fn test_is_recursive_large_scc() {
    let mut graph = CallGraph::new();
    let scc_size = 1_000;

    // Create ring: A -> B -> C -> ... -> A (cycle)
    let nodes: Vec<FunctionId> = (0..scc_size)
        .map(|i| FunctionId::new(
            PathBuf::from("test.rs"),
            format!("scc_func_{}", i),
            i * 10,
        ))
        .collect();

    for node in &nodes {
        graph.add_function(node.clone(), false, false, 1, 10);
    }

    // Create ring
    for i in 0..scc_size {
        graph.add_call_parts(
            nodes[i].clone(),
            nodes[(i + 1) % scc_size].clone(),
            CallType::Direct,
        );
    }

    // All nodes are in cycle
    assert!(graph.is_recursive(&nodes[0]));
    assert!(graph.is_recursive(&nodes[scc_size / 2]));
}
```

#### Test 3: Large DAG for `topological_sort`

```rust
/// Test topological_sort on a 10,000 node DAG
/// Validates O(N) performance without stack overflow
#[test]
fn test_topological_sort_large_dag() {
    let mut graph = CallGraph::new();
    let num_nodes = 10_000;

    // Create nodes
    let nodes: Vec<FunctionId> = (0..num_nodes)
        .map(|i| FunctionId::new(
            PathBuf::from(format!("module_{}/file.rs", i / 100)),
            format!("func_{}", i),
            (i % 100) * 10,
        ))
        .collect();

    for node in &nodes {
        graph.add_function(node.clone(), i % 10 == 0, false, 1, 10);
    }

    // Create DAG edges: each node calls next 3 nodes (forward only)
    for i in 0..num_nodes {
        for j in 1..=3 {
            if i + j < num_nodes {
                graph.add_call_parts(
                    nodes[i].clone(),
                    nodes[i + j].clone(),
                    CallType::Direct,
                );
            }
        }
    }

    let start = std::time::Instant::now();
    let result = graph.topological_sort();
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    let sorted = result.unwrap();
    assert_eq!(sorted.len(), num_nodes);

    // Should complete quickly (< 1 second)
    assert!(elapsed.as_secs() < 1, "Topo sort took too long: {:?}", elapsed);
}
```

#### Test 4: Deep Chain for `topological_sort`

```rust
/// Test topological_sort on a 5,000 depth chain
/// Worst case for stack depth
#[test]
fn test_topological_sort_deep_chain() {
    let mut graph = CallGraph::new();
    let depth = 5_000;

    let nodes: Vec<FunctionId> = (0..depth)
        .map(|i| FunctionId::new(
            PathBuf::from("deep.rs"),
            format!("level_{}", i),
            i * 5,
        ))
        .collect();

    for node in &nodes {
        graph.add_function(node.clone(), false, false, 1, 5);
    }

    // Linear chain: 0 -> 1 -> 2 -> ... -> N
    for i in 0..depth - 1 {
        graph.add_call_parts(
            nodes[i].clone(),
            nodes[i + 1].clone(),
            CallType::Direct,
        );
    }

    let result = graph.topological_sort();
    assert!(result.is_ok());

    let sorted = result.unwrap();

    // Verify order: leaves first (depth-1), then depth-2, ..., then 0
    // Last node in chain should come first in topo order
    assert_eq!(sorted.first().unwrap().name, format!("level_{}", depth - 1));
    assert_eq!(sorted.last().unwrap().name, "level_0");
}
```

#### CI Test: Medium-Sized Validation

```rust
/// Quick test that runs in CI to catch regressions
/// Smaller than stress tests but large enough to detect naive recursion
#[test]
fn test_graph_operations_ci_validation() {
    let mut graph = CallGraph::new();
    let size = 500; // Large enough to stress, small enough for CI

    // Create linear chain with cycle at end
    let nodes: Vec<FunctionId> = (0..size)
        .map(|i| FunctionId::new(
            PathBuf::from("ci_test.rs"),
            format!("f{}", i),
            i,
        ))
        .collect();

    for node in &nodes {
        graph.add_function(node.clone(), false, false, 1, 5);
    }

    // Linear chain
    for i in 0..size - 1 {
        graph.add_call_parts(nodes[i].clone(), nodes[i + 1].clone(), CallType::Direct);
    }

    // Add cycle at the end
    graph.add_call_parts(nodes[size - 1].clone(), nodes[size - 10].clone(), CallType::Direct);

    // Test operations complete without overflow
    assert!(graph.is_recursive(&nodes[size - 5])); // In cycle
    assert!(!graph.is_recursive(&nodes[0])); // Before cycle starts

    // Topo sort should handle cycle gracefully
    let _ = graph.topological_sort();
}
```

### Test Organization

```
tests/
├── call_graph_stress_test.rs  # New file with stress tests
└── stress_test_large_projects.rs  # Existing parallel analysis tests
```

### Architecture Changes

None - adds new test file only.

### Data Structures

Test helpers for graph generation:

```rust
/// Generate a linear chain of N nodes
fn create_linear_chain(n: usize) -> (CallGraph, Vec<FunctionId>)

/// Generate a ring (cycle) of N nodes
fn create_ring(n: usize) -> (CallGraph, Vec<FunctionId>)

/// Generate a DAG with N nodes and fan-out F
fn create_dag(n: usize, fan_out: usize) -> (CallGraph, Vec<FunctionId>)
```

### APIs and Interfaces

No public API changes.

## Dependencies

- **Prerequisites**: Spec 206 (iterative implementations must exist)
- **Affected Components**: `tests/` directory only
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

The stress tests themselves are the unit tests for graph operations at scale.

### Integration Tests

Existing integration tests continue to run.

### Performance Tests

Include timing assertions in stress tests to catch O(N^2) regressions:

```rust
assert!(elapsed.as_millis() < 1000, "Operation too slow: {:?}", elapsed);
```

### User Acceptance

Tests pass in CI:
```bash
cargo test call_graph_stress
cargo test test_graph_operations_ci_validation
```

## Documentation Requirements

- **Code Documentation**: Doc comments explaining test purpose
- **User Documentation**: None required
- **Architecture Updates**: None required

## Implementation Notes

### Why These Specific Sizes?

| Test | Size | Rationale |
|------|------|-----------|
| Linear chain | 5,000 | ~5MB stack with 1KB frames, within 8MB default |
| SCC ring | 1,000 | Large enough to stress, not too slow |
| DAG | 10,000 | Realistic large project size |
| CI test | 500 | Fast enough for every CI run |

### Stack Size Calculation

Default stack: 8MB
Typical recursive frame: ~1KB (function pointer, visited set ref, rec_stack ref)
Max safe depth: 8,000,000 / 1,000 = 8,000 nodes

We test at 5,000 to have safety margin.

### Marking Tests `#[ignore]`

Large stress tests should be `#[ignore]` to not slow CI:
```rust
#[test]
#[ignore = "stress test - run with --ignored"]
fn test_topological_sort_large_dag() { ... }
```

But `test_graph_operations_ci_validation` runs always.

## Migration and Compatibility

No migration required. Adds new tests only.

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Flaky tests | Medium | Use deterministic graph generation |
| Slow CI | Low | Keep CI test small, use #[ignore] for large tests |
| False positives | Low | Test behavior, not just completion |

## Related Specifications

- **Spec 206**: Convert Recursive Graph Traversal to Iterative (prerequisite)
