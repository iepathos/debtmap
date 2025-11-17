---
number: 178
title: Fix Moderate Complexity Recommendation Logic
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-11-16
---

# Specification 178: Fix Moderate Complexity Recommendation Logic

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

A critical bug exists in the moderate complexity recommendation generation that produces nonsensical advice like "Reduce complexity from 9 to ~10" when the current complexity (9) is already below the target (10).

**Bug Evidence** (from blog sample code analysis):

```
#1 SCORE: 4.15 [MEDIUM]
├─ LOCATION: ./src/state_reconciliation.rs:81 reconcile_state()
├─ COMPLEXITY: cyclomatic=9, cognitive=16, nesting=4
├─ WHY THIS MATTERS: Approaching complexity threshold (9/16).
                      Preventive refactoring will keep code maintainable.
├─ RECOMMENDED ACTION: Reduce complexity from 9 to ~10
                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                       ❌ NONSENSICAL: Suggests INCREASING complexity!
```

```
#2 SCORE: 1.30 [LOW]
├─ LOCATION: ./src/validation.rs:30 validate_config()
├─ COMPLEXITY: cyclomatic=6, cognitive=6, nesting=1
├─ WHY THIS MATTERS: Approaching complexity threshold (6/6).
├─ RECOMMENDED ACTION: Reduce complexity from 6 to ~10
                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                       ❌ NONSENSICAL: Target is higher than current!
```

**Root Cause** (`src/priority/scoring/concise_recommendation.rs:492`):

```rust
fn generate_moderate_recommendation(...) -> ActionableRecommendation {
    let target_complexity = 10;  // ← HARDCODED TARGET
    let complexity_reduction = cyclomatic.saturating_sub(target_complexity).max(5);

    ActionableRecommendation {
        primary_action: format!(
            "Reduce complexity from {} to ~{}",
            cyclomatic, target_complexity  // ← BUG: Always shows 10 as target
        ),
        // ...
    }
}
```

**The Logic Error**:
1. When `cyclomatic < 10`, `saturating_sub()` returns 0
2. `.max(5)` forces it to 5 (so impact shows "-5 complexity")
3. But the message still says "Reduce from {current} to ~10"
4. Result: "Reduce from 6 to ~10" or "Reduce from 9 to ~10" (increasing!)

**Impact**: User-facing recommendations are confusing and undermine trust in the tool's intelligence.

## Objective

Fix the moderate complexity recommendation logic to provide sensible, contextually appropriate advice for functions that are already below or near complexity thresholds.

## Requirements

### Functional Requirements

1. **Correct Target Calculation**
   - When current complexity < 10, target should be lower (not 10)
   - Target should always be less than current complexity
   - Target should be achievable through refactoring

2. **Contextual Recommendations**
   - Functions already below threshold (cyclo < 10): "Maintain current complexity"
   - Functions near threshold (cyclo 10-15): "Reduce to single-digit complexity"
   - Functions above threshold (cyclo > 15): "Reduce to ~10"

3. **Sensible Reduction Estimates**
   - Impact should match the recommendation
   - "Reduce from 9 to ~10" should never appear
   - Reduction amount should be realistic (-2 to -5 for moderate, not fixed 5)

### Non-Functional Requirements

1. **Consistency**
   - All complexity-based recommendations should use consistent logic
   - Target complexity should align with industry best practices
   - Messages should be clear and actionable

2. **Accuracy**
   - Estimated impact should match recommended action
   - "Approaching threshold" should only appear when actually approaching
   - Complexity tiers should be well-defined

## Acceptance Criteria

- [ ] Functions with cyclomatic < 10 do NOT suggest "Reduce to ~10"
- [ ] Recommendation targets are always lower than current complexity
- [ ] "Approaching threshold" message only appears when cyclo >= 8 OR cognitive >= 15
- [ ] Impact estimate matches recommendation (e.g., "from 9 to ~7" shows "-2 complexity")
- [ ] Functions with cyclo < 8 get "maintenance" recommendations, not "reduction"
- [ ] All three moderate complexity patterns tested: below, near, above threshold
- [ ] Integration test with blog sample code validates correct output
- [ ] No regression in other pattern recommendations (nesting, branching, mixed, chaotic)
- [ ] Documentation updated to explain complexity tiers and targets

