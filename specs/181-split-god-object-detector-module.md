---
number: 181
title: Refactor God Object Detection Module (Stillwater Compliant)
category: optimization
priority: high
status: draft
dependencies: []
sub_specs: [181a, 181b, 181c, 181d, 181e, 181f, 181g, 181h, 181i]
created: 2025-11-30
updated: 2025-12-03
---

# Specification 181: Refactor God Object Detection Module (Stillwater Compliant)

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

---

## IMPORTANT: This Spec Has Been Split for Automation

This spec is **too large for single-run automation** (estimated 6-7 days). It has been split into **9 sub-specs** that can be executed independently:

| Sub-Spec | Phase | Estimated Time | Description |
|----------|-------|----------------|-------------|
| **181a** | 1 | 1 day | Foundation & Analysis (read-only planning) |
| **181b** | 2 | 0.5 day | Extract Types & Thresholds |
| **181c** | 3 | 1 day | Extract Pure Scoring Functions |
| **181d** | 4 | 0.5 day | Extract Pure Predicates |
| **181e** | 5 | 1 day | Extract Classification Logic |
| **181f** | 6 | 1 day | Extract Recommendation Logic |
| **181g** | 7 | 1 day | Create Orchestration Layer |
| **181h** | 8 | 0.5 day | Update Public API & Cleanup |
| **181i** | 9 | 0.5 day | Delete Old Files (final) |

**To implement this spec**, run each sub-spec sequentially:
```bash
prodigy run workflows/implement.yml -y --args 181a
prodigy run workflows/implement.yml -y --args 181b
prodigy run workflows/implement.yml -y --args 181c
# ... and so on through 181i
```

**Completion criteria**: This spec is complete when all 9 sub-specs (181a through 181i) are marked as completed.

---

## Context

The god object detection system has grown to 8,033 lines across three files, violating both the Stillwater philosophy and project guidelines:

- `src/organization/god_object_detector.rs` - 4,362 lines (detection + orchestration)
- `src/organization/god_object_analysis.rs` - 3,304 lines (analysis + scoring)
- `src/organization/god_object_metrics.rs` - 367 lines (metric tracking)

Additionally, partial modularization exists in `src/organization/god_object/`:
- `ast_visitor.rs` - 365 lines (AST traversal and data collection)
- `metrics.rs` - 349 lines (metric calculations)
- `mod.rs` - 15 lines (module exports)

### Stillwater Philosophy Violations

According to `../stillwater/PHILOSOPHY.md`, this code violates:

1. **Pure Core, Imperative Shell** - Mixed I/O with business logic throughout
2. **Composition Over Complexity** - Monolithic functions instead of composable pieces
3. **Types Guide, Don't Restrict** - Unclear boundaries and responsibilities
4. **Pragmatism Over Purity** - Fighting modularity instead of embracing it

### Project Guideline Violations

From `CLAUDE.md`:
- Files should be under 200 lines (target), max 300 lines acceptable
- Functions should be under 20 lines
- Maximum cyclomatic complexity of 5
- Pure functions separated from I/O operations

### Problems Created

Large, mixed-responsibility files make it difficult to:
- **Test** - Heavy coupling requires complex test setups
- **Understand** - Multiple concerns interwoven across 4000+ lines
- **Refactor** - Fear of breaking hidden dependencies
- **Collaborate** - Frequent merge conflicts
- **Maintain** - Cannot isolate changes to single concern

## Objective

Refactor the god object detection system following **Stillwater principles**:

**Pure Core** (business logic):
- Detection predicates (pure functions)
- Classification algorithms (pure transformations)
- Scoring calculations (deterministic math)
- Recommendation generators (pure decision trees)

**Imperative Shell** (I/O boundary):
- AST traversal (already separated in `ast_visitor.rs`)
- File system operations (in callers)
- Output formatting (in io module)

**Module Structure** under `src/organization/god_object/`:
- Each module **< 200 lines** (300 line absolute maximum)
- Clear, acyclic dependency graph
- Single responsibility per module
- Composable, testable functions

## Requirements

### Functional Requirements

1. **Stillwater Architecture**
   - **Pure Core**: All detection, classification, scoring logic is pure
   - **I/O Shell**: AST traversal isolated to `ast_visitor.rs` (already done)
   - **Composability**: Small functions that compose into pipelines
   - **Testability**: 100% unit test coverage for pure functions

