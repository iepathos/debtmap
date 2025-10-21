---
number: 119
title: Path Count and Test Recommendation Separation
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-21
---

# Specification 119: Path Count and Test Recommendation Separation

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap v0.2.9 conflates "number of execution paths" with "number of tests needed", leading to misleading output where simple functions are reported as having more execution paths than they actually do.

**False Positive Example**:
```rust
// ContextMatcher::any() - cyclomatic=1
pub fn any() -> Self {
    Self { role: None, file_type: None, /* ... */ }
}
```

**Current Output**:
```
WHY: Business logic with 100% coverage gap, currently 0% covered.
     Needs 2 test cases to cover all 2 execution paths
```

**The Problem**:
- Function has **cyclomatic=1** → **1 execution path** (no branching)
- Debtmap recommends **2 tests** (minimum for edge cases)
- But says "cover all **2 execution paths**" ← **WRONG**

**Root Cause** (`src/priority/scoring/recommendation_helpers.rs:117`):
```rust
format!("Needs {} test cases to cover all {} execution paths",
       test_cases_needed,      // 2 (minimum recommendation)
       cyclomatic.max(2))      // 2 (artificially inflated from 1)
```

**Why This Matters**:
- Misleads users about actual code complexity
- Makes simple functions appear more complex than they are
- Undermines trust in debtmap's analysis
- Violates principle of truth-first reporting

## Objective

Accurately report the true number of execution paths while maintaining appropriate test recommendations, separating "what exists" from "what's needed".

## Requirements

### Functional Requirements

**FR1: Report Actual Path Count**
- Display true cyclomatic complexity as path count
- Never artificially inflate path count for reporting
- Show "1 execution path" for cyclomatic=1 functions

**FR2: Maintain Minimum Test Recommendations**
- Still recommend minimum 2 tests for any function
- Clearly separate test count from path count
- Explain rationale for minimum tests

**FR3: Update Messaging Format**
- Old: "Needs N tests to cover all M paths" (where M might be inflated)
- New: "Needs N tests to cover M path(s)" (M is actual)
- Or: "Needs N tests (minimum 2 recommended)"

**FR4: Grammatical Correctness**
- "1 execution path" (singular)
- "N execution paths" (plural for N > 1)
- Consistent terminology throughout

**FR5: Contextual Explanation**
- Explain why 2+ tests recommended for simple functions
- Mention happy path + edge cases
- Optional: Link to testing best practices

### Non-Functional Requirements

**NFR1: Accuracy**
- 100% accurate path count reporting
- No artificial inflation or deflation
- Cyclomatic complexity = execution paths (by definition)

**NFR2: Clarity**
- Users immediately understand the distinction
- No confusion between "paths" and "tests"
- Clear rationale for recommendations

**NFR3: Consistency**
- Same terminology across all output formats
- Consistent explanation of minimum test count
- Aligned with industry standard definitions

## Acceptance Criteria

- [x] Path count always equals cyclomatic complexity (no max(N, 2) inflation)
- [x] Test recommendations can exceed path count (e.g., 2 tests for 1 path)
- [x] Output uses singular "path" when count=1, plural "paths" when count>1
- [x] Explanation provided for why minimum 2 tests recommended
- [x] `ContextMatcher::any()` reports "1 execution path" not "2 execution paths"
- [x] All output formatters updated with new messaging
- [x] Documentation explains path vs test count relationship
- [x] Test suite validates correct path reporting
- [x] User documentation includes examples

## Technical Details

### Implementation Approach

**Phase 1: Fix Path Count Reporting**

**File**: `src/priority/scoring/recommendation_helpers.rs:117`

```rust
// OLD (INCORRECT):
format!(
    "Needs {} test cases to cover all {} execution paths",
    test_cases_needed,
    cyclomatic.max(2)  // ❌ Artificially inflates path count
)

// NEW (CORRECT):
let actual_paths = cyclomatic;  // Never inflate
let min_tests = test_cases_needed.max(2);  // Minimum 2 tests recommended

if actual_paths == 1 {
    format!(
        "Needs {} tests to cover the single execution path (minimum 2 tests recommended for edge cases)",
        min_tests
    )
} else {
    format!(
        "Needs {} tests to cover {} execution path{}",
        min_tests,
        actual_paths,
        if actual_paths == 1 { "" } else { "s" }
    )
}
```

