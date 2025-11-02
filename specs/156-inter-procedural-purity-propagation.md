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

**Scope**: This specification targets **Rust-only** implementation initially. Multi-language support (Python, JavaScript, TypeScript) will be addressed in follow-up specifications once the Rust implementation is validated.

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

### Integration with Existing Infrastructure

This implementation leverages significant existing infrastructure in debtmap:

1. **Call Graph System** (`src/analysis/call_graph/mod.rs`)
   - Reuse existing `RustCallGraph` with trait dispatch, function pointers, closures
   - Leverage `TraitRegistry`, `FunctionPointerTracker`, `CrossModuleTracker`
   - Use existing `FunctionId` from `src/priority/call_graph/types.rs`

2. **Purity Analysis** (`src/analysis/purity_analysis.rs`)
   - Extend existing `PurityAnalyzer` for phase 1 (intrinsic analysis)
   - Reuse `PurityLevel`, `PurityViolation`, `PurityAnalysis` types
   - Integrate with existing `IoDetector` (Spec 141)

3. **FunctionMetrics Integration** (`src/core/mod.rs`)
   - Already has `is_pure: Option<bool>` and `purity_confidence: Option<f32>`
   - Add optional fields: `purity_reason: Option<String>`, `call_dependencies: Option<Vec<String>>`

4. **Multi-Pass Framework** (`src/analysis/multi_pass.rs`)
   - Integrate propagation as additional analysis phase
   - Use existing performance tracking (25% overhead limit)

### Implementation Approach

#### Phase 1: Call Graph Construction

**Note**: Leverage existing `RustCallGraph` from `src/analysis/call_graph/mod.rs` instead of building from scratch.

```rust
// src/analysis/purity_propagation/call_graph_adapter.rs

use crate::analysis::call_graph::RustCallGraph;
use crate::priority::call_graph::{CallGraph, FunctionId};

/// Adapter to use existing RustCallGraph for purity propagation
pub struct PurityCallGraphAdapter {
    rust_graph: RustCallGraph,
}

impl PurityCallGraphAdapter {
    /// Create adapter from existing call graph
    pub fn from_rust_graph(rust_graph: RustCallGraph) -> Self {
        Self { rust_graph }
    }

    /// Get dependencies for a function
    pub fn get_dependencies(&self, func_id: &FunctionId) -> Vec<FunctionId> {
        self.rust_graph.base_graph.get_callees(func_id)
    }

    /// Get dependents (callers) for a function
    pub fn get_dependents(&self, func_id: &FunctionId) -> Vec<FunctionId> {
        self.rust_graph.base_graph.get_callers(func_id)
    }

    /// Check if function is in a cycle (recursive)
    pub fn is_in_cycle(&self, func_id: &FunctionId) -> bool {
        self.rust_graph.base_graph.is_recursive(func_id)
    }

    /// Topological sort for bottom-up analysis
    pub fn topological_sort(&self) -> Result<Vec<FunctionId>> {
        // Delegate to existing call graph implementation
        self.rust_graph.base_graph.topological_sort()
    }
}
```

**Key Advantage**: Reusing `RustCallGraph` provides:
- Trait method resolution (no false negatives from trait dispatch)
- Function pointer and closure tracking
- Framework pattern detection (test functions, handlers)
- Cross-module dependency analysis
- Already tested and validated infrastructure

#### Phase 2: Purity Propagation

