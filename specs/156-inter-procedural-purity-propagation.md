---
number: 156
title: Inter-Procedural Purity Propagation
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-11-01
---

# Specification 156: Inter-Procedural Purity Propagation

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

**Current Limitation**: Debtmap analyzes each function in isolation. If function `foo()` calls function `bar()`, we cannot determine `foo()`'s purity even if `bar()` is known to be pure. This creates a **40-60% false negative rate** where pure functions that call other pure functions are incorrectly classified as impure.

**Example Problem**:
```rust
// This is pure
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// This should also be pure, but is marked impure because add() is an "unknown function call"
fn calculate_total(items: &[i32]) -> i32 {
    items.iter().map(|x| add(*x, 10)).sum()
}
```

**Current Behavior** (from `src/analyzers/purity_detector.rs:204-280`):
- Each function analyzed independently
- Unknown function calls conservatively marked as potential side effects
- No propagation of purity information across function boundaries
- Breaks analytical chain even when all callees are pure

**Impact on Scoring**:
- Pure functions calling pure functions get 1.0x multiplier instead of 0.70x
- Risk level artificially inflated from Low to Medium/High
- Functional programming patterns (composition, pipelines) penalized
- Developers discouraged from extracting helper functions

## Objective

Implement **two-phase inter-procedural purity analysis** that propagates purity information from callees to callers, enabling whole-program purity inference while maintaining conservative safety guarantees.

## Requirements

### Functional Requirements

1. **Call Graph Construction**
   - Build complete function call graph during analysis
   - Track caller-callee relationships with source locations
   - Handle direct calls, method calls, and trait method calls
   - Support cross-file and cross-module analysis

2. **Two-Phase Analysis**
   - **Phase 1**: Analyze all functions in isolation, build initial purity estimates
   - **Phase 2**: Propagate purity bottom-up from leaf functions to roots
   - Use topological sort to ensure dependencies analyzed first
   - Handle cycles conservatively (recursive functions marked impure)

3. **Purity Propagation Rules**
   - If all callees are pure AND function has no other side effects → function is pure
   - If any callee is impure → function is impure (unless side effect is isolated)
   - Unknown functions → conservative (impure) unless marked with `#[pure]` attribute
   - Confidence reduced for each level of indirection

4. **Caching and Incremental Analysis**
   - Cache purity results by function signature
   - Support incremental re-analysis when files change
   - Invalidate cache for affected functions in call graph
   - Persist cache across debtmap runs

### Non-Functional Requirements

- **Performance**: Two-phase analysis completes within 2x time of single-phase
- **Accuracy**: Reduce false negative rate from 40-60% to <15%
- **Scalability**: Handle call graphs with 10,000+ functions
- **Memory**: Purity cache stays under 50MB for 100K LOC projects

## Acceptance Criteria

- [ ] Call graph correctly identifies all function dependencies across files
- [ ] Topological sort handles acyclic call graphs correctly
- [ ] Recursive functions detected and marked impure with high confidence
- [ ] Pure function calling pure functions correctly marked pure (0.70x multiplier)
- [ ] Confidence decreases by 0.1 per call depth level (max depth 5)
- [ ] Cache persists between runs and invalidates on file changes
- [ ] Performance: 100K LOC codebase analyzed in <45 seconds (vs 25s baseline)
- [ ] False negative rate reduced to <15% on validation corpus
- [ ] Integration tests verify purity propagates through 3+ levels of calls

## Technical Details

### Implementation Approach

#### Phase 1: Call Graph Construction