**Phase 2: Update All Formatters**

**File**: `src/priority/scoring/recommendation.rs`

```rust
fn generate_coverage_explanation(
    cyclomatic: u32,
    test_cases_needed: u32,
    coverage_gap: i32,
) -> String {
    let actual_paths = cyclomatic;
    let recommended_tests = test_cases_needed.max(2);

    // Different messaging for single-path vs multi-path functions
    match actual_paths {
        1 => format!(
            "Single-path function with {}% coverage gap. \
             Recommending {} tests (happy path + edge cases)",
            coverage_gap, recommended_tests
        ),
        _ => format!(
            "{}-path function with {}% coverage gap. \
             Recommending {} tests to cover all execution paths",
            actual_paths, coverage_gap, recommended_tests
        ),
    }
}
```

**Phase 3: Add Rationale for Minimum Tests**

```rust
/// Explain why we recommend minimum tests even for simple functions
fn explain_minimum_test_rationale(paths: u32, recommended: u32) -> Option<String> {
    if paths == 1 && recommended >= 2 {
        Some(
            "Even simple functions benefit from multiple tests: \
             one for the happy path, one for edge cases/boundaries"
                .to_string(),
        )
    } else if paths < recommended {
        Some(format!(
            "Recommended {} tests for {} paths to ensure edge case coverage",
            recommended, paths
        ))
    } else {
        None
    }
}
```

**Phase 4: Update Test Calculation**

**File**: `src/priority/scoring/test_calculation.rs`

```rust
pub fn calculate_tests_needed(
    cyclomatic: u32,
    coverage_percent: f64,
    tier: Option<ComplexityTier>,
) -> TestRecommendation {
    // ... existing logic ...

    let (count, formula, rationale) = match tier {
        ComplexityTier::Simple => {
            let tests = (cyclomatic as f64 * coverage_gap).ceil() as u32;
            let tests = tests.max(2); // Minimum 2 tests

            // NEW: Explain why tests > paths
            let explanation = if tests > cyclomatic {
                format!(
                    "{} tests recommended for {} path(s) - includes edge case coverage",
                    tests, cyclomatic
                )
            } else {
                format!("One test per execution path")
            };

            (
                tests,
                format!("max(cyclomatic × coverage_gap, 2) = {}", tests),
                explanation,
            )
        }
        // ... other tiers ...
    };

    // ... rest of function ...
}
```

### Architecture Changes

**Modified Files**:
- `src/priority/scoring/recommendation_helpers.rs` - Primary fix
- `src/priority/scoring/recommendation.rs` - Update explanations
- `src/priority/scoring/test_calculation.rs` - Add rationale
- `src/priority/formatter_verbosity.rs` - Update output formatting
- `src/risk/insights.rs` - Update recommendation text

**New Functions**:
```rust
/// Format path count with correct grammar (singular/plural)
fn format_path_count(count: u32) -> String {
    match count {
        1 => "1 execution path".to_string(),
        n => format!("{} execution paths", n),
    }
}

/// Explain test recommendation rationale
fn explain_test_rationale(
    paths: u32,
    tests: u32,
    coverage_percent: f64,
) -> String {
    if paths == 1 && tests >= 2 {
        format!(
            "{} tests for single-path function (happy path + {} edge case{})",
            tests,
            tests - 1,
            if tests > 2 { "s" } else { "" }
        )
    } else if tests > paths {
        format!(
            "{} tests for {} paths (includes edge case coverage)",
            tests, paths
        )
    } else {
        format!("{} tests for {} paths (one per path)", tests, paths)
    }
}
```

### Data Structures

**No changes** - This is primarily a formatting/messaging fix.