```rust
// src/analysis/purity_propagation/mod.rs

use crate::analysis::purity_analysis::{PurityAnalyzer, PurityAnalysis, PurityLevel};
use crate::priority::call_graph::FunctionId;

pub struct PurityPropagator {
    /// Cache of function purity results
    cache: DashMap<FunctionId, PurityResult>,

    /// Call graph adapter for dependency tracking
    call_graph: PurityCallGraphAdapter,

    /// Existing purity analyzer for intrinsic analysis (phase 1)
    purity_analyzer: PurityAnalyzer,
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

    /// Part of recursive cycle with side effects
    RecursiveWithSideEffects,

    /// Part of recursive cycle but otherwise pure
    RecursivePure,

    /// Unknown dependencies
    UnknownDeps { count: usize },
}

impl PurityResult {
    /// Convert from existing PurityAnalysis (phase 1 result)
    pub fn from_analysis(analysis: PurityAnalysis) -> Self {
        let reason = if !analysis.violations.is_empty() {
            PurityReason::SideEffects {
                effects: analysis.violations.iter().map(|v| v.description()).collect(),
            }
        } else {
            PurityReason::Intrinsic
        };

        Self {
            level: analysis.purity,
            confidence: 1.0,
            reason,
        }
    }
}

impl PurityPropagator {
    pub fn propagate(&mut self, functions: &[FunctionMetrics]) -> Result<()> {
        // Phase 1: Initial purity analysis using existing PurityAnalyzer
        for func in functions {
            let initial = self.analyze_intrinsic_purity(func)?;
            let func_id = FunctionId::new(func.file.clone(), func.name.clone(), func.line);
            self.cache.insert(func_id, initial);
        }

        // Phase 2: Propagate purity bottom-up
        let sorted = self.call_graph.topological_sort()?;

        for func_id in sorted {
            self.propagate_for_function(&func_id)?;
        }

        Ok(())
    }

    /// Analyze intrinsic purity using existing PurityAnalyzer
    fn analyze_intrinsic_purity(&self, func: &FunctionMetrics) -> Result<PurityResult> {
        // Get function source code from file
        let source = extract_function_source(&func.file, func.line)?;

        // Use existing PurityAnalyzer for intrinsic analysis
        let analysis = self.purity_analyzer.analyze_code(&source, Language::Rust);

        // Convert to PurityResult
        Ok(PurityResult::from_analysis(analysis))
    }

    fn propagate_for_function(&mut self, func_id: &FunctionId) -> Result<()> {
        // Get current purity result
        let mut result = self.cache.get(func_id)
            .ok_or_else(|| anyhow!("Function not in cache"))?
            .clone();

        // Get all dependencies
        let deps = self.call_graph.get_dependencies(func_id);

        // Check if function is in a cycle (recursive)
        if self.call_graph.is_in_cycle(func_id) {
            // Distinguish between pure recursion and recursion with side effects
            if result.level == PurityLevel::StrictlyPure || result.level == PurityLevel::LocallyPure {
                // Pure structural recursion (e.g., factorial, tree traversal)
                // Keep pure but reduce confidence due to recursion complexity
                result.reason = PurityReason::RecursivePure;
                result.confidence *= 0.7; // Penalty for recursion
            } else {
                // Recursion with side effects is impure
                result.level = PurityLevel::Impure;
                result.reason = PurityReason::RecursiveWithSideEffects;
                result.confidence = 0.95;
            }
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

**Note**: Reuse existing `FunctionId` from `src/priority/call_graph/types.rs`:

```rust
// Existing FunctionId structure (no changes needed)
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct FunctionId {
    pub file: PathBuf,
    pub name: String,
    pub line: usize,
    #[serde(default)]
    pub module_path: String,
}