## Technical Details

### Implementation Approach

**Step 1: Define Complexity Tiers**

```rust
// src/priority/scoring/concise_recommendation.rs

enum ComplexityTier {
    Low,        // cyclo < 8, cognitive < 15
    Moderate,   // cyclo 8-14, cognitive 15-24
    High,       // cyclo 15-24, cognitive 25-39
    VeryHigh,   // cyclo >= 25, cognitive >= 40
}

fn classify_complexity_tier(cyclomatic: u32, cognitive: u32) -> ComplexityTier {
    match (cyclomatic, cognitive) {
        (c, cog) if c < 8 && cog < 15 => ComplexityTier::Low,
        (c, cog) if c < 15 && cog < 25 => ComplexityTier::Moderate,
        (c, cog) if c < 25 && cog < 40 => ComplexityTier::High,
        _ => ComplexityTier::VeryHigh,
    }
}
```

**Step 2: Calculate Appropriate Target**

```rust
fn calculate_target_complexity(current: u32, tier: ComplexityTier) -> u32 {
    match tier {
        ComplexityTier::Low => {
            // Already low - maintain or slightly improve
            current.saturating_sub(1).max(3)
        },
        ComplexityTier::Moderate => {
            // Aim for single-digit complexity
            if current >= 10 {
                // 10-14 → target 8
                8
            } else {
                // 8-9 → target 5-6 (reduce by 2-3)
                current.saturating_sub(3).max(5)
            }
        },
        ComplexityTier::High => {
            // Aim for moderate complexity (10)
            10
        },
        ComplexityTier::VeryHigh => {
            // Significant reduction needed (aim for 10-15)
            (current / 2).min(15).max(10)
        },
    }
}
```

**Step 3: Generate Tier-Appropriate Recommendations**

```rust
fn generate_moderate_recommendation(
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let tier = classify_complexity_tier(cyclomatic, cognitive);
    let target = calculate_target_complexity(cyclomatic, tier);
    let reduction = cyclomatic.saturating_sub(target);

    match tier {
        ComplexityTier::Low => {
            // Already below thresholds - maintenance recommendation
            ActionableRecommendation {
                primary_action: "Maintain current low complexity".to_string(),
                rationale: format!(
                    "Function has low complexity ({}/{}). \
                     Continue following current patterns to keep it maintainable.",
                    cyclomatic, cognitive
                ),
                steps: Some(vec![
                    ActionStep {
                        description: "Add tests to preserve behavior during future changes".to_string(),
                        impact: "+safety for refactoring".to_string(),
                        difficulty: Difficulty::Easy,
                        commands: vec![format!("cargo test {}::", metrics.name)],
                    },
                ]),
                estimated_effort_hours: Some(0.5),
                // ... other fields
            }
        },

        ComplexityTier::Moderate => {
            // Near threshold - preventive refactoring
            ActionableRecommendation {
                primary_action: if cyclomatic >= 10 {
                    format!("Reduce complexity from {} to ~{}", cyclomatic, target)
                } else {
                    format!("Optional: Reduce complexity from {} to ~{} for future-proofing",
                            cyclomatic, target)
                },
                rationale: format!(
                    "Moderate complexity ({}/{}). {} threshold but maintainable. \
                     Preventive refactoring will ease future changes.",
                    cyclomatic,
                    cognitive,
                    if cyclomatic >= 10 { "Approaching" } else { "Below" }
                ),
                steps: Some(vec![
                    ActionStep {
                        description: "Extract most complex section into focused function".to_string(),
                        impact: format!("-{} complexity", reduction),
                        difficulty: Difficulty::Medium,
                        commands: vec!["cargo clippy".to_string()],
                    },
                    ActionStep {
                        description: "Add tests before refactoring if coverage < 80%".to_string(),
                        impact: "+safety net for refactoring".to_string(),
                        difficulty: Difficulty::Medium,
                        commands: vec![format!("cargo test {}::", metrics.name)],
                    },
                ]),
                estimated_effort_hours: Some((cyclomatic as f32 / 10.0) * 1.5),
                // ... other fields
            }
        },

        ComplexityTier::High | ComplexityTier::VeryHigh => {
            // Existing logic for high complexity
            // (same as current implementation)
            ActionableRecommendation {
                primary_action: format!("Reduce complexity from {} to ~{}", cyclomatic, target),
                rationale: format!(
                    "High complexity ({}/{}). Exceeds maintainability thresholds. \
                     Refactoring required.",
                    cyclomatic, cognitive
                ),
                // ... rest of high complexity recommendation
                // ... (keep existing implementation)
            }
        },
    }
}
```

