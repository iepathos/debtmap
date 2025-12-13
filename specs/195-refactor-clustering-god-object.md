---
number: 195
title: Refactor Clustering God Object
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-13
---

# Specification 195: Refactor Clustering God Object

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The file `src/organization/behavioral_decomposition/clustering.rs` has been identified as a "God Object" by debtmap's self-analysis:

**Metrics:**
- **Lines of Code**: 1001 (far exceeds 200-line guideline)
- **Functions**: 19
- **Responsibilities**: 6 distinct responsibilities
- **Accumulated Cyclomatic Complexity**: 148
- **Accumulated Cognitive Complexity**: 286 → 143 (dampened)
- **Maximum Nesting Depth**: 5 (should be ≤2)
- **Test Coverage**: 60.1% (below 85% target)
- **Debt Score**: 86.2 (critical)

**Stillwater Philosophy Violations:**

1. **Pure Core Violation** - I/O (`eprintln!`) mixed with pure clustering logic:
   ```rust
   // Line 520-529: Side effect embedded in pure function
   if crate::progress::ProgressManager::global()
       .map(|pm| pm.verbosity() >= 2)
       .unwrap_or(false)
   {
       eprintln!("WARNING: {} methods were not clustered...", ...);
   }
   ```

2. **Function Size Violations** - Multiple functions exceed 20 lines:
   - `apply_community_detection`: 115 lines
   - `apply_hybrid_clustering`: 125 lines
   - `ensure_all_methods_clustered`: 81 lines
   - `merge_tiny_clusters`: 84 lines
   - `build_method_call_adjacency_matrix_with_functions`: 65 lines

3. **Mixed Abstraction Levels** - Single file contains:
   - Low-level visitor pattern (`MethodCallVisitor`)
   - Mid-level algorithms (community detection)
   - High-level pipelines (production clustering)

4. **Nesting Depth** - 5 levels in `apply_hybrid_clustering` subcluster refinement

**Six Responsibilities Identified:**

| # | Responsibility | Lines | Functions | Description |
|---|----------------|-------|-----------|-------------|
| 1 | Call Graph Building | 22-158 | 3 | Adjacency matrix construction, visitor pattern |
| 2 | Community Detection | 160-286 | 3 | Louvain-style clustering algorithm |
| 3 | Hybrid Clustering | 288-423 | 1 | Name-based + call-graph combination |
| 4 | Production Pipeline | 425-581 | 3 | Orchestration, safety checks, deduplication |
| 5 | Cluster Refinement | 583-795 | 5 | Subdivide, merge, verb patterns |
| 6 | Pattern Detection | 835-922 | 5 | IO boundary, query, matching, lookup detection |

## Objective

Refactor `clustering.rs` into 5 focused modules following the Stillwater philosophy:

1. **Separate concerns** into cohesive modules (one responsibility per module)
2. **Extract pure functions** from I/O operations
3. **Reduce function sizes** to under 20 lines each
4. **Decrease nesting depth** to maximum 2 levels
5. **Improve testability** by isolating pure clustering logic
6. **Increase coverage** to 85%+ through better unit testing

The refactored structure will be:

```
src/organization/behavioral_decomposition/clustering/
├── mod.rs              # Re-exports, public API
├── call_graph.rs       # Responsibility 1: Adjacency matrix building
├── community.rs        # Responsibility 2: Community detection algorithm
├── hybrid.rs           # Responsibility 3: Hybrid clustering
├── pipeline.rs         # Responsibility 4: Production pipeline orchestration
└── refinement.rs       # Responsibility 5+6: Cluster refinement & patterns
```

## Requirements

### Functional Requirements

1. **Module Separation**
   - Split 1001-line file into 5 modules of ~150-200 lines each
   - Each module has single responsibility
   - Clear module boundaries with minimal cross-dependencies
   - Public API remains backward compatible

2. **Pure Function Extraction**
   - Extract I/O (logging) to boundary functions
   - All clustering algorithms become pure functions
   - Return warnings as data, not side effects:
     ```rust
     pub struct ClusteringResult {
         pub clusters: Vec<MethodCluster>,
         pub warnings: Vec<ClusteringWarning>,
     }
     ```

3. **Function Size Reduction**
   - All functions under 20 lines (target: 5-10 lines)
   - Long functions decomposed via composition
   - Complex conditionals extracted to predicates