**Optional Enhancement** (for JSON output):
```rust
#[derive(Serialize, Deserialize)]
pub struct TestRecommendation {
    /// Number of tests recommended
    pub test_count: u32,

    /// Actual number of execution paths (cyclomatic complexity)
    pub execution_paths: u32,

    /// Explanation of why test_count may exceed execution_paths
    pub rationale: String,

    /// Formula used for calculation
    pub formula: String,
}
```

### APIs and Interfaces

**Command-Line Output Changes**:

```diff
- WHY: Business logic with 100% coverage gap, currently 0% covered.
-      Needs 2 test cases to cover all 2 execution paths

+ WHY: Business logic with 100% coverage gap, currently 0% covered.
+      Needs 2 tests for single execution path (happy path + edge cases)
```

**Verbose Output**:
```
RECOMMENDATION:
├─ TESTS NEEDED: 2
├─ EXECUTION PATHS: 1
├─ RATIONALE: Minimum 2 tests recommended (happy path + edge cases)
└─ FORMULA: max(cyclomatic × coverage_gap, 2) = max(1 × 1.0, 2) = 2
```

## Dependencies

**Prerequisites**: None

**Affected Components**:
- All recommendation formatters
- Test calculation modules
- User-facing output

**External Dependencies**: None

## Testing Strategy

### Unit Tests

**Test Path Count Accuracy**:
```rust
#[test]
fn test_single_path_function_reports_one_path() {
    let cyclomatic = 1;
    let coverage_pct = 0.0;

    let recommendation = generate_simple_function_recommendation(
        cyclomatic,
        coverage_pct,
        100, // coverage_gap
        FunctionRole::PureLogic,
        &func,
        &None,
    );

    let explanation = recommendation.1;

    // Should mention "1 execution path" or "single execution path"
    assert!(
        explanation.contains("1 execution path")
            || explanation.contains("single execution path"),
        "Expected '1 execution path', got: {}",
        explanation
    );

    // Should NOT claim "2 execution paths"
    assert!(
        !explanation.contains("2 execution paths"),
        "Should not inflate path count to 2"
    );
}

#[test]
fn test_multi_path_function_reports_actual_count() {
    let cyclomatic = 5;
    let coverage_pct = 0.5;

    let recommendation = generate_simple_function_recommendation(
        cyclomatic,
        coverage_pct,
        50, // coverage_gap
        FunctionRole::PureLogic,
        &func,
        &None,
    );

    let explanation = recommendation.1;

    // Should mention actual path count
    assert!(explanation.contains("5 execution paths")
        || explanation.contains("5-path function"));
}

#[test]
fn test_minimum_test_rationale_provided() {
    let cyclomatic = 1;
    let tests_needed = 2;

    let rationale = explain_test_rationale(cyclomatic, tests_needed, 0.0);

    // Should explain why 2 tests for 1 path
    assert!(rationale.contains("happy path"));
    assert!(rationale.contains("edge case"));
}

#[test]
fn test_grammatical_correctness() {
    assert_eq!(format_path_count(1), "1 execution path");
    assert_eq!(format_path_count(2), "2 execution paths");
    assert_eq!(format_path_count(10), "10 execution paths");
}
```

### Integration Tests

**Regression Test for ContextMatcher::any()**:
```rust
#[test]
fn test_context_matcher_any_path_count() {
    let output = analyze_file("src/context/rules.rs");

    let any_func = output.find_function("any", 52);
    assert!(any_func.is_some());

    let recommendation = any_func.unwrap().recommendation;

    // Should report 1 path, not 2
    assert!(
        recommendation.contains("1 execution path")
            || recommendation.contains("single execution path"),
        "Should report actual path count (1), got: {}",
        recommendation
    );

    // Should NOT claim 2 paths
    assert!(
        !recommendation.contains("2 execution paths"),
        "Should not inflate path count"
    );
}
```

