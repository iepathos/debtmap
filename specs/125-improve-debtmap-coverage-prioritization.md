---
number: 125
title: Improve Debtmap Coverage-Driven Prioritization Strategy
category: optimization
priority: medium
status: draft
dependencies: [122]
created: 2025-10-05
---

# Specification 125: Improve Debtmap Coverage-Driven Prioritization Strategy

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: [Spec 122 - Fix Coverage Scoring Inversion]

## Context

When coverage data is enabled, debtmap's top 10 recommendations become flooded with untested functions that have nearly identical scores (76-77 range). This "wall of similar-scored items" obscures the real architectural problems (God Objects, God Modules) that should be top priority.

### Current Behavior (With Coverage)

```
🎯 TOP 10 RECOMMENDATIONS

#1 SCORE: 97.9 [God Object] shared_cache.rs
#2 SCORE: 97.7 [God Module] enhanced_markdown/mod.rs
#3 SCORE: 77.1 [🔴 UNTESTED] detect_react_test_issues()
#4 SCORE: 76.8 [🔴 UNTESTED] copy_dir_recursive()
#5 SCORE: 76.6 [🔴 UNTESTED] detect_missing_assertions()
#6 SCORE: 76.6 [🔴 UNTESTED] detect_complex_tests()
#7 SCORE: 76.6 [🔴 UNTESTED] detect_timing_dependent_tests()
#8 SCORE: 76.6 [🔴 UNTESTED] detect_async_test_issues()
#9 SCORE: 76.5 [Low Coverage] write_priority_section()
#10 SCORE: 76.3 [🔴 UNTESTED] perform_validation()
```

**Problems:**
1. Items #3-#10 are all untested functions with complexity 7-11 (moderate, not critical)
2. All have nearly identical scores (76.3-77.1), making prioritization meaningless
3. Architectural debt (God Objects) is buried in untested-function noise
4. Actionability is low: which untested function should I tackle first?

### Desired Behavior

```
🎯 TOP 10 RECOMMENDATIONS

#1 SCORE: 165 [God Object] shared_cache.rs
   ↳ 2529 lines, 129 functions, 7 responsibilities
   ↳ Coverage: 45% - many complex functions untested

#2 SCORE: 118 [God Object] debt_item.rs
   ↳ 2972 lines, 124 functions, 5 responsibilities
   ↳ Coverage: 38% - scoring logic needs tests

#3 SCORE: 116 [God Module] enhanced_markdown/mod.rs
   ↳ 1634 lines, 82 functions, violates cohesion
   ↳ Coverage: 62% - formatting logic partially tested

#4 SCORE: 52 [Complex + Untested] FileDebtMetrics::generate_recommendation()
   ↳ Complexity: 17, 0% coverage, 6 callers
   ↳ Critical business logic requires tests

#5 SCORE: 35 [Complex + Untested] format_priority_item_markdown()
   ↳ Complexity: 10, 0% coverage, 10 callers
   ↳ Public API needs comprehensive tests

... (similar pattern, only showing high-impact items)
```

**Improvements:**
1. Architectural debt maintains top 3 positions
2. Coverage shown as context, not as separate high-priority item
3. Untested functions only surface if they're also complex (≥10) or high-dependency
4. Clear actionability: fix God Objects first, then complex untested functions

## Objective

Redesign prioritization strategy to:
1. Surface architectural issues (God Objects, God Modules) as top priority regardless of coverage
2. Show coverage data as **context** within existing debt items, not as standalone issues
3. Only surface untested functions if they meet complexity/dependency thresholds
4. Provide tiered recommendations with clear action hierarchy

## Requirements

### Functional Requirements

1. **Priority Tiers**: Group recommendations into explicit tiers:
   - **Tier 1 (Critical Architecture)**: God Objects, God Modules, excessive complexity
   - **Tier 2 (Complex Untested)**: Untested functions with complexity ≥ 15 or dependencies ≥ 10
   - **Tier 3 (Testing Gaps)**: Untested functions with moderate complexity (10-14)
   - **Tier 4 (Maintenance)**: Low-complexity untested functions, minor refactorings