4. **Nesting Reduction**
   - Maximum nesting depth: 2 levels
   - Use early returns and helper functions
   - Extract nested loops to iterator chains

5. **Preserve Behavior**
   - All existing clustering behavior maintained
   - Same outputs for same inputs
   - Backward compatible public API

### Non-Functional Requirements

1. **Testability**
   - Pure functions unit tested without mocks
   - Each module independently testable
   - Coverage target: 85%+

2. **Performance**
   - No performance regression
   - Same or better clustering speed
   - Efficient memory usage maintained

3. **Maintainability**
   - Clear module purposes
   - Consistent patterns across modules
   - Easy to extend with new clustering strategies

## Acceptance Criteria

- [ ] Clustering split into 5 modules in `clustering/` directory
- [ ] All functions under 20 lines
- [ ] Maximum nesting depth ≤ 2
- [ ] I/O extracted to boundary (no `eprintln!` in pure functions)
- [ ] Warnings returned as data (`ClusteringResult` struct)
- [ ] Cyclomatic complexity < 5 for all functions
- [ ] Test coverage ≥ 85%
- [ ] All existing tests pass
- [ ] No clippy warnings
- [ ] `just test` passes
- [ ] Performance benchmark shows no regression
- [ ] Public API unchanged (backward compatible)

## Technical Details

### Implementation Approach

**Phase 1: Create Module Structure**

Create the new module directory and files:

```rust
// src/organization/behavioral_decomposition/clustering/mod.rs
//! Clustering algorithms for behavioral method grouping.
//!
//! This module provides various clustering strategies:
//! - Call graph analysis for method relationship detection
//! - Community detection for cohesive grouping
//! - Hybrid clustering combining name and call patterns
//! - Production-ready pipeline with refinements

mod call_graph;
mod community;
mod hybrid;
mod pipeline;
mod refinement;

pub use call_graph::build_method_call_adjacency_matrix;
pub use call_graph::build_method_call_adjacency_matrix_with_functions;
pub use community::apply_community_detection;
pub use hybrid::apply_hybrid_clustering;
pub use pipeline::apply_production_ready_clustering;
pub use pipeline::ClusteringResult;
pub use pipeline::ClusteringWarning;
```

**Phase 2: Extract Call Graph Module (Responsibility 1)**

```rust
// src/organization/behavioral_decomposition/clustering/call_graph.rs
//! Call graph construction for method relationship analysis.

use std::collections::{HashMap, HashSet};
use syn::visit::Visit;

/// Build method call adjacency matrix from impl blocks.
pub fn build_method_call_adjacency_matrix(
    impl_blocks: &[&syn::ItemImpl],
) -> HashMap<(String, String), usize> {
    build_method_call_adjacency_matrix_with_functions(impl_blocks, &[])
}

/// Build adjacency matrix with standalone function support.
pub fn build_method_call_adjacency_matrix_with_functions(
    impl_blocks: &[&syn::ItemImpl],
    standalone_functions: &[&syn::ItemFn],
) -> HashMap<(String, String), usize> {
    let all_names = collect_all_function_names(impl_blocks, standalone_functions);
    let mut matrix = HashMap::new();

    process_impl_methods(impl_blocks, &all_names, &mut matrix);
    process_standalone_functions(standalone_functions, &all_names, &mut matrix);

    matrix
}

// PURE HELPER FUNCTIONS (each under 10 lines)

fn collect_all_function_names(
    impl_blocks: &[&syn::ItemImpl],
    standalone_functions: &[&syn::ItemFn],
) -> HashSet<String> {
    let impl_names = impl_blocks.iter()
        .flat_map(|b| b.items.iter())
        .filter_map(extract_method_name);

    let standalone_names = standalone_functions.iter()
        .map(|f| f.sig.ident.to_string());

    impl_names.chain(standalone_names).collect()
}

fn extract_method_name(item: &syn::ImplItem) -> Option<String> {
    match item {
        syn::ImplItem::Fn(method) => Some(method.sig.ident.to_string()),
        _ => None,
    }
}

// ... more focused helper functions
```

**Phase 3: Extract Community Detection Module (Responsibility 2)**

