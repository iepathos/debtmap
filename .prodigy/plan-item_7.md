# Implementation Plan: Decompose analyze_comprehensive into Focused Functions

## Problem Summary

**Location**: ./src/organization/god_object_detector.rs:GodObjectDetector::analyze_comprehensive:490
**Priority Score**: 16.77
**Debt Type**: ComplexityHotspot (Cognitive: 78, Cyclomatic: 34)
**Current Metrics**:
- Function Length: 315 lines
- Cyclomatic Complexity: 34
- Cognitive Complexity: 78
- Function Role: Pure Logic
- Nesting Depth: 3

**Issue**: High complexity (34/78) makes function hard to test and maintain. This pure logic function should be decomposed into smaller, focused functions that each handle a single responsibility.

## Target State

**Expected Impact** (from debtmap):
- Complexity Reduction: 17.0 (target cyclomatic ~10-15)
- Coverage Improvement: 0.0 (already testable as pure function)
- Risk Reduction: 5.87

**Success Criteria**:
- [ ] Cyclomatic complexity reduced from 34 to ≤15
- [ ] Cognitive complexity reduced from 78 to ≤40
- [ ] Function broken into 4-5 focused sub-functions
- [ ] Each extracted function has single responsibility
- [ ] All existing tests continue to pass
- [ ] No clippy warnings
- [ ] Proper formatting

## Implementation Phases

### Phase 1: Extract God Object Type Detection Logic

**Goal**: Extract the complex conditional logic that determines whether we have a God Class vs God File into a dedicated function.

**Changes**:
- Extract lines 512-558 into `determine_god_object_type()` function
- This function takes visitor data and returns `(total_methods, total_fields, all_methods, total_complexity, detection_type)`
- Reduce nesting by separating the decision logic
- Make the function pure and independently testable

**Rationale**: This block has high cognitive complexity with nested conditionals and filtering logic. Extracting it will reduce the main function's cyclomatic complexity by ~8 points.

**Testing**:
```bash
cargo test god_object_detector
cargo clippy -- -D warnings
```

**Success Criteria**:
- [ ] New `determine_god_object_type()` function created
- [ ] Function is pure (no side effects)
- [ ] Main function cyclomatic complexity reduced by 6-8 points
- [ ] All tests pass
- [ ] Ready to commit

### Phase 2: Extract Complexity Weighting Logic

**Goal**: Extract the complexity-weighted metrics calculation into a focused function.

**Changes**:
- Extract lines 583-630 into `calculate_weighted_metrics()` function
- This function handles filtering by detection type and calculating purity weights
- Returns `(weighted_method_count, avg_complexity, purity_weighted_count, purity_distribution)`
- Simplifies the conditional logic for purity weighting

**Rationale**: This section has multiple nested conditionals based on detection type and involves complex filtering. Extracting it reduces cognitive load and makes the weighting logic testable.

**Testing**:
```bash
cargo test god_object_detector
cargo clippy -- -D warnings
```

**Success Criteria**:
- [ ] New `calculate_weighted_metrics()` function created
- [ ] Cyclomatic complexity reduced by 5-7 points
- [ ] Function handles both GodClass and GodFile cases
- [ ] All tests pass
- [ ] Ready to commit

### Phase 3: Extract God Object Scoring Logic

**Goal**: Consolidate the three different scoring code paths into a single focused function.

**Changes**:
- Extract lines 632-672 into `calculate_final_god_object_score()` function
- This function takes all metrics and thresholds
- Returns `(god_object_score, is_god_object)`
- Eliminates the three-way conditional (purity vs weighted vs raw)

**Rationale**: The scoring logic has three different code paths with similar structure. Consolidating into one function with clear parameter handling reduces duplication and complexity.

**Testing**:
```bash
cargo test god_object_detector
cargo clippy -- -D warnings
```

**Success Criteria**:
- [ ] New `calculate_final_god_object_score()` function created
- [ ] Cyclomatic complexity reduced by 4-6 points
- [ ] Scoring logic is clear and testable
- [ ] All tests pass
- [ ] Ready to commit

### Phase 4: Extract Domain Analysis and Recommendations

**Goal**: Extract the cross-domain analysis and recommendation generation into a dedicated function.

**Changes**:
- Extract lines 674-735 into `analyze_domains_and_recommend_splits()` function
- This function handles domain counting, cross-domain severity, and split recommendations
- Returns `(recommended_splits, analysis_method, cross_domain_severity, domain_count, domain_diversity, struct_ratio)`
- Simplifies the priority-based recommendation logic