```rust
// src/analysis/call_graph.rs

#[derive(Debug, Clone)]
pub struct CallGraph {
    /// Map from function ID to list of functions it calls
    dependencies: DashMap<FunctionId, Vec<FunctionId>>,

    /// Reverse map: function ID to functions that call it
    dependents: DashMap<FunctionId, Vec<FunctionId>>,

    /// Strongly connected components (for cycle detection)
    sccs: Vec<Vec<FunctionId>>,
}

impl CallGraph {
    /// Build call graph from analyzed functions
    pub fn build(functions: &[FunctionMetrics]) -> Result<Self> {
        let mut graph = Self::new();

        for func in functions {
            let func_id = FunctionId::from_metrics(func);

            // Extract function calls from AST
            let calls = Self::extract_function_calls(&func.ast)?;

            for callee in calls {
                graph.add_edge(func_id.clone(), callee);
            }
        }

        // Detect cycles using Tarjan's algorithm
        graph.sccs = graph.find_strongly_connected_components();

        Ok(graph)
    }

    /// Topological sort for bottom-up analysis
    pub fn topological_sort(&self) -> Result<Vec<FunctionId>> {
        // Kahn's algorithm
        let mut in_degree = self.compute_in_degrees();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Start with leaf functions (in-degree 0)
        for (func_id, degree) in &in_degree {
            if *degree == 0 {
                queue.push_back(func_id.clone());
            }
        }

        while let Some(func_id) = queue.pop_front() {
            result.push(func_id.clone());

            // Reduce in-degree for dependents
            if let Some(dependents) = self.dependents.get(&func_id) {
                for dep in dependents.iter() {
                    let degree = in_degree.get_mut(dep).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        // Check for cycles
        if result.len() != self.dependencies.len() {
            bail!("Call graph contains cycles");
        }

        Ok(result)
    }

    /// Find strongly connected components (cycles)
    fn find_strongly_connected_components(&self) -> Vec<Vec<FunctionId>> {
        // Tarjan's algorithm for SCC detection
        // Functions in same SCC are mutually recursive
        tarjan_scc(&self.dependencies)
    }
}
```

#### Phase 2: Purity Propagation

```rust
// src/analysis/purity_propagation.rs

pub struct PurityPropagator {
    /// Cache of function purity results
    cache: DashMap<FunctionId, PurityResult>,

    /// Call graph for dependency tracking
    call_graph: CallGraph,
}

#[derive(Debug, Clone)]
pub struct PurityResult {
    pub level: PurityLevel,
    pub confidence: f64,
    pub reason: PurityReason,
}

#[derive(Debug, Clone)]
pub enum PurityReason {
    /// Function has no side effects or calls
    Intrinsic,

    /// All dependencies are pure
    PropagatedFromDeps { depth: usize },

    /// Has side effects
    SideEffects { effects: Vec<SideEffect> },

    /// Part of recursive cycle
    Recursive,

    /// Unknown dependencies
    UnknownDeps { count: usize },
}

impl PurityPropagator {
    pub fn propagate(&mut self, functions: &[FunctionMetrics]) -> Result<()> {
        // Phase 1: Initial purity analysis (existing code)
        for func in functions {
            let initial = self.analyze_intrinsic_purity(func)?;
            let func_id = FunctionId::from_metrics(func);
            self.cache.insert(func_id, initial);
        }

        // Phase 2: Propagate purity bottom-up
        let sorted = self.call_graph.topological_sort()?;

        for func_id in sorted {
            self.propagate_for_function(&func_id)?;
        }

        Ok(())
    }

    fn propagate_for_function(&mut self, func_id: &FunctionId) -> Result<()> {
        // Get current purity result
        let mut result = self.cache.get(func_id)
            .ok_or_else(|| anyhow!("Function not in cache"))?
            .clone();

        // Get all dependencies
        let deps = self.call_graph.get_dependencies(func_id);

        // Check if function is in a cycle
        if self.call_graph.is_in_cycle(func_id) {
            result.level = PurityLevel::Impure;
            result.reason = PurityReason::Recursive;
            result.confidence = 0.95;
            self.cache.insert(func_id.clone(), result);
            return Ok(());
        }

        // Check all dependencies
        let mut all_deps_pure = true;
        let mut max_depth = 0;
        let mut unknown_count = 0;

        for dep_id in &deps {
            if let Some(dep_result) = self.cache.get(dep_id) {
                if dep_result.level != PurityLevel::StrictlyPure {
                    all_deps_pure = false;
                    break;
                }

                // Track propagation depth
                if let PurityReason::PropagatedFromDeps { depth } = dep_result.reason {
                    max_depth = max_depth.max(depth);
                }
            } else {
                unknown_count += 1;
                all_deps_pure = false;
            }
        }

        // Update purity if all deps are pure
        if all_deps_pure && result.level != PurityLevel::Impure {
            result.level = PurityLevel::StrictlyPure;
            result.reason = PurityReason::PropagatedFromDeps {
                depth: max_depth + 1
            };

            // Reduce confidence based on depth
            result.confidence *= 0.9_f64.powi(max_depth as i32 + 1);
            result.confidence = result.confidence.max(0.5);
        } else if unknown_count > 0 {
            result.reason = PurityReason::UnknownDeps { count: unknown_count };
            result.confidence *= 0.8;
        }

        self.cache.insert(func_id.clone(), result);
        Ok(())
    }
}
```