2. **Module Structure**
   - Refactor under `src/organization/god_object/` directory
   - Each module < 200 lines (strict), < 300 lines (absolute max)
   - Single responsibility per module
   - Acyclic dependency graph

3. **Integration with Existing Modules**
   - Preserve existing `ast_visitor.rs` (365 lines - AST data collection)
   - Audit existing `metrics.rs` (349 lines) - may need splitting
   - Merge `god_object_analysis.rs` functions into appropriate modules
   - Delete `god_object_detector.rs` and `god_object_analysis.rs` when complete

4. **Public API Compatibility**
   - Preserve all public exports from `mod.rs` (line 30-57)
   - Maintain `GodObjectDetector` struct and `OrganizationDetector` trait impl
   - No breaking changes to consumers
   - Clear deprecation warnings if needed

5. **Dependency Direction (Acyclic)**
   ```
   types.rs (foundation)
     ↑
     ├── thresholds.rs
     ├── predicates.rs
     ├── scoring.rs
     │     ↑
     ├── classifier.rs
     │     ↑
     ├── recommender.rs
     │     ↑
     └── detector.rs (orchestration)
           ↑
         mod.rs (public API)

   ast_visitor.rs (I/O shell - parallel, no core deps)
   ```

### Non-Functional Requirements

1. **Performance**
   - No performance regression (< 5% overhead acceptable)
   - Maintain parallel processing capabilities
   - Zero additional heap allocations in hot paths
   - Benchmark critical paths before/after

2. **Maintainability**
   - Each module independently testable
   - Clear, documented boundaries between concerns
   - Functions < 20 lines, cyclomatic complexity < 5
   - Comprehensive module-level documentation with examples

3. **Testability**
   - All 6 existing test files continue to pass
   - Unit tests for every pure function
   - Property tests for invariants
   - Integration tests for composition

4. **Incremental Migration**
   - Work proceeds in small, testable stages
   - Each stage compiles and passes tests
   - No "big bang" switchover
   - Gradual deprecation of old modules

## Acceptance Criteria

### Architecture
- [ ] All modules under `src/organization/god_object/`
- [ ] Each module < 200 lines (300 absolute max)
- [ ] Pure core functions separated from I/O shell
- [ ] No circular dependencies (verified with `cargo-depgraph`)
- [ ] Clear dependency hierarchy (types → utils → domain → orchestration)

### Modules Created/Updated
- [ ] `types.rs` - Data structures (< 200 lines)
- [ ] `thresholds.rs` - Configuration and constants (< 150 lines)
- [ ] `predicates.rs` - Pure detection predicates (< 200 lines)
- [ ] `scoring.rs` - Pure scoring algorithms (< 200 lines)
- [ ] `classifier.rs` - Pure classification logic (< 200 lines)
- [ ] `recommender.rs` - Pure recommendation generation (< 250 lines)
- [ ] `detector.rs` - Orchestration layer (< 250 lines)
- [ ] `mod.rs` - Public API and re-exports (< 150 lines)
- [ ] `ast_visitor.rs` - PRESERVED as-is (I/O shell)
- [ ] Existing `metrics.rs` - AUDITED and possibly split

### Cleanup
- [ ] `god_object_detector.rs` deleted
- [ ] `god_object_analysis.rs` deleted
- [ ] All functionality preserved in new modules
- [ ] Deprecated re-exports removed after 1 release

### Quality
- [ ] All 6 test files pass (`tests/god_object_*.rs`)
- [ ] No test modifications required (backward compatible)
- [ ] Unit tests added for all pure functions
- [ ] Module-level documentation with examples
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes with no failures
- [ ] `cargo bench` shows < 5% regression

### Public API
- [ ] `GodObjectDetector` struct preserved
- [ ] `OrganizationDetector` trait implementation preserved
- [ ] All public functions re-exported from `mod.rs`
- [ ] No breaking changes to external consumers
- [ ] Documentation updated for module organization

## Technical Details

### Implementation Approach (Incremental)

Following **Stillwater principle**: "Incremental progress over big bangs"

**Phase 1: Foundation & Analysis** (1 day)
1. Audit existing `god_object/` modules (`ast_visitor.rs`, `metrics.rs`)
2. Map all public API exports from current files
3. Create dependency graph of all functions
4. Identify pure vs impure functions in both files
5. Group functions by responsibility (types, scoring, detection, etc.)
6. Write benchmarks for critical paths

**Deliverable**: `REFACTORING_PLAN.md` with function-to-module mapping