**Golden File Tests**:
```rust
#[test]
fn test_path_count_output_format() {
    let test_cases = vec![
        ("single_path.rs", "1 execution path"),
        ("three_paths.rs", "3 execution paths"),
        ("complex.rs", "12 execution paths"),
    ];

    for (file, expected_pattern) in test_cases {
        let output = analyze_file(&format!("tests/fixtures/{}", file));
        assert!(
            output.contains(expected_pattern),
            "Expected '{}' in output for {}",
            expected_pattern,
            file
        );
    }
}
```

### User Acceptance Testing

**Scenario 1**: Simple constructor function
```
INPUT: Simple constructor with cyclomatic=1

EXPECTED OUTPUT:
"Needs 2 tests for single execution path (happy path + edge cases)"

NOT:
"Needs 2 tests to cover all 2 execution paths" ❌
```

**Scenario 2**: Complex function
```
INPUT: Complex function with cyclomatic=12

EXPECTED OUTPUT:
"Needs 12 tests to cover 12 execution paths"

OR (if recommending more):
"Needs 15 tests for 12 paths (includes edge case coverage)"
```

## Documentation Requirements

### Code Documentation

**Function Documentation**:
```rust
/// Calculate test recommendations for function coverage
///
/// # Important: Path Count vs Test Recommendations
///
/// This function separates two distinct concepts:
/// 1. **Execution Paths**: Actual count from cyclomatic complexity (measured)
/// 2. **Test Recommendations**: Number of tests needed (may exceed paths)
///
/// We recommend minimum 2 tests even for single-path functions to ensure
/// both happy path and edge case coverage.
///
/// # Examples
///
/// ```
/// // Single-path constructor (cyclomatic=1)
/// let rec = calculate_tests(1, 0.0);
/// assert_eq!(rec.execution_paths, 1);  // Actual paths
/// assert_eq!(rec.test_count, 2);       // Recommended tests
/// assert!(rec.rationale.contains("happy path + edge cases"));
///
/// // Multi-path function (cyclomatic=5)
/// let rec = calculate_tests(5, 0.5);
/// assert_eq!(rec.execution_paths, 5);
/// assert_eq!(rec.test_count, 3);  // Covers 50% gap
/// ```
///
/// # See Also
///
/// - Spec 119: Path Count and Test Recommendation Separation
/// - McCabe (1976): Cyclomatic Complexity definition
pub fn calculate_tests_needed(cyclomatic: u32, coverage: f64) -> TestRecommendation
```

### User Documentation

**Update**: `book/src/understanding-recommendations.md`

```markdown
## Test Recommendations vs Execution Paths

Debtmap distinguishes between:

### Execution Paths (Measured)
The **actual number of independent paths** through a function, determined by
cyclomatic complexity.

```rust
// 1 execution path (no branching)
fn simple() -> i32 {
    42
}

// 3 execution paths (if-else = 2 branches + 1)
fn with_conditional(x: i32) -> i32 {
    if x > 0 { 1 } else { -1 }
}
```

### Test Recommendations (Guidance)
The **number of tests we recommend** to adequately cover the function.

**Key Point**: Test count may EXCEED path count for thoroughness.

### Why More Tests Than Paths?

Even simple functions (1 path) benefit from multiple tests:

```rust
// 1 execution path, but recommend 2+ tests
fn parse_port(s: &str) -> u16 {
    s.parse().unwrap_or(8080)
}

// Recommended tests:
// 1. Happy path: parse_port("3000") → 3000
// 2. Edge case: parse_port("invalid") → 8080
// 3. Boundary: parse_port("65535") → 65535
```

### Reading Debtmap Output

```
NEEDS: 2 tests for single execution path (happy path + edge cases)
       ↑                 ↑
       Test count        Actual paths
```

For complex functions:
```
NEEDS: 12 tests to cover 12 execution paths
       ↑                 ↑
       One test per path (minimum)
```

### Formula Reference