```rust
// src/organization/behavioral_decomposition/clustering/community.rs
//! Community detection for method clustering.

use std::collections::HashMap;
use super::types::MethodCluster;

const MAX_METHODS_FOR_CLUSTERING: usize = 200;
const MAX_ITERATIONS: usize = 10;
const MIN_CLUSTER_SIZE: usize = 3;
const MIN_COHESION_SCORE: f64 = 0.2;

/// Apply Louvain-style community detection algorithm.
///
/// Pure function - no I/O, deterministic results.
pub fn apply_community_detection(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    if should_skip_clustering(methods, adjacency) {
        return Vec::new();
    }

    let (clusters, method_to_cluster) = initialize_clusters(methods);
    let refined = iteratively_improve(clusters, method_to_cluster, adjacency, methods);

    filter_and_convert_clusters(refined, adjacency, &method_to_cluster)
}

// PURE HELPER FUNCTIONS

fn should_skip_clustering(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> bool {
    adjacency.is_empty() || methods.len() > MAX_METHODS_FOR_CLUSTERING
}

fn initialize_clusters(methods: &[String]) -> (HashMap<usize, Vec<String>>, HashMap<String, usize>) {
    let clusters = methods.iter()
        .enumerate()
        .map(|(i, m)| (i, vec![m.clone()]))
        .collect();

    let method_to_cluster = methods.iter()
        .enumerate()
        .map(|(i, m)| (m.clone(), i))
        .collect();

    (clusters, method_to_cluster)
}

// ... more focused pure functions
```

**Phase 4: Extract Pure I/O Separation in Pipeline**

```rust
// src/organization/behavioral_decomposition/clustering/pipeline.rs
//! Production-ready clustering pipeline.

/// Warning generated during clustering.
#[derive(Debug, Clone)]
pub enum ClusteringWarning {
    UnclusteredMethods {
        count: usize,
        sample: Vec<String>,
    },
}

/// Result of clustering with warnings as data.
#[derive(Debug)]
pub struct ClusteringResult {
    pub clusters: Vec<MethodCluster>,
    pub warnings: Vec<ClusteringWarning>,
}

/// Apply production-ready clustering pipeline.
///
/// Pure function - returns warnings as data, not side effects.
pub fn apply_production_ready_clustering(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> ClusteringResult {
    let production_methods = filter_test_methods(methods);

    if production_methods.is_empty() {
        return ClusteringResult::empty();
    }

    production_methods
        .pipe(|m| apply_hybrid_clustering(&m, adjacency))
        .pipe(|c| subdivide_oversized_clusters(c, adjacency))
        .pipe(merge_tiny_clusters)
        .pipe(apply_rust_patterns)
        .pipe(merge_duplicate_categories)
        .pipe(|c| ensure_all_methods_with_warnings(c, &production_methods, adjacency))
}

// Previously: eprintln! side effect
// Now: returns warning as data
fn ensure_all_methods_with_warnings(
    mut clusters: Vec<MethodCluster>,
    all_methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> ClusteringResult {
    let missing = find_missing_methods(&clusters, all_methods);
    let warnings = if !missing.is_empty() {
        vec![ClusteringWarning::UnclusteredMethods {
            count: missing.len(),
            sample: missing.iter().take(5).cloned().collect(),
        }]
    } else {
        vec![]
    };

    clusters = recover_missing_methods(clusters, missing, adjacency);

    ClusteringResult { clusters, warnings }
}
```

**Phase 5: Decompose Long Functions via Composition**

Example: `apply_hybrid_clustering` (currently 125 lines) becomes:

```rust
// src/organization/behavioral_decomposition/clustering/hybrid.rs

pub fn apply_hybrid_clustering(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    let name_clusters = cluster_methods_by_behavior(methods);

    if name_clusters.is_empty() {
        return apply_community_detection(methods, adjacency);
    }

    name_clusters.into_iter()
        .flat_map(|(category, methods)| refine_category(category, methods, adjacency))
        .collect()
}

fn refine_category(
    category: BehaviorCategory,
    methods: Vec<String>,
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    if methods.len() <= 5 {
        return vec![create_small_cluster(category, methods, adjacency)];
    }

    refine_large_category(category, methods, adjacency)
}

fn create_small_cluster(
    category: BehaviorCategory,
    methods: Vec<String>,
    adjacency: &HashMap<(String, String), usize>,
) -> MethodCluster {
    let (internal, external) = calculate_cohesion(&methods, adjacency);

    MethodCluster {
        category,
        methods,
        fields_accessed: vec![],
        internal_calls: internal,
        external_calls: external,
        cohesion_score: compute_cohesion_score(internal, external),
    }
}

fn refine_large_category(
    category: BehaviorCategory,
    methods: Vec<String>,
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    let sub_clusters = apply_community_detection(&methods, adjacency);

    if sub_clusters.is_empty() {
        return vec![create_small_cluster(category, methods, adjacency)];
    }

    let refined = refine_subclusters(sub_clusters, &category);
    let lost = find_lost_methods(&refined, &methods);

    recover_lost_into_clusters(refined, lost, category, adjacency)
}

// Each function: 5-15 lines, single responsibility
```