### Architecture Changes

No architectural changes - this is a bug fix within existing recommendation generation.

### Data Structures

```rust
// Add to src/priority/scoring/concise_recommendation.rs

/// Complexity tier classification for tier-appropriate recommendations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComplexityTier {
    /// Low complexity: cyclo < 8, cognitive < 15
    Low,
    /// Moderate complexity: cyclo 8-14, cognitive 15-24
    Moderate,
    /// High complexity: cyclo 15-24, cognitive 25-39
    High,
    /// Very high complexity: cyclo >= 25, cognitive >= 40
    VeryHigh,
}
```

### APIs and Interfaces

No API changes - internal refactoring only.

## Dependencies

**Prerequisites**: None - standalone bug fix

**Affected Components**:
- `src/priority/scoring/concise_recommendation.rs` - Main fix location
- `generate_moderate_recommendation()` function

**External Dependencies**: None

## Testing Strategy

### Unit Tests

**Test 1: Low Complexity (Below Threshold)**

```rust
#[test]
fn test_low_complexity_gets_maintenance_recommendation() {
    let metrics = create_test_metrics(6, 8);  // Low complexity
    let rec = generate_moderate_recommendation(6, 8, &metrics);

    assert!(rec.primary_action.contains("Maintain"));
    assert!(!rec.primary_action.contains("Reduce from 6 to ~10"));
    assert!(rec.rationale.contains("low complexity"));
}
```

**Test 2: Moderate Complexity (Near Threshold)**

```rust
#[test]
fn test_moderate_complexity_suggests_lower_target() {
    let metrics = create_test_metrics(9, 16);
    let rec = generate_moderate_recommendation(9, 16, &metrics);

    // Should suggest reducing to 5-6, not 10
    assert!(rec.primary_action.contains("Optional") || rec.primary_action.contains("to ~5") || rec.primary_action.contains("to ~6"));
    assert!(!rec.primary_action.contains("to ~10"));
}

#[test]
fn test_moderate_at_threshold_suggests_reduction() {
    let metrics = create_test_metrics(12, 20);
    let rec = generate_moderate_recommendation(12, 20, &metrics);

    // Should suggest reducing to 8
    assert!(rec.primary_action.contains("to ~8"));
    assert!(!rec.primary_action.contains("to ~10"));
}
```

**Test 3: High Complexity (Above Threshold)**

```rust
#[test]
fn test_high_complexity_suggests_target_10() {
    let metrics = create_test_metrics(20, 30);
    let rec = generate_moderate_recommendation(20, 30, &metrics);

    // Should suggest reducing to 10
    assert!(rec.primary_action.contains("from 20 to ~10"));
}
```

**Test 4: Impact Matches Recommendation**

```rust
#[test]
fn test_impact_matches_target() {
    let metrics = create_test_metrics(12, 20);
    let rec = generate_moderate_recommendation(12, 20, &metrics);

    // If recommending "from 12 to ~8", impact should be "-4 complexity"
    if rec.primary_action.contains("to ~8") {
        assert!(rec.steps.is_some());
        let steps = rec.steps.unwrap();
        let extract_step = steps.iter().find(|s| s.description.contains("Extract"));
        assert!(extract_step.is_some());
        assert!(extract_step.unwrap().impact.contains("-4") ||
                extract_step.unwrap().impact.contains("4"));
    }
}
```

### Integration Tests

**Test 5: Blog Sample Code Validation**