**Phase 2: Extract Types & Thresholds** (0.5 day)
1. Create `types.rs` with all data structures from both files
2. Create `thresholds.rs` with constants and configuration
3. Update both old files to use new modules
4. Run tests (should pass)
5. Commit: "refactor: extract god object types and thresholds"

**Deliverable**: Compiling code, passing tests, reduced duplication

**Phase 3: Extract Pure Scoring Functions** (1 day)
1. Create `scoring.rs` with pure scoring algorithms
2. Move `calculate_god_object_score*` functions
3. Move purity and complexity weighting
4. Add unit tests for determinism
5. Update old files to import from `scoring`
6. Run tests
7. Commit: "refactor: extract pure scoring functions"

**Deliverable**: ~200 line `scoring.rs`, all tests pass

**Phase 4: Extract Pure Predicates** (0.5 day)
1. Create `predicates.rs` with detection predicates
2. Move `is_god_object`, threshold checks, etc.
3. Make all functions pure (take values, not refs to state)
4. Add unit tests
5. Commit: "refactor: extract detection predicates"

**Deliverable**: ~150 line `predicates.rs`, increased testability

**Phase 5: Extract Classification Logic** (1 day)
1. Create `classifier.rs` with classification functions
2. Move `classify_god_object`, type detection
3. Move responsibility grouping and inference
4. Ensure all are pure transformations
5. Add unit tests with property testing
6. Commit: "refactor: extract classification logic"

**Deliverable**: ~200 line `classifier.rs`, pure functions

**Phase 6: Extract Recommendation Logic** (1 day)
1. Create `recommender.rs` with recommendation generation
2. Move `suggest_module_splits*` functions
3. Move domain analysis for recommendations
4. Ensure pure (no I/O, just data transformation)
5. Add unit tests
6. Commit: "refactor: extract recommendation logic"

**Deliverable**: ~250 line `recommender.rs`, tested

**Phase 7: Create Orchestration Layer** (1 day)
1. Create `detector.rs` with orchestration
2. Move `GodObjectDetector` struct
3. Move `OrganizationDetector` trait implementation
4. Compose pure functions into analysis pipeline
5. Keep adapters for clustering integration
6. Add integration tests
7. Commit: "refactor: create god object detector orchestration"

**Deliverable**: ~250 line `detector.rs`, working end-to-end

**Phase 8: Update Public API & Cleanup** (0.5 day)
1. Update `mod.rs` with all re-exports
2. Mark old files as deprecated
3. Verify all 6 test files pass
4. Run full clippy check
5. Commit: "refactor: update god object public API"

**Deliverable**: Clean public API, all tests green

**Phase 9: Delete Old Files** (0.5 day)
1. Delete `god_object_detector.rs`
2. Delete `god_object_analysis.rs`
3. Audit `god_object_metrics.rs` - keep or merge into `types.rs`
4. Remove deprecated warnings
5. Run full test suite
6. Run benchmarks (verify < 5% regression)
7. Commit: "refactor: complete god object modularization (spec 181)"

**Deliverable**: Clean module structure, all tests pass, benchmarks good

**Total Estimated Time**: 6-7 days

### Module Responsibilities

**types.rs** (Foundation - Pure Data)
```rust
// Data structures from god_object_analysis.rs and god_object_detector.rs
pub struct GodObjectAnalysis { ... }
pub struct EnhancedGodObjectAnalysis { ... }
pub enum DetectionType { GodClass, GodFile, GodModule }
pub struct ModuleSplit { ... }
pub struct StructMetrics { ... }
pub enum GodObjectConfidence { ... }
pub struct PurityDistribution { ... }
pub struct FunctionVisibilityBreakdown { ... }
// etc.
```

**thresholds.rs** (Configuration - Pure Constants)
```rust
// Pure constants and configuration
pub struct GodObjectThresholds {
    pub method_count_threshold: usize,
    pub field_count_threshold: usize,
    // ...
}

pub const HYBRID_STANDALONE_THRESHOLD: usize = 50;
pub const HYBRID_DOMINANCE_RATIO: usize = 3;
// etc.
```

**predicates.rs** (Detection Predicates - Pure Functions)
```rust
// Pure boolean predicates
pub fn exceeds_method_threshold(count: usize, threshold: usize) -> bool
pub fn exceeds_field_threshold(count: usize, threshold: usize) -> bool
pub fn is_hybrid_god_module(standalone: usize, impl_methods: usize) -> bool
pub fn should_recommend_split(score: f64, confidence: GodObjectConfidence) -> bool
```