### Module Size Estimates

| Module | Estimated Lines | Responsibility |
|--------|-----------------|----------------|
| `mod.rs` | ~30 | Re-exports, documentation |
| `call_graph.rs` | ~140 | Adjacency matrix, visitor |
| `community.rs` | ~130 | Detection algorithm, modularity |
| `hybrid.rs` | ~130 | Hybrid clustering |
| `pipeline.rs` | ~140 | Orchestration, warnings |
| `refinement.rs` | ~200 | Subdivide, merge, patterns |
| **Total** | ~770 | (reduced from 1001) |

### Architecture Changes

```
BEFORE:
src/organization/behavioral_decomposition/
├── mod.rs
├── categorization.rs
├── clustering.rs          ← 1001 lines, 6 responsibilities
└── types.rs

AFTER:
src/organization/behavioral_decomposition/
├── mod.rs
├── categorization.rs
├── clustering/            ← New module directory
│   ├── mod.rs            ← Public API
│   ├── call_graph.rs     ← Responsibility 1
│   ├── community.rs      ← Responsibility 2
│   ├── hybrid.rs         ← Responsibility 3
│   ├── pipeline.rs       ← Responsibility 4
│   └── refinement.rs     ← Responsibility 5+6
└── types.rs
```

### Data Structures

New types for clean I/O separation:

```rust
/// Warning generated during clustering process.
#[derive(Debug, Clone, PartialEq)]
pub enum ClusteringWarning {
    /// Some methods could not be assigned to cohesive clusters.
    UnclusteredMethods {
        count: usize,
        sample: Vec<String>,
    },
    /// Cluster was force-merged due to size constraints.
    ForceMerged {
        from_category: String,
        into_category: String,
        method_count: usize,
    },
}

/// Result of clustering operation with metadata.
#[derive(Debug)]
pub struct ClusteringResult {
    /// The resulting method clusters.
    pub clusters: Vec<MethodCluster>,
    /// Warnings generated during clustering.
    pub warnings: Vec<ClusteringWarning>,
}

impl ClusteringResult {
    pub fn empty() -> Self {
        Self { clusters: vec![], warnings: vec![] }
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}
```

### APIs and Interfaces

**Public API (unchanged for backward compatibility):**

```rust
// These signatures remain the same
pub fn build_method_call_adjacency_matrix(
    impl_blocks: &[&syn::ItemImpl],
) -> HashMap<(String, String), usize>;

pub fn build_method_call_adjacency_matrix_with_functions(
    impl_blocks: &[&syn::ItemImpl],
    standalone_functions: &[&syn::ItemFn],
) -> HashMap<(String, String), usize>;

pub fn apply_community_detection(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster>;

pub fn apply_hybrid_clustering(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster>;

// CHANGED: Now returns ClusteringResult instead of Vec<MethodCluster>
pub fn apply_production_ready_clustering(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> ClusteringResult;  // New return type

// For backward compatibility, add:
pub fn apply_production_ready_clustering_simple(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    apply_production_ready_clustering(methods, adjacency).clusters
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/organization/behavioral_decomposition/clustering.rs` (deleted)
  - `src/organization/behavioral_decomposition/clustering/` (new directory)
  - `src/organization/behavioral_decomposition/mod.rs` (update imports)
  - Any code calling clustering functions (update for `ClusteringResult`)
- **External Dependencies**: None

## Testing Strategy