2. **Coverage as Context**: Show coverage within items, not as separate recommendations:
   - "God Object: shared_cache.rs (45% coverage - many complex functions untested)"
   - Include untested function list within God Object recommendation

3. **Threshold-Based Surfacing**: Only create standalone recommendations for untested functions if:
   - Cyclomatic complexity ≥ 15, OR
   - Total dependencies (callers + callees) ≥ 10, OR
   - Public API function, OR
   - Entry point (main, framework callbacks)

4. **Implement Category Filtering**: The existing `--filter` flag should filter recommendations by category (Architecture, Testing, Performance, CodeQuality)

### Non-Functional Requirements

1. **Clarity**: Users should immediately understand what's most important
2. **Actionability**: Each recommendation should have clear next steps
3. **Consistency**: Priority ordering should be deterministic and explainable
4. **Flexibility**: Users can customize tier thresholds via configuration

## Acceptance Criteria

- [ ] Top 3 recommendations are architectural issues (God Objects, God Modules) when present
- [ ] Coverage data appears as context within debt items, not as separate items
- [ ] Untested functions only surface in top 10 if complexity ≥ 15 or dependencies ≥ 10
- [ ] `--filter Testing` shows only Testing category items (testing gaps, test quality issues)
- [ ] `--filter Architecture` shows only Architecture category items (God Objects, etc.)
- [ ] `--filter` accepts comma-separated categories: `--filter Architecture,Testing`
- [ ] Tier labels (Tier 1-4) shown clearly in output
- [ ] Configuration allows customizing tier thresholds
- [ ] All existing tests pass without modification
- [ ] User documentation explains new prioritization strategy

## Technical Details

### Current Prioritization (Flat Scoring)

```rust
// Everyone gets same treatment, sorted by single score
pub fn prioritize_debt(items: &[DebtItem]) -> Vec<DebtItem> {
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    sorted
}
```

**Problem**: No concept of issue type or tier, just a flat score.

### Proposed Prioritization (Tiered Strategy)

```rust
pub enum RecommendationTier {
    T1CriticalArchitecture,  // God Objects, God Modules
    T2ComplexUntested,        // Untested + high complexity/dependencies
    T3TestingGaps,            // Untested + moderate complexity
    T4Maintenance,            // Everything else
}

pub struct TieredRecommendation {
    pub tier: RecommendationTier,
    pub primary_debt: DebtItem,
    pub coverage_context: Option<CoverageContext>,
    pub related_items: Vec<DebtItem>,  // Untested functions within God Object
}

pub struct CoverageContext {
    pub coverage_percent: f64,
    pub untested_functions: Vec<FunctionInfo>,
    pub critical_gaps: Vec<TestingGap>,
}

pub fn prioritize_tiered(items: &[DebtItem], config: &TierConfig) -> Vec<TieredRecommendation> {
    // Step 1: Classify items by tier
    let mut t1_items = items.iter().filter(|i| i.is_architectural_issue()).collect();
    let mut t2_items = items.iter().filter(|i| i.is_complex_untested(config)).collect();
    let mut t3_items = items.iter().filter(|i| i.is_moderate_untested(config)).collect();
    let mut t4_items = items.iter().filter(|i| true).collect();  // Everything else

    // Step 2: Sort within each tier by score
    t1_items.sort_by_score();
    t2_items.sort_by_score();
    t3_items.sort_by_score();
    t4_items.sort_by_score();

    // Step 3: Attach coverage context to architectural items
    for item in &mut t1_items {
        if let Some(file_debt) = item.as_file_debt() {
            item.coverage_context = Some(CoverageContext::for_file(file_debt));
        }
    }

    // Step 4: Build tiered recommendations
    let mut recommendations = Vec::new();
    recommendations.extend(t1_items.into_iter().map(|i| TieredRecommendation::new(T1, i)));
    recommendations.extend(t2_items.into_iter().map(|i| TieredRecommendation::new(T2, i)));
    recommendations.extend(t3_items.into_iter().map(|i| TieredRecommendation::new(T3, i)));
    // T4 items only shown in verbose mode or separate report

    recommendations
}
```