**scoring.rs** (Scoring Algorithms - Pure Math)
```rust
// Pure scoring calculations (from god_object_analysis.rs)
pub fn calculate_god_object_score(
    method_count: usize,
    responsibility_count: usize,
    thresholds: &GodObjectThresholds,
) -> f64

pub fn calculate_god_object_score_weighted(
    weighted_method_count: f64,
    responsibility_count: usize,
    thresholds: &GodObjectThresholds,
) -> f64

pub fn calculate_complexity_weight(complexity: u32) -> f64
pub fn calculate_purity_weight(purity: PurityLevel) -> f64
```

**classifier.rs** (Classification - Pure Transformations)
```rust
// Pure classification logic (from god_object_analysis.rs)
pub fn determine_confidence(
    score: f64,
    method_count: usize,
    responsibility_count: usize,
) -> GodObjectConfidence

pub fn group_methods_by_responsibility(
    methods: &[String]
) -> HashMap<String, Vec<String>>

pub fn infer_responsibility_with_confidence(
    method_name: &str
) -> ClassificationResult

pub fn classify_detection_type(
    struct_count: usize,
    standalone_count: usize,
    impl_method_count: usize,
) -> DetectionType
```

**recommender.rs** (Recommendations - Pure Generation)
```rust
// Pure recommendation generation (from god_object_detector.rs)
pub fn suggest_module_splits_by_domain(
    metrics: &[StructMetrics]
) -> Vec<ModuleSplit>

pub fn recommend_module_splits_enhanced(
    analysis: &GodObjectAnalysis,
    metrics: &[StructMetrics],
) -> Vec<ModuleSplit>

fn generate_split_rationale(split: &ModuleSplit) -> String
```

**detector.rs** (Orchestration - Composition)
```rust
// Orchestration layer (from god_object_detector.rs)
pub struct GodObjectDetector {
    thresholds: GodObjectThresholds,
}

impl GodObjectDetector {
    // Composes pure functions into analysis pipeline
    pub fn analyze(&self, visitor: &TypeVisitor) -> EnhancedGodObjectAnalysis {
        let metrics = build_per_struct_metrics(visitor);
        let score = scoring::calculate_god_object_score(...);
        let confidence = classifier::determine_confidence(...);
        let splits = recommender::suggest_module_splits_by_domain(&metrics);
        // ... compose results
    }
}

impl OrganizationDetector for GodObjectDetector {
    fn detect_anti_patterns(&self, file: &syn::File) -> Vec<OrganizationAntiPattern> {
        // Adapter: I/O → Pure Core → I/O
    }
}
```

**ast_visitor.rs** (I/O Shell - Preserved)
```rust
// AST traversal and data collection (EXISTING)
// This is the I/O boundary - not part of pure core
pub struct TypeVisitor { ... }
impl Visit for TypeVisitor { ... }
```

**mod.rs** (Public API - Re-exports)
```rust
// Re-exports for backward compatibility
pub use types::*;
pub use thresholds::GodObjectThresholds;
pub use scoring::{calculate_god_object_score, calculate_god_object_score_weighted};
pub use classifier::{determine_confidence, group_methods_by_responsibility};
pub use recommender::{suggest_module_splits_by_domain, recommend_module_splits_enhanced};
pub use detector::GodObjectDetector;
pub use ast_visitor::TypeVisitor;
```

### Architecture Changes

**Before (8,033 lines total):**
```
src/organization/
  god_object_detector.rs (4,362 lines) - mixed concerns
  god_object_analysis.rs (3,304 lines) - mixed concerns
  god_object_metrics.rs  (367 lines)   - tracking
  god_object/
    ast_visitor.rs (365 lines)         - I/O shell
    metrics.rs     (349 lines)         - calculations
    mod.rs         (15 lines)          - minimal exports
```

**After (estimated 3,000-3,500 lines total):**
```
src/organization/
  god_object/
    # Pure Core (business logic)
    types.rs       (~200 lines - data structures)
    thresholds.rs  (~100 lines - constants)
    predicates.rs  (~150 lines - detection predicates)
    scoring.rs     (~200 lines - scoring algorithms)
    classifier.rs  (~200 lines - classification logic)
    recommender.rs (~250 lines - recommendation generation)
    detector.rs    (~250 lines - orchestration)

    # I/O Shell (existing, preserved)
    ast_visitor.rs (365 lines - AST traversal)

    # Public API
    mod.rs         (~150 lines - re-exports, composition)

    # Optional (if metrics.rs needs splitting)
    metrics/
      calculations.rs (~200 lines - metric math)
      tracking.rs     (~150 lines - history tracking)
```