### Unit Tests (Pure Functions)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // call_graph tests
    #[test]
    fn test_collect_function_names_empty() {
        let names = collect_all_function_names(&[], &[]);
        assert!(names.is_empty());
    }

    #[test]
    fn test_adjacency_matrix_simple_call() {
        // Create impl block with self.other() call
        let impl_blocks = create_test_impl_with_call("method_a", "method_b");
        let matrix = build_method_call_adjacency_matrix(&impl_blocks);

        assert_eq!(matrix.get(&("method_a".into(), "method_b".into())), Some(&1));
    }

    // community tests
    #[test]
    fn test_should_skip_empty_adjacency() {
        assert!(should_skip_clustering(&["a".into()], &HashMap::new()));
    }

    #[test]
    fn test_modularity_calculation() {
        let adjacency = create_test_adjacency();
        let cluster = vec!["a".into(), "b".into()];
        let all = vec!["a".into(), "b".into(), "c".into()];

        let modularity = calculate_method_modularity("a", &cluster, &adjacency, &all);
        assert!(modularity >= 0.0 && modularity <= 1.0);
    }

    // pipeline tests
    #[test]
    fn test_clustering_result_empty() {
        let result = ClusteringResult::empty();
        assert!(result.clusters.is_empty());
        assert!(!result.has_warnings());
    }

    #[test]
    fn test_unclustered_warning_generated() {
        // Setup: methods that can't be clustered
        let methods = vec!["orphan1".into(), "orphan2".into()];
        let adjacency = HashMap::new(); // No connections

        let result = apply_production_ready_clustering(&methods, &adjacency);

        assert!(result.warnings.iter().any(|w| matches!(w,
            ClusteringWarning::UnclusteredMethods { .. }
        )));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_full_clustering_pipeline() {
    let methods = vec![
        "parse_header".into(),
        "parse_body".into(),
        "validate_data".into(),
        "save_result".into(),
    ];

    let adjacency = create_parse_method_adjacency(&methods);
    let result = apply_production_ready_clustering(&methods, &adjacency);

    // Verify: parse methods grouped together
    let parse_cluster = result.clusters.iter()
        .find(|c| c.methods.contains(&"parse_header".to_string()));

    assert!(parse_cluster.is_some());
    assert!(parse_cluster.unwrap().methods.contains(&"parse_body".to_string()));
}
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_no_method_loss(
        methods in prop::collection::vec("[a-z]+", 1..50)
    ) {
        let adjacency = HashMap::new();
        let result = apply_production_ready_clustering(&methods, &adjacency);

        let clustered: HashSet<_> = result.clusters
            .iter()
            .flat_map(|c| &c.methods)
            .cloned()
            .collect();

        // Every non-test method must be in some cluster
        let non_test: HashSet<_> = methods.iter()
            .filter(|m| !is_test_method(m))
            .cloned()
            .collect();

        prop_assert!(non_test.is_subset(&clustered));
    }

    #[test]
    fn test_cohesion_bounds(
        methods in prop::collection::vec("[a-z]+", 3..20)
    ) {
        let adjacency = create_random_adjacency(&methods);
        let result = apply_production_ready_clustering(&methods, &adjacency);

        for cluster in &result.clusters {
            prop_assert!(cluster.cohesion_score >= 0.0);
            prop_assert!(cluster.cohesion_score <= 1.0);
        }
    }
}
```

### Performance Tests

```rust
#[test]
fn test_clustering_performance() {
    let methods: Vec<String> = (0..100)
        .map(|i| format!("method_{}", i))
        .collect();

    let adjacency = create_dense_adjacency(&methods);

    let start = std::time::Instant::now();
    let _ = apply_production_ready_clustering(&methods, &adjacency);
    let duration = start.elapsed();

    // Should complete in < 100ms for 100 methods
    assert!(duration < std::time::Duration::from_millis(100));
}
```

## Documentation Requirements

### Code Documentation

Each module and public function documented:

```rust
//! Call graph construction for method relationship analysis.
//!
//! This module builds adjacency matrices representing method call
//! relationships within impl blocks and standalone functions.
//!
//! # Pure Function Properties
//!
//! All functions in this module are pure:
//! - Deterministic output for same input
//! - No side effects (no I/O, no logging)
//! - Thread-safe
//!
//! # Example
//!
//! ```
//! use debtmap::organization::behavioral_decomposition::clustering::call_graph;
//!
//! let adjacency = call_graph::build_method_call_adjacency_matrix(&impl_blocks);
//! for ((from, to), count) in &adjacency {
//!     println!("{} calls {} ({} times)", from, to, count);
//! }
//! ```