// Already has constructor:
impl FunctionId {
    pub fn new(file: PathBuf, name: String, line: usize) -> Self { ... }
}
```

**Limitation**: This `FunctionId` doesn't distinguish between function overloads (same name, different signatures). This is acceptable because:
- Rust doesn't support function overloading (only trait methods can have same name)
- Line numbers provide sufficient uniqueness for same-named functions in different scopes
- Trait methods are handled separately by `TraitRegistry` in call graph

### Persistent Caching

Purity propagation results are cached to avoid re-analysis on subsequent runs:

```rust
// src/analysis/purity_propagation/cache.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const CACHE_VERSION: u32 = 1;
const CACHE_FILE: &str = ".debtmap/purity_cache.bincode";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurityCache {
    /// Schema version for migration compatibility
    version: u32,

    /// Cached purity results indexed by function ID
    entries: HashMap<FunctionId, CachedPurity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedPurity {
    /// Purity propagation result
    result: PurityResult,

    /// xxHash64 of function source code
    source_hash: u64,

    /// xxHash64 of sorted dependency IDs
    deps_hash: u64,

    /// File modification time (seconds since epoch)
    file_mtime: u64,
}

impl PurityCache {
    /// Load cache from disk, creating new if doesn't exist
    pub fn load(project_root: &Path) -> Result<Self> {
        let cache_path = project_root.join(CACHE_FILE);

        if !cache_path.exists() {
            return Ok(Self::new());
        }

        let bytes = std::fs::read(&cache_path)?;
        let cache: PurityCache = bincode::deserialize(&bytes)?;

        // Validate version
        if cache.version != CACHE_VERSION {
            eprintln!("Cache version mismatch, rebuilding cache");
            return Ok(Self::new());
        }

        Ok(cache)
    }

    /// Save cache to disk
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let cache_path = project_root.join(CACHE_FILE);
        std::fs::create_dir_all(cache_path.parent().unwrap())?;

        let bytes = bincode::serialize(self)?;
        std::fs::write(&cache_path, bytes)?;

        Ok(())
    }

    /// Check if cached entry is still valid
    pub fn is_valid(&self, func_id: &FunctionId, current_mtime: u64,
                     current_source_hash: u64, current_deps_hash: u64) -> bool {
        if let Some(cached) = self.entries.get(func_id) {
            cached.file_mtime == current_mtime
                && cached.source_hash == current_source_hash
                && cached.deps_hash == current_deps_hash
        } else {
            false
        }
    }

    /// Invalidate entries for a specific file
    pub fn invalidate_file(&mut self, file_path: &Path) {
        self.entries.retain(|id, _| id.file != file_path);
    }
}
```

**Cache Invalidation Strategy**:
1. **File modification time** - Invalidate if file changed
2. **Source hash** - Detect in-file changes even if mtime unchanged
3. **Dependencies hash** - Invalidate if any dependency changed
4. **Transitive invalidation** - If function A's purity changes, invalidate all callers of A

**Memory Limit**: Cache size is bounded by file count × functions per file. For 100K LOC:
- ~2000 files × 20 functions/file = 40,000 entries
- ~200 bytes/entry × 40,000 = 8MB (well under 50MB limit)

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
- **Existing Infrastructure** (reused):
  - `src/analysis/call_graph/mod.rs` - RustCallGraph with trait dispatch
  - `src/analysis/purity_analysis.rs` - PurityAnalyzer for intrinsic analysis
  - `src/priority/call_graph/` - FunctionId and base CallGraph
  - `src/core/mod.rs` - FunctionMetrics with purity fields
  - `src/analysis/multi_pass.rs` - Multi-phase analysis framework
- **New Components** (to be created):
  - `src/analysis/purity_propagation/mod.rs` - PurityPropagator
  - `src/analysis/purity_propagation/cache.rs` - PurityCache
  - `src/analysis/purity_propagation/call_graph_adapter.rs` - Adapter
- **External Dependencies**:
  - `bincode` (already in dependencies) - Cache serialization
  - `xxhash-rust` (add) - Fast hashing for cache validation

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
    fn test_pure_recursive_function() {
        let code = r#"
            fn factorial(n: u32) -> u32 {
                if n <= 1 { 1 } else { n * factorial(n - 1) }
            }
        "#;

        let analysis = analyze_with_propagation(code).unwrap();
        let func = analysis.get_function("factorial").unwrap();

        // Pure recursion should remain pure with reduced confidence
        assert_eq!(func.purity_level, PurityLevel::StrictlyPure);
        assert_eq!(func.purity_reason, PurityReason::RecursivePure);
        assert!(func.purity_confidence < 1.0); // Reduced confidence
        assert!(func.purity_confidence >= 0.7); // But still reasonably confident
    }

    #[test]
    fn test_impure_recursive_function() {
        let code = r#"
            fn recursive_with_io(n: u32) {
                if n > 0 {
                    println!("Count: {}", n);
                    recursive_with_io(n - 1);
                }
            }
        "#;

        let analysis = analyze_with_propagation(code).unwrap();
        let func = analysis.get_function("recursive_with_io").unwrap();

        // Recursion with side effects is impure
        assert_eq!(func.purity_level, PurityLevel::Impure);
        assert_eq!(func.purity_reason, PurityReason::RecursiveWithSideEffects);
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

1. **Circular Dependencies**: Detect cycles using existing call graph infrastructure
   - **Pure recursion** (e.g., factorial): Keep pure, reduce confidence by 30%
   - **Impure recursion** (e.g., recursive I/O): Mark as impure with high confidence
   - Rationale: Pure structural recursion is mathematically pure and common in functional code

2. **Generic Functions**: Treat as single function (no per-monomorphization tracking)
   - Line numbers differentiate same-named functions in different scopes
   - Purity analysis is signature-agnostic (generic type parameters don't affect purity)

3. **Trait Methods**: Leverage existing `TraitRegistry` from `RustCallGraph`
   - Propagate purity for trait methods with known concrete implementations
   - Unknown trait impls: conservative (impure) unless marked `#[pure]` attribute (future work)