**Key Improvements:**
- 60% reduction in total lines (through deduplication and clarity)
- Clear separation: Pure Core (1,350 lines) vs I/O Shell (365 lines)
- Each module < 300 lines (most < 200)
- Acyclic dependencies
- 100% testable pure functions

### Data Structures (Moved to types.rs)

All existing types from `god_object_analysis.rs` and `god_object_detector.rs`:

```rust
// From god_object_analysis.rs
pub struct GodObjectAnalysis { ... }
pub struct EnhancedGodObjectAnalysis { ... }
pub enum DetectionType { GodClass, GodFile, GodModule }
pub struct ModuleSplit { ... }
pub struct StructMetrics { ... }
pub enum GodObjectConfidence { High, Medium, Low }
pub struct PurityDistribution { ... }
pub struct FunctionVisibilityBreakdown { ... }
pub enum Priority { Critical, High, Medium, Low }
pub enum SplitAnalysisMethod { ... }
pub enum RecommendationSeverity { ... }
pub struct GodObjectThresholds { ... }
pub struct ClassificationResult { ... }
pub enum SignalType { ... }

// From god_object_detector.rs
pub struct GodObjectClassificationParams<'a> { ... }
pub struct DomainAnalysisParams<'a> { ... }

// Adapters (may stay in detector.rs)
struct CallGraphAdapter { ... }
struct FieldAccessAdapter<'a> { ... }
```

**Stillwater Pattern**: Types are pure data with no behavior. Methods belong in modules, not on structs.

### APIs and Interfaces

**Public API (preserved in mod.rs) - Re-exported from current files:**

```rust
// From current mod.rs exports (lines 30-57)
pub use god_object_analysis::{
    calculate_god_object_score,
    calculate_god_object_score_weighted,
    determine_confidence,
    group_methods_by_responsibility,
    recommend_module_splits,
    recommend_module_splits_enhanced,
    suggest_module_splits_by_domain,
    DetectionType,
    EnhancedGodObjectAnalysis,
    GodObjectAnalysis,
    GodObjectConfidence,
    GodObjectThresholds,
    GodObjectType,
    ModuleSplit,
    Priority,
    // ... etc
};

pub use god_object::TypeVisitor;  // ast_visitor
pub use GodObjectDetector;        // detector
```

**After Refactoring - Same exports, different sources:**

```rust
pub use types::*;                 // All data structures
pub use thresholds::*;            // Thresholds and constants
pub use scoring::*;               // Scoring functions
pub use classifier::*;            // Classification functions
pub use recommender::*;           // Recommendation functions
pub use detector::GodObjectDetector;
pub use ast_visitor::TypeVisitor;
```

**Stillwater Pattern**: Public API unchanged, internal implementation reorganized

## Dependencies

### Prerequisites
- None (internal refactoring only)

### Affected Components
- `src/organization/mod.rs` - Re-exports from god_object module (lines 30-57)
- **Test files** (6 files, must continue to pass):
  - `tests/god_object_metrics_test.rs`
  - `tests/god_object_struct_recommendations.rs`
  - `tests/god_object_type_based_clustering_test.rs`
  - `tests/god_object_confidence_classification_test.rs`
  - `tests/god_object_detection_test.rs`
  - `tests/god_object_config_rs_test.rs`
- Any consumers using `use debtmap::organization::god_object_analysis::*`
- Any consumers using `use debtmap::organization::GodObjectDetector`

### External Dependencies
- No new dependencies required
- Uses existing: `syn`, `serde`, standard library collections
- May benefit from (optional):
  - `proptest` - for property-based testing of pure functions
  - `criterion` - for benchmarking

## Testing Strategy

### Stillwater Testing Principles

**Pure functions are 100% testable** - No mocks, no I/O, just inputs and outputs.

### Unit Tests (Per Module)