```rust
// tests/blog_samples_recommendation_test.rs

#[test]
fn test_blog_sample_reconcile_state_recommendation() {
    // From blog samples: cyclo=9, cognitive=16, nesting=4
    let code = include_str!("../test_data/blog_samples/state_reconciliation.rs");
    let analysis = analyze_rust_code(code, "state_reconciliation.rs");

    let reconcile_fn = analysis.functions.iter()
        .find(|f| f.name == "reconcile_state")
        .expect("Should find reconcile_state function");

    let debt_item = create_complexity_debt_item(reconcile_fn);
    let recommendation = &debt_item.recommendation;

    // Should NOT say "Reduce from 9 to ~10"
    assert!(!recommendation.primary_action.contains("to ~10"));

    // Should suggest reasonable target (5-8 range)
    assert!(
        recommendation.primary_action.contains("to ~5") ||
        recommendation.primary_action.contains("to ~6") ||
        recommendation.primary_action.contains("to ~7") ||
        recommendation.primary_action.contains("to ~8") ||
        recommendation.primary_action.contains("Maintain") ||
        recommendation.primary_action.contains("Optional")
    );
}

#[test]
fn test_blog_sample_validate_config_recommendation() {
    // From blog samples: cyclo=6, cognitive=6, nesting=1
    let code = include_str!("../test_data/blog_samples/validation.rs");
    let analysis = analyze_rust_code(code, "validation.rs");

    let validate_fn = analysis.functions.iter()
        .find(|f| f.name == "validate_config")
        .expect("Should find validate_config function");

    let debt_item = create_complexity_debt_item(validate_fn);
    let recommendation = &debt_item.recommendation;

    // Low complexity - should get maintenance recommendation
    assert!(
        recommendation.primary_action.contains("Maintain") ||
        !recommendation.primary_action.contains("Reduce")
    );

    // Should NOT suggest increasing to 10
    assert!(!recommendation.primary_action.contains("to ~10"));
}
```

### Regression Tests

**Test 6: Other Patterns Unchanged**

```rust
#[test]
fn test_high_nesting_pattern_unchanged() {
    // Ensure HighNesting recommendations still work
    let metrics = create_test_metrics_with_nesting(12, 50, 5);
    let pattern = ComplexityPattern::detect(&complexity_metrics);

    assert!(matches!(pattern, ComplexityPattern::HighNesting { .. }));

    let rec = generate_complexity_steps(12, 50, &metrics);
    assert!(rec.primary_action.contains("Reduce nesting"));
}

#[test]
fn test_high_branching_pattern_unchanged() {
    let metrics = create_test_metrics(22, 45);
    let pattern = ComplexityPattern::detect(&complexity_metrics);

    assert!(matches!(pattern, ComplexityPattern::HighBranching { .. }));

    let rec = generate_complexity_steps(22, 45, &metrics);
    assert!(rec.primary_action.contains("Split into"));
}
```

## Documentation Requirements

### Code Documentation

1. **Document Complexity Tiers**:
```rust
/// Complexity tier thresholds based on industry standards and empirical analysis
///
/// # Tier Definitions
///
/// - **Low** (cyclo < 8, cognitive < 15): Well-structured, easy to understand
///   - Recommendation: Maintain current patterns
///   - Example: Simple validation, accessors, small functions
///
/// - **Moderate** (cyclo 8-14, cognitive 15-24): Manageable but approaching limits
///   - Recommendation: Optional preventive refactoring
///   - Example: Business logic with moderate branching
///
/// - **High** (cyclo 15-24, cognitive 25-39): Exceeds maintainability thresholds
///   - Recommendation: Refactoring required
///   - Example: Complex orchestration, large case statements
///
/// - **Very High** (cyclo >= 25, cognitive >= 40): Critical complexity
///   - Recommendation: Significant refactoring required
///   - Example: God functions, tangled logic
```