### Implementation Strategy

1. **Phase 1: Implement Tier Classification (Spec 122 prerequisite)**
   - Add `is_architectural_issue()` method to `DebtItem`
   - Add `is_complex_untested()` with configurable thresholds
   - Add `is_moderate_untested()` for tier 3 classification

2. **Phase 2: Build Coverage Context**
   - Create `CoverageContext` struct to hold coverage metadata
   - Extract untested functions from file-level debt items
   - Identify "critical gaps" (untested public APIs, entry points)

3. **Phase 3: Tiered Prioritization**
   - Implement tier-based sorting (T1 → T2 → T3 → T4)
   - Within each tier, sort by score descending
   - Attach coverage context to appropriate items

4. **Phase 4: Implement Category Filtering**
   - Implement filtering logic for `--filter` flag (currently defined but not implemented)
   - Support single category: `--filter Testing`
   - Support multiple categories: `--filter Architecture,Testing`
   - Filter recommendations before output formatting

5. **Phase 5: Update Output Formatting**
   - Show tier labels in markdown output
   - Display coverage context within architectural items
   - Ensure filtered output shows category-specific context

6. **Phase 6: Configuration Support**
   - Add tier threshold configuration options
   - Allow customizing tier weights and rules
   - Provide presets (strict, balanced, lenient)

### File Changes Required

**New Files:**
- `src/priority/tiers.rs`: Tier classification logic
- `src/priority/coverage_context.rs`: Coverage context building
- `src/priority/filter.rs`: Category filtering implementation

**Modified Files:**
- `src/priority/scoring/debt_item.rs`: Add tier classification methods
- `src/priority/formatter.rs`: Show tier labels and coverage context
- `src/config.rs`: Add tier configuration options
- `src/commands/analyze.rs`: Implement category filtering logic for existing `--filter` flag
- `src/priority/mod.rs`: Apply filters before output

### Data Structures

```rust
pub enum RecommendationTier {
    T1CriticalArchitecture { reason: ArchitectureIssue },
    T2ComplexUntested { complexity: usize, dependencies: usize },
    T3TestingGaps { coverage_percent: f64 },
    T4Maintenance,
}

pub struct TierConfig {
    pub t2_complexity_threshold: usize,      // Default: 15
    pub t2_dependency_threshold: usize,      // Default: 10
    pub t3_complexity_threshold: usize,      // Default: 10
    pub show_t4_in_main_report: bool,        // Default: false
}

pub struct CoverageContext {
    pub file_coverage_percent: f64,
    pub untested_count: usize,
    pub untested_functions: Vec<UntestedFunction>,
    pub critical_gaps: Vec<TestingGap>,
}

pub struct UntestedFunction {
    pub name: String,
    pub complexity: usize,
    pub location: SourceLocation,
    pub is_public: bool,
    pub is_entry_point: bool,
}

pub struct TestingGap {
    pub severity: GapSeverity,
    pub description: String,
    pub affected_functions: Vec<String>,
}

pub enum GapSeverity {
    Critical,   // Public API, entry point
    High,       // Complex business logic
    Medium,     // Moderate complexity
    Low,        // Simple utility functions
}
```

## Dependencies

**Prerequisites**:
- [Spec 122] Fix Coverage Scoring Inversion (must complete first)

**Affected Components**:
- Priority scoring system
- Output formatting (markdown, JSON)
- CLI argument parsing
- Configuration system

**External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_tier_classification_god_object() {
    let item = create_god_object_debt_item(methods: 100, responsibilities: 5);
    assert_eq!(item.classify_tier(), RecommendationTier::T1CriticalArchitecture);
}