**predicates.rs** - Boolean logic tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exceeds_method_threshold_pure() {
        assert!(exceeds_method_threshold(20, 15));
        assert!(!exceeds_method_threshold(10, 15));
    }

    #[test]
    fn test_is_hybrid_god_module_ratios() {
        assert!(is_hybrid_god_module(60, 15));  // 60 > 15*3
        assert!(!is_hybrid_god_module(60, 25)); // 60 < 25*3
    }
}
```

**scoring.rs** - Deterministic math tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_scoring_deterministic() {
        let thresholds = GodObjectThresholds::default();
        let score1 = calculate_god_object_score(20, 5, &thresholds);
        let score2 = calculate_god_object_score(20, 5, &thresholds);
        assert_eq!(score1, score2);
    }

    proptest! {
        #[test]
        fn score_never_negative(method_count in 0..1000usize, resp_count in 0..100usize) {
            let thresholds = GodObjectThresholds::default();
            let score = calculate_god_object_score(method_count, resp_count, &thresholds);
            prop_assert!(score >= 0.0);
        }
    }
}
```

**classifier.rs** - Classification invariants
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_mapping() {
        assert_eq!(determine_confidence(2.5, 30, 6), GodObjectConfidence::High);
        assert_eq!(determine_confidence(1.2, 18, 3), GodObjectConfidence::Medium);
    }

    #[test]
    fn test_group_methods_pure() {
        let methods = vec!["get_user".to_string(), "set_user".to_string()];
        let groups = group_methods_by_responsibility(&methods);
        assert!(!groups.is_empty());
    }
}
```

### Integration Tests (Existing Tests Must Pass)

All 6 existing test files must pass without modification:

```rust
// tests/god_object_detection_test.rs
// tests/god_object_metrics_test.rs
// etc.
// Should work unchanged due to backward-compatible API
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn scoring_monotonic_in_method_count(
        base_methods in 10..100usize,
        delta in 1..50usize
    ) {
        let thresholds = GodObjectThresholds::default();
        let score1 = calculate_god_object_score(base_methods, 5, &thresholds);
        let score2 = calculate_god_object_score(base_methods + delta, 5, &thresholds);
        prop_assert!(score2 >= score1); // More methods = higher score
    }

    #[test]
    fn classification_idempotent(method_name: String) {
        let result1 = infer_responsibility_with_confidence(&method_name);
        let result2 = infer_responsibility_with_confidence(&method_name);
        prop_assert_eq!(result1.category, result2.category);
    }
}
```

### Benchmark Tests

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_god_object_scoring(c: &mut Criterion) {
    let thresholds = GodObjectThresholds::default();
    c.bench_function("calculate_god_object_score", |b| {
        b.iter(|| {
            calculate_god_object_score(
                black_box(25),
                black_box(6),
                black_box(&thresholds)
            )
        })
    });
}

criterion_group!(benches, bench_god_object_scoring);
criterion_main!(benches);
```

## Documentation Requirements

### Module-Level Documentation (Required for Each Module)

Following **Stillwater principles**: Document what makes it pure and composable.

```rust
// scoring.rs
//! # God Object Scoring (Pure Core)
//!
//! Pure functions for calculating god object scores and weights.
//! All functions are:
//! - **Deterministic**: Same inputs always produce same outputs
//! - **Side-effect free**: No I/O, no mutations, no hidden state
//! - **Composable**: Can be combined into pipelines
//!
//! ## Stillwater Architecture
//!
//! This is part of the **Pure Core** - business logic separated from I/O.
//! AST traversal (I/O) happens in `ast_visitor.rs`.
//!
//! ## Examples
//!
//! ```rust
//! use debtmap::organization::god_object::scoring::*;
//!
//! let thresholds = GodObjectThresholds::default();
//! let score = calculate_god_object_score(25, 6, &thresholds);
//! assert!(score > 1.0); // Exceeds threshold
//! ```

// classifier.rs
//! # God Object Classification (Pure Transformations)
//!
//! Pure functions for classifying god objects and grouping methods.
//!
//! ## Purity Guarantees
//!
//! - Input: Method names, metrics, scores
//! - Output: Classifications, confidence levels, groupings
//! - No I/O, no external state
//!
//! ## Examples
//!
//! ```rust
//! let methods = vec!["get_user", "set_user", "fetch_orders"];
//! let groups = group_methods_by_responsibility(&methods);
//! // Returns: {"data_access": ["get_user", "set_user"], ...}
//! ```
```

### Function Documentation (Every Public Function)

```rust
/// Calculate god object score based on method and responsibility counts.
///
/// **Pure function** - deterministic, no side effects.
///
/// # Arguments
///
/// * `method_count` - Number of methods in the type
/// * `responsibility_count` - Number of distinct responsibilities
/// * `thresholds` - Configuration thresholds
///
/// # Returns
///
/// Score as f64, where:
/// - < 1.0: Not a god object
/// - 1.0-2.0: Moderate god object
/// - > 2.0: Severe god object
///
/// # Examples
///
/// ```rust
/// let thresholds = GodObjectThresholds::default();
/// let score = calculate_god_object_score(30, 7, &thresholds);
/// assert!(score > 1.5);
/// ```
///
/// # Purity
///
/// This function is pure:
/// - Same inputs always produce same output
/// - No I/O operations
/// - No mutations of input parameters
/// - Testable without mocks
pub fn calculate_god_object_score(
    method_count: usize,
    responsibility_count: usize,
    thresholds: &GodObjectThresholds,
) -> f64 {
    // ...
}
```

### Architecture Documentation

Update `ARCHITECTURE.md` or `CLAUDE.md`:

```markdown
## God Object Detection Module (Stillwater Architecture)