2. **Document Target Calculation Logic**:
```rust
/// Calculate appropriate complexity reduction target based on current tier
///
/// # Target Selection Strategy
///
/// - **Low tier**: Maintain or slight improvement (current - 1, min 3)
/// - **Moderate tier**: Aim for single-digit (8 if >= 10, else current - 3)
/// - **High tier**: Aim for moderate complexity (10)
/// - **Very High tier**: Significant reduction (half current, capped at 10-15)
///
/// # Examples
///
/// - complexity=6 → target=5 (maintain)
/// - complexity=9 → target=6 (preventive)
/// - complexity=12 → target=8 (reduce to single-digit)
/// - complexity=20 → target=10 (significant reduction)
/// - complexity=40 → target=15 (very high → high tier)
```

### User Documentation

Update `docs/recommendations.md`:

```markdown
## Complexity Recommendations

Debtmap provides contextual recommendations based on your function's complexity tier:

### Low Complexity (cyclo < 8, cognitive < 15)
✅ **Status**: Healthy
💡 **Recommendation**: Maintain current patterns
📊 **Example**: "Maintain current low complexity"

### Moderate Complexity (cyclo 8-14, cognitive 15-24)
⚠️ **Status**: Approaching thresholds
💡 **Recommendation**: Optional preventive refactoring
📊 **Example**: "Optional: Reduce complexity from 9 to ~6 for future-proofing"

### High Complexity (cyclo 15-24, cognitive 25-39)
❌ **Status**: Exceeds thresholds
💡 **Recommendation**: Refactoring required
📊 **Example**: "Reduce complexity from 18 to ~10"

### Very High Complexity (cyclo >= 25, cognitive >= 40)
🚨 **Status**: Critical
💡 **Recommendation**: Significant refactoring required
📊 **Example**: "Reduce complexity from 35 to ~15"
```

## Implementation Notes

### Testing with Blog Samples

The blog sample code provides excellent test cases:

**`reconcile_state`** (cyclo=9, cognitive=16, nesting=4):
- Currently says: "Reduce from 9 to ~10" ❌
- Should say: "Optional: Reduce from 9 to ~6 for future-proofing" ✅
- Or: "Moderate complexity (9/16). Below threshold but consider extraction for complex sections"

**`validate_config`** (cyclo=6, cognitive=6, nesting=1):
- Currently says: "Reduce from 6 to ~10" ❌
- Should say: "Maintain current low complexity" ✅
- Or: "Low complexity (6/6). Continue current patterns."

### Gradual Rollout Strategy

1. **Phase 1**: Fix ModerateComplexity pattern only
2. **Phase 2**: Validate no regressions in other patterns
3. **Phase 3**: Apply similar tier logic to other patterns if beneficial

### Edge Cases

**Boundary conditions**:
- cyclo=7 (just below moderate): Low tier, maintenance recommendation
- cyclo=8 (at moderate threshold): Moderate tier, optional refactoring
- cyclo=14 (top of moderate): Moderate tier, stronger refactoring recommendation
- cyclo=15 (entering high): High tier, required refactoring

## Migration and Compatibility

### Breaking Changes

None - this is a user-facing message fix, no API changes.

### Backward Compatibility

Fully compatible:
- Same recommendation data structures
- Same function signatures
- Only message text and logic improved

### Migration Path

No migration required - improved recommendations appear immediately on next analysis.

## Success Metrics

1. **Accuracy**: 0 instances of "Reduce from X to ~Y" where X < Y
2. **Clarity**: User feedback confirms recommendations make sense
3. **Adoption**: Blog post examples validate correct behavior
4. **Regression**: No changes to other pattern recommendations

## Rollback Plan

If issues discovered:

1. **Quick revert**: Restore original `generate_moderate_recommendation()` function
2. **Investigate**: Analyze which tier logic caused issues
3. **Refine**: Adjust tier thresholds or target calculations
4. **Re-deploy**: Test more thoroughly before re-deploying

## References

- **Bug Report**: Blog sample analysis identified "reduce from 9 to ~10" issue
- **Original Code**: `src/priority/scoring/concise_recommendation.rs:458-504`
- **Related**: Spec 176 (entropy fix), Spec 177 (pattern-aware recommendations)
- **Industry Standards**: Cyclomatic < 10, Cognitive < 25 (widely accepted thresholds)