**Rationale**: This section handles a complete sub-concern (domain analysis) and has complex nested conditionals. Extracting it makes the main function focus on orchestration.

**Testing**:
```bash
cargo test god_object_detector
cargo clippy -- -D warnings
```

**Success Criteria**:
- [ ] New `analyze_domains_and_recommend_splits()` function created
- [ ] Cyclomatic complexity reduced by 5-7 points
- [ ] Domain analysis logic is isolated and testable
- [ ] All tests pass
- [ ] Ready to commit

### Phase 5: Extract Module Structure and Visibility Analysis

**Goal**: Extract the final section that handles module structure and visibility breakdown.

**Changes**:
- Extract lines 739-770 into `analyze_module_structure_and_visibility()` function
- This function handles Rust-specific visibility and module structure analysis
- Returns `(visibility_breakdown, module_structure)`
- Separates the Rust-specific logic from the general analysis

**Rationale**: This is the final complex section with nested conditionals. Extracting it completes the decomposition and reduces the main function to pure orchestration.

**Testing**:
```bash
cargo test god_object_detector
cargo clippy -- -D warnings
```

**Success Criteria**:
- [ ] New `analyze_module_structure_and_visibility()` function created
- [ ] Main function now ~50-80 lines of pure orchestration
- [ ] Cyclomatic complexity ≤15, cognitive ≤40
- [ ] All tests pass
- [ ] Ready to commit

## Testing Strategy

**For each phase**:
1. Run `cargo test --lib god_object_detector` to verify existing tests pass
2. Run `cargo clippy -- -D warnings` to ensure no new warnings
3. Verify function signatures are correct and types align
4. Check that the main function's complexity is decreasing

**Final verification**:
1. `just ci` - Full CI checks
2. `cargo test --all` - All tests across codebase
3. Verify cyclomatic complexity ≤15 (use debtmap or manual inspection)
4. Verify cognitive complexity ≤40

**Tracking Progress**:
After each phase, document the complexity reduction:
```bash
# Check complexity metrics
cargo clippy -- -W clippy::cognitive_complexity
```

## Rollback Plan

If a phase fails:
1. Revert the phase with `git reset --hard HEAD~1`
2. Review the compilation or test errors
3. Check if function signatures need adjustment
4. Retry with corrected implementation

If multiple phases fail:
1. Return to last known good commit
2. Re-evaluate the extraction strategy
3. Consider smaller extraction steps

## Function Signatures (Proposed)

```rust
// Phase 1
fn determine_god_object_type(
    visitor: &TypeVisitor,
    standalone_count: usize,
) -> (usize, usize, Vec<String>, u32, DetectionType);

// Phase 2
fn calculate_weighted_metrics(
    visitor: &TypeVisitor,
    detection_type: DetectionType,
    relevant_complexity: &[FunctionComplexityInfo],
) -> (f64, f64, f64, Option<PurityDistribution>);

// Phase 3
fn calculate_final_god_object_score(
    purity_weighted_count: f64,
    weighted_method_count: f64,
    total_methods: usize,
    total_fields: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    avg_complexity: f64,
    purity_distribution: Option<PurityDistribution>,
    has_complexity_data: bool,
    thresholds: &GodObjectThresholds,
) -> (f64, bool);

// Phase 4
fn analyze_domains_and_recommend_splits(
    per_struct_metrics: &[StructMetrics],
    total_methods: usize,
    lines_of_code: usize,
    is_god_object: bool,
    path: &Path,
    all_methods: &[String],
    responsibility_groups: &HashMap<String, Vec<String>>,
) -> (Vec<ModuleSplit>, SplitAnalysisMethod, Option<CrossDomainSeverity>, usize, f64, f64);

// Phase 5
fn analyze_module_structure_and_visibility(
    path: &Path,
    is_god_object: bool,
    visitor: &TypeVisitor,
    all_methods: &[String],
    total_methods: usize,
    source_content: &Option<String>,
) -> (Option<FunctionVisibilityBreakdown>, Option<ModuleStructure>);
```

## Notes

- The main function `analyze_comprehensive` will be reduced to ~50-80 lines of orchestration
- Each extracted function is pure and independently testable
- The extraction follows the natural sections of the current implementation
- No behavior changes - purely structural refactoring
- Function parameters may need adjustment during implementation to handle ownership/borrowing correctly
- Consider using references vs cloning based on actual usage patterns
- The `TypeVisitor` is already computed, so passing it to helper functions is efficient