### Data Structures

```rust
/// Unique identifier for functions across the codebase
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FunctionId {
    pub file_path: PathBuf,
    pub module_path: String,
    pub function_name: String,
    pub signature_hash: u64,  // Hash of parameter types
}

impl FunctionId {
    pub fn from_metrics(func: &FunctionMetrics) -> Self {
        Self {
            file_path: func.file_path.clone(),
            module_path: func.module_path.clone(),
            function_name: func.name.clone(),
            signature_hash: Self::hash_signature(&func.params),
        }
    }
}
```

### Integration with Scoring

```rust
// src/priority/unified_scorer.rs

fn calculate_purity_adjustment(func: &FunctionMetrics, purity: &PurityResult) -> f64 {
    match purity.level {
        PurityLevel::StrictlyPure => {
            // Apply confidence-based multiplier
            if purity.confidence > 0.8 {
                0.70  // High confidence: 30% reduction
            } else if purity.confidence > 0.6 {
                0.80  // Medium confidence: 20% reduction
            } else {
                0.90  // Low confidence: 10% reduction
            }
        }
        _ => 1.0
    }
}
```

## Dependencies

- **Prerequisites**: None (foundational change)
- **Affected Components**:
  - `src/analyzers/purity_detector.rs` - Extract call information
  - `src/analysis/purity_analysis.rs` - Integrate propagation
  - `src/priority/unified_scorer.rs` - Use propagated purity
  - `src/data_flow.rs` - Use call graph
- **External Dependencies**: None (pure Rust implementation)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_function_calling_pure_function() {
        let code = r#"
            fn add(a: i32, b: i32) -> i32 { a + b }
            fn sum_with_offset(items: &[i32]) -> i32 {
                items.iter().map(|x| add(*x, 10)).sum()
            }
        "#;

        let analysis = analyze_with_propagation(code).unwrap();
        let sum_func = analysis.get_function("sum_with_offset").unwrap();

        assert_eq!(sum_func.purity_level, PurityLevel::StrictlyPure);
        assert!(sum_func.purity_confidence > 0.7);
    }

    #[test]
    fn test_recursive_function_marked_impure() {
        let code = r#"
            fn factorial(n: u32) -> u32 {
                if n <= 1 { 1 } else { n * factorial(n - 1) }
            }
        "#;

        let analysis = analyze_with_propagation(code).unwrap();
        let func = analysis.get_function("factorial").unwrap();

        assert_eq!(func.purity_level, PurityLevel::Impure);
        assert_eq!(func.purity_reason, PurityReason::Recursive);
    }

    #[test]
    fn test_confidence_decreases_with_depth() {
        let code = r#"
            fn level0(x: i32) -> i32 { x + 1 }
            fn level1(x: i32) -> i32 { level0(x) }
            fn level2(x: i32) -> i32 { level1(x) }
            fn level3(x: i32) -> i32 { level2(x) }
        "#;

        let analysis = analyze_with_propagation(code).unwrap();

        let l0 = analysis.get_function("level0").unwrap();
        let l3 = analysis.get_function("level3").unwrap();

        assert!(l0.purity_confidence > l3.purity_confidence);
        assert!(l3.purity_confidence > 0.5); // Still reasonably confident
    }
}
```

### Integration Tests

```rust
// tests/purity_propagation_test.rs