/// Builds an adjacency matrix from method call patterns.
///
/// Analyzes method bodies to find `self.method()` calls and builds
/// a matrix of call relationships.
///
/// # Arguments
///
/// * `impl_blocks` - References to syn ItemImpl blocks to analyze
///
/// # Returns
///
/// HashMap where key is (caller, callee) and value is call count
///
/// # Pure Function
///
/// This function is pure - no I/O, deterministic results.
pub fn build_method_call_adjacency_matrix(
    impl_blocks: &[&syn::ItemImpl],
) -> HashMap<(String, String), usize> {
    // ...
}
```

### User Documentation

Update ARCHITECTURE.md:

```markdown
## Behavioral Decomposition Clustering

The clustering subsystem groups methods by behavior for split recommendations.

### Module Structure

```
clustering/
├── mod.rs          # Public API
├── call_graph.rs   # Build method call adjacency matrix
├── community.rs    # Community detection algorithm
├── hybrid.rs       # Hybrid name + call-graph clustering
├── pipeline.rs     # Production pipeline with warnings
└── refinement.rs   # Cluster refinement and patterns
```

### Design Principles

1. **Pure Functions**: All clustering algorithms are pure
2. **Warnings as Data**: No logging in core; return `ClusteringResult`
3. **Single Responsibility**: Each module handles one aspect
4. **Composable Pipeline**: Production clustering is composed of steps

### Usage

```rust
use debtmap::organization::behavioral_decomposition::clustering;

// Build call graph
let adjacency = clustering::build_method_call_adjacency_matrix(&impl_blocks);

// Apply clustering
let result = clustering::apply_production_ready_clustering(&methods, &adjacency);

// Handle warnings at I/O boundary
for warning in &result.warnings {
    eprintln!("Warning: {:?}", warning);
}

// Use clusters
for cluster in result.clusters {
    println!("Cluster {}: {} methods", cluster.category.display_name(), cluster.methods.len());
}
```
```

### Architecture Updates

Add clustering module to ARCHITECTURE.md module graph.

## Implementation Notes

### Refactoring Workflow

1. **Create module structure** - New directory and mod.rs
2. **Move responsibility 1** - Call graph to call_graph.rs
3. **Move responsibility 2** - Community detection to community.rs
4. **Move responsibility 3** - Hybrid clustering to hybrid.rs
5. **Move responsibilities 4** - Pipeline to pipeline.rs
6. **Move responsibilities 5+6** - Refinement to refinement.rs
7. **Delete original file** - Remove clustering.rs
8. **Update imports** - Fix all references
9. **Add new tests** - Cover extracted functions
10. **Verify behavior** - Run full test suite

### Common Pitfalls

1. **Circular imports** - Ensure clean dependency graph between modules
2. **Breaking API** - Maintain backward compatibility where possible
3. **Test coverage gaps** - Newly extracted functions need tests
4. **Performance regression** - Profile before and after

### Tools

```bash
# Find usages before refactoring
rg "use.*clustering::" src/

# Check complexity after
cargo clippy -- -W clippy::cognitive_complexity

# Run specific tests
cargo test --package debtmap --lib organization::behavioral_decomposition::clustering

# Coverage check
cargo tarpaulin --packages debtmap --out Html
```

## Migration and Compatibility

### Breaking Changes

**Minor API Change:**
- `apply_production_ready_clustering` returns `ClusteringResult` instead of `Vec<MethodCluster>`
- Add `apply_production_ready_clustering_simple` for backward compatibility

### Migration Steps

1. Update imports from `clustering::*` to `clustering::module::*`
2. Handle `ClusteringResult` instead of `Vec<MethodCluster>` where warnings are desired
3. Or use `_simple` variant for unchanged behavior

### Deprecation

```rust
#[deprecated(
    since = "0.8.0",
    note = "Use apply_production_ready_clustering().clusters instead"
)]
pub fn apply_production_ready_clustering_simple(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    apply_production_ready_clustering(methods, adjacency).clusters
}
```

## Success Metrics

After implementation:

- [ ] File size: 1001 → ~770 lines total (5 files)
- [ ] Max function size: 125 → 20 lines
- [ ] Max nesting: 5 → 2 levels
- [ ] Cyclomatic complexity: <5 per function
- [ ] Coverage: 60% → 85%+
- [ ] Debt score: 86.2 → <20 (target)

## References

- **Stillwater Philosophy** - Pure core, imperative shell
- **Spec 187** - Extract Pure Functions from Analyzers (similar pattern)
- **CLAUDE.md** - Function design guidelines (max 20 lines)
- **Debtmap self-analysis** - God Object detection results