#[test]
fn test_tier_classification_complex_untested() {
    let item = create_function_debt_item(complexity: 20, coverage: 0.0);
    let config = TierConfig::default();
    assert!(item.is_complex_untested(&config));
    assert_eq!(item.classify_tier(), RecommendationTier::T2ComplexUntested);
}

#[test]
fn test_tier_classification_simple_untested_filtered() {
    let item = create_function_debt_item(complexity: 5, coverage: 0.0);
    let config = TierConfig::default();
    assert!(!item.is_complex_untested(&config));
    assert_eq!(item.classify_tier(), RecommendationTier::T4Maintenance);
}

#[test]
fn test_coverage_context_extraction() {
    let file_debt = create_file_debt_with_untested_functions();
    let context = CoverageContext::for_file(&file_debt);

    assert_eq!(context.file_coverage_percent, 45.0);
    assert_eq!(context.untested_count, 12);
    assert_eq!(context.critical_gaps.len(), 3);
}
```

### Integration Tests

```rust
#[test]
fn test_tiered_prioritization_architecture_first() {
    let items = vec![
        create_god_object_item(score: 95.0),
        create_untested_function_item(complexity: 10, score: 76.0),
        create_god_module_item(score: 90.0),
    ];

    let recommendations = prioritize_tiered(&items, &TierConfig::default());

    assert_eq!(recommendations[0].tier, RecommendationTier::T1CriticalArchitecture);
    assert_eq!(recommendations[1].tier, RecommendationTier::T1CriticalArchitecture);
    assert!(recommendations[2].tier != RecommendationTier::T1CriticalArchitecture);
}

#[test]
fn test_filter_by_category_testing() {
    let output = run_command("debtmap analyze . --filter Testing");

    assert!(output.contains("Testing Gaps"));
    assert!(!output.contains("God Object"));  // Architecture items filtered out
    assert!(!output.contains("Performance"));  // Performance items filtered out
}

#[test]
fn test_filter_multiple_categories() {
    let output = run_command("debtmap analyze . --filter Architecture,Testing");

    assert!(output.contains("God Object") || output.contains("Architecture"));
    assert!(output.contains("Testing Gaps") || output.contains("Testing"));
    assert!(!output.contains("Performance"));  // Performance items filtered out
}
```

### User Acceptance Tests

1. Analyze project without coverage → see architectural issues in top 3
2. Analyze project with coverage → still see architectural issues in top 3
3. Run `--filter Testing` → see only testing-related debt items
4. Run `--filter Architecture,CodeQuality` → see only those categories
5. Customize tier thresholds in config → see different prioritization

## Documentation Requirements

### Code Documentation

```rust
/// Classifies debt items into priority tiers for strategic remediation.
///
/// # Tier Hierarchy
///
/// 1. **Tier 1 (Critical Architecture)**: God Objects, God Modules, excessive complexity
///    - Must address before adding new features
///    - High impact on maintainability
///
/// 2. **Tier 2 (Complex Untested)**: Untested code with high complexity or dependencies
///    - Risk of bugs in critical paths
///    - Should be tested before refactoring
///
/// 3. **Tier 3 (Testing Gaps)**: Untested code with moderate complexity
///    - Improve coverage to prevent future issues
///    - Lower priority than architectural debt
///
/// 4. **Tier 4 (Maintenance)**: Low-complexity issues
///    - Address opportunistically
///    - Minimal impact on system health
pub fn prioritize_tiered(items: &[DebtItem], config: &TierConfig) -> Vec<TieredRecommendation>
```

### User Documentation

Add prioritization guide to debtmap docs:

```markdown
## Understanding Recommendation Tiers

Debtmap uses a tiered prioritization strategy:

### Tier 1: Critical Architecture 🔴
- God Objects (100+ methods, 5+ responsibilities)
- God Modules (50+ functions in single file)
- Excessive complexity (cyclomatic > 50)

**Action**: Refactor before adding features