4. **Closures**: Leverage existing `FunctionPointerTracker`
   - Closures analyzed inline within parent function
   - Closure captures affect parent purity (mutable captures = impure parent)

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

**None** - All changes are backward compatible:
- `FunctionMetrics` already has `is_pure: Option<bool>` and `purity_confidence: Option<f32>`
- New optional fields added with `Option<T>` wrapper
- Scoring gracefully handles `None` values (no propagated purity = use existing logic)

### New Optional Fields in FunctionMetrics

```rust
// src/core/mod.rs
pub struct FunctionMetrics {
    // ... existing fields ...

    /// Optional: Reason for purity classification (from propagation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purity_reason: Option<String>,

    /// Optional: List of function IDs this function calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_dependencies: Option<Vec<String>>,
}
```

### Migration Strategy

1. **Phase 1**: Add purity propagation as opt-in feature flag
   ```bash
   debtmap analyze --enable-interprocedural-purity
   ```

2. **Phase 2**: Run both analyses in parallel, compare results
   - Log discrepancies for validation
   - Gather metrics on false negative reduction

3. **Phase 3**: Enable by default after validation (4-6 weeks)
   - Keep feature flag for rollback capability
   - Monitor performance metrics

4. **Phase 4**: Remove feature flag after stable (2-3 months)
   - Old single-phase analysis code can be archived

### Compatibility Guarantees

- **Cache versioning**: `CACHE_VERSION` field ensures safe schema evolution
- **Graceful degradation**: Missing cache file = full analysis (no errors)
- **JSON output**: New fields only included when present (`skip_serializing_if`)
- **Performance**: Feature flag allows rollback if performance degrades

---

## Specification Improvements Summary

This specification has been updated with the following enhancements:

### 1. **Language Scope Clarification**
- **Added**: Explicit Rust-only scope for initial implementation
- **Rationale**: Multi-language support requires separate designs for Python, JavaScript, TypeScript

### 2. **Integration with Existing Infrastructure**
- **Added**: Comprehensive section documenting reuse of existing components
- **Key Reuse**:
  - `RustCallGraph` with trait dispatch, function pointers, closures
  - `PurityAnalyzer` for intrinsic (phase 1) analysis
  - Existing `FunctionId` from call graph types
  - `MultiPassAnalyzer` framework for performance tracking
- **Benefit**: ~60% of infrastructure already exists, reducing implementation complexity

### 3. **Persistent Caching Implementation**
- **Added**: Complete cache implementation with:
  - Binary serialization (bincode format)
  - Multi-level invalidation (mtime, source hash, deps hash)
  - Schema versioning for future migrations
  - Memory bounds calculation (8MB for 100K LOC)
- **Rationale**: Required for acceptable performance on large codebases

### 4. **Refined Recursive Function Handling**
- **Changed**: Distinguished between pure recursion vs recursion with side effects
- **Before**: All recursive functions marked impure
- **After**: Pure recursion (factorial, tree traversal) stays pure with 30% confidence penalty
- **Rationale**: Pure structural recursion is mathematically pure and common in functional programming

### 5. **Data Structure Simplification**
- **Changed**: Use existing `FunctionId` instead of new structure
- **Removed**: `signature_hash` field (not needed for Rust)
- **Rationale**: Rust doesn't support function overloading; line numbers provide uniqueness

### 6. **Updated Test Cases**
- **Added**: Separate tests for pure vs impure recursion
- **Changed**: Adjusted expectations to match refined recursion handling

### 7. **Enhanced Migration Strategy**
- **Added**: 4-phase rollout plan with feature flag
- **Added**: Explicit opt-in command line flag
- **Added**: Backward compatibility guarantees
- **Rationale**: Safe, gradual rollout minimizes risk

### 8. **External Dependencies**
- **Added**: `xxhash-rust` for fast cache validation hashing
- **Clarified**: `bincode` already in dependencies

### Implementation Complexity Reduction

**Estimated LOC**: ~1100 new lines
- PurityPropagator: ~300 LOC
- PurityCache: ~200 LOC
- Integration glue: ~150 LOC
- Tests: ~400 LOC
- Documentation: ~50 LOC

**Complexity Level**: Medium (was High) due to infrastructure reuse

**Implementation Time**: 3-5 days for experienced Rust developer (was 7-10 days)