Location: `src/organization/god_object/`

### Architecture Pattern: Pure Core, Imperative Shell

**Pure Core (Business Logic)**:
- `types.rs` - Data structures
- `thresholds.rs` - Configuration
- `predicates.rs` - Detection predicates
- `scoring.rs` - Scoring algorithms
- `classifier.rs` - Classification logic
- `recommender.rs` - Recommendation generation

**Imperative Shell (I/O Boundary)**:
- `ast_visitor.rs` - AST traversal and data collection
- `detector.rs` - Orchestration (composes pure functions)

**Public API**:
- `mod.rs` - Re-exports for consumers

### Dependency Flow

```
ast_visitor.rs (I/O) ──→ [data] ──→ Pure Core ──→ [results] ──→ consumers

Pure Core hierarchy:
  types.rs (foundation)
    ↑
    ├── thresholds.rs
    ├── predicates.rs
    ├── scoring.rs
    │     ↑
    ├── classifier.rs
    │     ↑
    └── recommender.rs
```

### Testing Strategy

All pure functions have 100% unit test coverage without mocks.
Property-based tests verify invariants.
Integration tests verify composition.
```

## Implementation Notes

### Stillwater Refactoring Strategy

**Core Principle**: Incremental extraction of pure functions, preserving I/O boundaries.

1. **Identify Pure vs Impure**
   - Pure: Takes data, returns data, no side effects
   - Impure: Performs I/O, mutates state, talks to external systems

2. **Extract Pure Functions First**
   - Start with scoring (easiest - pure math)
   - Then predicates (simple boolean logic)
   - Then classification (pure transformations)
   - Finally orchestration (composes pure functions)

3. **Preserve I/O Shell**
   - `ast_visitor.rs` stays as-is (it's already properly separated)
   - Don't try to make I/O "pure" - keep it at boundaries

4. **Test at Every Step**
   - Extract module → write unit tests → verify integration tests pass
   - Commit small, working changes
   - Never leave codebase in non-compiling state

### Common Pitfalls

**Circular Dependencies**
```rust
// ❌ BAD: Circular dependency
// scoring.rs imports classifier
// classifier.rs imports scoring
// Solution: Extract shared types to types.rs

// ✅ GOOD: Linear dependency
types.rs ← scoring.rs ← classifier.rs
```

**Forgetting to Export from mod.rs**
```rust
// ❌ BAD: Internal module not re-exported
// Users can't access new functions

// ✅ GOOD: Re-export in mod.rs
pub use scoring::calculate_god_object_score;
```

**Accidentally Making Pure Functions Impure**
```rust
// ❌ BAD: Added logging (side effect!)
fn calculate_score(count: usize) -> f64 {
    println!("Calculating score for {}", count); // I/O!
    count as f64 * 1.5
}

// ✅ GOOD: Pure, caller can log
fn calculate_score(count: usize) -> f64 {
    count as f64 * 1.5
}
```

### Pure Function Extraction Guide

**Stillwater Pattern**: Separate "what to compute" from "how to execute".

```rust
// ❌ BEFORE: Mixed I/O and logic
fn analyze_file_bad(path: &Path) -> Result<GodObjectAnalysis> {
    let content = std::fs::read_to_string(path)?;  // I/O
    let ast = syn::parse_file(&content)?;          // I/O
    let mut visitor = TypeVisitor::new();
    visitor.visit_file(&ast);                      // I/O

    let score = calculate_score(&visitor);         // Pure logic
    let confidence = determine_confidence(&score); // Pure logic

    Ok(GodObjectAnalysis { score, confidence })
}