### Tier 2: Complex Untested Code ⚠️
- Untested functions with complexity ≥ 15
- Untested code with ≥ 10 dependencies
- Public APIs without tests

**Action**: Add tests first, then refactor if needed

### Tier 3: Testing Gaps 🟡
- Untested functions with moderate complexity (10-14)
- Partial coverage of complex modules

**Action**: Improve coverage to prevent future issues

### Tier 4: Maintenance 🟢
- Simple untested utilities
- Minor refactoring opportunities

**Action**: Address opportunistically

### Filtering by Category

Focus on specific types of debt using the `--filter` flag:

```bash
# Show only testing gaps
debtmap analyze . --filter Testing

# Show only architectural issues
debtmap analyze . --filter Architecture

# Show multiple categories
debtmap analyze . --filter Architecture,Testing

# Available categories: Architecture, Testing, Performance, CodeQuality
```

### Customizing Tiers

```toml
[debtmap.tiers]
t2_complexity_threshold = 15  # Tier 2 complexity cutoff
t2_dependency_threshold = 10  # Tier 2 dependency cutoff
show_t4_in_main_report = false  # Hide maintenance items
```
```

### Architecture Updates

Document tiered prioritization in design docs:

```markdown
## Prioritization Strategy: Tiered Approach

Debtmap uses a multi-tier prioritization strategy to surface actionable recommendations:

1. **Tier Classification**: Items classified by type (architecture, testing, maintenance)
2. **Tier Ordering**: Architectural issues always surface first
3. **Score Within Tier**: Items sorted by score within each tier
4. **Coverage as Context**: Coverage shown within items, not as separate priority

This ensures users focus on high-impact architectural debt before tackling testing gaps.
```

## Implementation Notes

### Tier Threshold Calibration

**Default thresholds based on research:**
- T2 complexity ≥ 15: Industry consensus for "complex" (SonarQube, Code Climate)
- T2 dependencies ≥ 10: Coupling threshold from object-oriented metrics
- T3 complexity ≥ 10: McCabe's original recommendation for "testable"

**Customization examples:**
- **Strict**: T2 complexity ≥ 10, T3 complexity ≥ 7
- **Balanced**: T2 complexity ≥ 15, T3 complexity ≥ 10 (default)
- **Lenient**: T2 complexity ≥ 20, T3 complexity ≥ 15

### Category Filtering Implementation

The existing `--filter` flag (currently defined but not implemented) should:
1. Parse comma-separated category names (case-insensitive)
2. Map to `DebtCategory` enum values
3. Filter `UnifiedDebtItem` list before prioritization
4. Preserve tier ordering within filtered results

Valid category values:
- `Architecture` → God Objects, Feature Envy, etc.
- `Testing` → Testing gaps, test complexity, flaky tests
- `Performance` → Async misuse, nested loops, blocking I/O
- `CodeQuality` → Complexity hotspots, dead code, duplication

## Migration and Compatibility

### Breaking Changes

**Minor Breaking Change**: Recommendation ordering may change

**Mitigation**:
- Document new prioritization strategy in CHANGELOG
- Provide `--legacy-prioritization` flag for backward compatibility
- Add migration guide comparing old vs new ordering

### Configuration Migration

New config section:

```toml
[debtmap.tiers]
# Tier 2: Complex untested code
t2_complexity_threshold = 15
t2_dependency_threshold = 10

# Tier 3: Testing gaps
t3_complexity_threshold = 10

# Display options
show_tier_labels = true
show_t4_in_main_report = false

# Tier weights (for score adjustment within tiers)
t1_weight = 1.5  # Boost architectural issues
t2_weight = 1.0
t3_weight = 0.7
t4_weight = 0.3
```

### Rollout Plan

1. **Version 0.2.6**: Ship tiered prioritization as default
2. **Documentation**: Update all examples and guides
3. **Blog Post**: Explain new prioritization philosophy
4. **Feedback**: Gather user feedback on tier thresholds
5. **Tuning**: Adjust defaults based on real-world usage