| Complexity | Paths | Tests | Rationale |
|------------|-------|-------|-----------|
| cyclo=1 | 1 | 2 | Minimum: happy + edge |
| cyclo=5 | 5 | 5-7 | One per path + edges |
| cyclo=12 | 12 | 12-15 | Path coverage + boundaries |
```

**Update**: `book/src/faq.md`

```markdown
### Why does debtmap recommend 2 tests for a function with 1 execution path?

Even simple, linear functions benefit from multiple test cases:

- **Test 1 (Happy Path)**: Verify normal operation
- **Test 2 (Edge Cases)**: Boundary values, error conditions, special inputs

Example:
```rust
// 1 path, but needs 2+ tests
fn calculate_discount(amount: f64) -> f64 {
    amount * 0.1
}

// Tests:
// 1. Normal: calculate_discount(100.0) → 10.0
// 2. Zero: calculate_discount(0.0) → 0.0
// 3. Large: calculate_discount(f64::MAX) → ...
```

Debtmap recommends **minimum 2 tests** for any function to ensure robustness.
```

### Architecture Updates

None needed - this is a messaging clarification, not an architectural change.

## Implementation Notes

### Phased Rollout

**Phase 1**: Fix path count reporting (this spec)
**Phase 2**: Add detailed test rationale (optional enhancement)
**Phase 3**: Interactive explanations in IDE integrations

### Edge Cases

**Edge Case 1**: Cyclomatic=0 (dead code)
```rust
// Should this ever happen?
if cyclomatic == 0 {
    return "Unreachable code (no execution paths)".to_string();
}
```

**Edge Case 2**: Very high cyclomatic (>50)
```rust
if cyclomatic > 50 {
    // Recommend property-based testing instead of path enumeration
    format!(
        "{} execution paths (recommend property-based testing instead of \
         {} individual path tests)",
        cyclomatic,
        recommended_tests
    )
}
```

### Gotchas

**Gotcha 1**: Don't confuse with branch count
- Paths ≠ Branches
- Cyclomatic = decision points, not branch statements

**Gotcha 2**: Maintain minimum test recommendation
- Always recommend ≥2 tests for any function
- Never reduce test count below minimum

## Migration and Compatibility

### Breaking Changes

**None** - Pure improvement to output messaging.

### User Impact

**Positive Impact**:
- More accurate understanding of code complexity
- Clearer test guidance
- Better trust in analysis

**Potential Confusion**:
- Users might wonder why tests > paths
- Solution: Clear explanation in output

### Migration Steps

No migration needed - automatic improvement on upgrade.

## Success Metrics

### Quantitative Metrics

- **Accuracy**: 100% of path counts match cyclomatic complexity
- **Test Coverage**: No reduction in recommended test count
- **Regressions**: Zero test failures from output format changes

### Qualitative Metrics

- **User Understanding**: Fewer questions about "why 2 tests for 1 path?"
- **Trust**: Increased confidence in debtmap's accuracy
- **Clarity**: Clear distinction between measurement and recommendation

### Validation

**Before Implementation**:
```
ContextMatcher::any() - "Needs 2 tests to cover all 2 execution paths"
(User thinks: "But there's only 1 path! Is debtmap wrong?")
```

**After Implementation**:
```
ContextMatcher::any() - "Needs 2 tests for single execution path (happy path + edge cases)"
(User thinks: "Ah, 1 path but 2 tests for thoroughness. Makes sense!")
```

## Future Enhancements

### Phase 2: Detailed Test Scenarios
Generate specific test case suggestions:
```
RECOMMENDED TESTS (2 for 1 path):
  1. Happy path: Test with valid input
  2. Edge case: Test with boundary values (empty, max, min)
```

### Phase 3: Test Template Generation
Auto-generate test skeletons:
```rust
#[test]
fn test_any_happy_path() {
    let matcher = ContextMatcher::any();
    assert!(matcher.matches(&any_context));
}

#[test]
fn test_any_edge_cases() {
    // TODO: Add edge case tests
}
```

### Phase 4: Adaptive Recommendations
Learn from actual test suites to improve recommendations:
- Analyze projects with high coverage
- Identify common test patterns
- Suggest project-specific test strategies