#[test]
fn test_cross_file_purity_propagation() {
    // File 1: utils.rs
    let utils = r#"
        pub fn safe_divide(a: f64, b: f64) -> Option<f64> {
            if b != 0.0 { Some(a / b) } else { None }
        }
    "#;

    // File 2: calculator.rs
    let calculator = r#"
        use crate::utils::safe_divide;

        pub fn calculate_ratio(nums: &[f64]) -> Vec<Option<f64>> {
            nums.windows(2)
                .map(|w| safe_divide(w[0], w[1]))
                .collect()
        }
    "#;

    let analysis = analyze_multi_file(&[utils, calculator]).unwrap();
    let calc_func = analysis.get_function("calculate_ratio").unwrap();

    assert_eq!(calc_func.purity_level, PurityLevel::StrictlyPure);
}
```

### Performance Tests

```rust
#[bench]
fn bench_propagation_10k_functions(b: &mut Bencher) {
    let functions = generate_call_graph_with_n_functions(10_000);

    b.iter(|| {
        let mut propagator = PurityPropagator::new();
        propagator.propagate(&functions).unwrap();
    });
}
```

## Documentation Requirements

### Code Documentation

- Document `CallGraph` API with examples
- Explain topological sort algorithm choice
- Document confidence degradation formula
- Add examples for common patterns

### User Documentation

Update `docs/purity-analysis.md`:
```markdown
## Inter-Procedural Analysis

Debtmap now performs **whole-program purity analysis**. Pure functions that call
other pure functions are correctly identified as pure.

**Example**:
```rust
fn add(a: i32, b: i32) -> i32 { a + b }

fn calculate_total(items: &[i32]) -> i32 {
    items.iter().map(|x| add(*x, 10)).sum()
}
```

Both functions receive a 0.70x complexity multiplier (30% reduction) because
debtmap recognizes that `calculate_total` only calls pure functions.
```

### Architecture Updates

Add to `ARCHITECTURE.md`:
- Call graph construction in analysis pipeline
- Two-phase analysis workflow
- Caching and invalidation strategy

## Implementation Notes

### Handling Edge Cases

1. **Circular Dependencies**: Use Tarjan's algorithm to detect cycles, mark all functions in cycle as impure
2. **Generic Functions**: Create separate `FunctionId` per monomorphization
3. **Trait Methods**: Conservative approach - only propagate if concrete impl known
4. **Closures**: Treat as inline code within parent function scope

### Performance Optimizations

- Use `DashMap` for concurrent cache access
- Topological sort once, reuse for multiple analyses
- Lazy SCC computation (only when cycles detected)
- Incremental analysis: only re-propagate affected subgraph

### Migration Path

1. Add feature flag: `--enable-interprocedural-purity`
2. Run in parallel with existing analysis, compare results
3. Validate against ground truth corpus
4. Enable by default after validation
5. Remove old single-phase analysis

## Migration and Compatibility

### Breaking Changes

- `FunctionMetrics` gains new fields: `purity_reason`, `call_dependencies`
- Purity confidence formula changes (may affect scores)

### Migration Strategy

- Add new fields with `Option<T>` for backward compatibility
- Populate during analysis, None indicates old analysis
- Update scoring to handle both old and new formats

### Compatibility Guarantees

- Old analysis cache formats still readable (ignored fields)
- New cache format versioned with schema version
- Gradual rollout via feature flag