// ✅ AFTER: Stillwater separation
// I/O Shell (ast_visitor.rs)
pub fn collect_type_data(file: &syn::File) -> TypeVisitor {
    let mut visitor = TypeVisitor::new();
    visitor.visit_file(file);
    visitor
}

// Pure Core (scoring.rs)
pub fn calculate_score(visitor: &TypeVisitor) -> f64 {
    let method_count = visitor.total_method_count();
    let responsibility_count = visitor.responsibility_count();
    (method_count as f64 / 15.0) * (responsibility_count as f64 / 3.0)
}

// Pure Core (classifier.rs)
pub fn determine_confidence(score: f64) -> GodObjectConfidence {
    if score > 2.0 { High } else if score > 1.0 { Medium } else { Low }
}

// Orchestration (detector.rs)
impl GodObjectDetector {
    pub fn analyze(&self, visitor: &TypeVisitor) -> GodObjectAnalysis {
        // Compose pure functions
        let score = scoring::calculate_score(visitor);
        let confidence = classifier::determine_confidence(score);
        GodObjectAnalysis { score, confidence }
    }
}
```

**Benefits**:
- `calculate_score` and `determine_confidence` are 100% testable
- No mocks needed for testing
- Can refactor without breaking I/O
- Clear boundaries

## Migration and Compatibility

### Breaking Changes

**None** - Public API remains 100% backward compatible.

### Migration for Consumers

**No action required** - All imports continue to work:

```rust
// These all continue to work unchanged
use debtmap::organization::god_object_analysis::*;
use debtmap::organization::GodObjectDetector;
use debtmap::organization::{
    calculate_god_object_score,
    determine_confidence,
    // ... etc
};
```

### Internal Migration (For Contributors)

If directly importing from old files (not recommended but possible):

```rust
// Before (internal use)
use crate::organization::god_object_detector::GodObjectDetector;
use crate::organization::god_object_analysis::calculate_god_object_score;

// After (updated paths)
use crate::organization::god_object::detector::GodObjectDetector;
use crate::organization::god_object::scoring::calculate_god_object_score;

// But better: Use from mod.rs
use crate::organization::god_object::{GodObjectDetector, calculate_god_object_score};
```

### Rollback Plan

Each phase is independently committable. If issues arise:

1. **Phase-level rollback**: Revert last commit, fix issue, re-apply
2. **Full rollback**: Keep old files until Phase 9, can abort at any time
3. **Incremental fixes**: New modules coexist with old until Phase 9

### Compatibility Guarantees

- All 6 test files pass without modification
- Public API signatures unchanged
- Return types unchanged
- Error handling unchanged
- Performance characteristics maintained

## Success Metrics

### Quantitative Metrics

- ✅ **Line Count**: 8,033 → ~3,500 lines (60% reduction)
- ✅ **Module Count**: 3 files → 9 focused modules
- ✅ **Module Size**: All < 300 lines (target: < 200)
- ✅ **Pure Functions**: 90%+ of logic in pure functions
- ✅ **Test Coverage**: 85%+ for pure functions
- ✅ **Performance**: < 5% regression on benchmarks
- ✅ **Build Time**: No increase

### Qualitative Metrics

- ✅ **Stillwater Compliance**: Clear pure core / I/O shell separation
- ✅ **Testability**: Pure functions testable without mocks
- ✅ **Maintainability**: Single responsibility per module
- ✅ **Clarity**: Each module's purpose obvious from name
- ✅ **Composition**: Functions easily composable into pipelines
- ✅ **Documentation**: Every public function documented with purity guarantees

### Verification Steps

1. **All tests pass**
   ```bash
   cargo test --all-features
   # All 6 god_object test files pass
   ```

2. **No clippy warnings**
   ```bash
   cargo clippy --all-targets --all-features -- -D warnings
   ```

3. **Benchmarks acceptable**
   ```bash
   cargo bench --bench god_object_bench
   # < 5% regression vs baseline
   ```

4. **Documentation builds**
   ```bash
   cargo doc --no-deps --document-private-items
   ```

5. **Dependency graph acyclic**
   ```bash
   cargo depgraph --focus organization::god_object
   # No cycles in dependency graph
   ```

## References

- **`../stillwater/PHILOSOPHY.md`** - Core architectural principles
- **`CLAUDE.md`** - Project-specific guidelines
- **`STILLWATER_EVALUATION.md`** - Original evaluation identifying this issue
- **Functional Core, Imperative Shell** - Gary Bernhardt
- **Parse, Don't Validate** - Alexis King
