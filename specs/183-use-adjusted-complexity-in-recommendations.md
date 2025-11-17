---
number: 183
title: Use Adjusted Complexity in Recommendations and Output
category: foundation
priority: high
status: draft
dependencies: [182]
created: 2025-11-16
---

# Specification 183: Use Adjusted Complexity in Recommendations and Output

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 182 (Use Adjusted Complexity in Debt Classification)

## Context

After spec 182, `DebtType::ComplexityHotspot` will contain both raw and adjusted cyclomatic complexity. However, the recommendation generation and output formatting logic still uses raw complexity values, leading to confusing and incorrect messages.

**Current Problems**:

1. **"WHY THIS MATTERS" uses raw complexity**:
   ```
   WHY THIS MATTERS: Approaching complexity threshold (9/16).
   ```
   Should say: `Approaching complexity threshold (4/16)` using adjusted complexity.

2. **"RECOMMENDED ACTION" uses raw complexity**:
   ```
   RECOMMENDED ACTION: Reduce complexity from 9 to ~10
   ```
   Should say: `Reduce complexity from 4 to ~10` (or acknowledge it's already below threshold).

3. **Pattern display shows entropy but recommendations ignore it**:
   ```
   COMPLEXITY: cyclomatic=9 (dampened: 4, factor: 0.51), entropy=0.28
   WHY THIS MATTERS: Approaching complexity threshold (9/16).  ← Inconsistent!
   ```

**Real-World Example** (`state_reconciliation.rs:81`):
```
#1 SCORE: 4.15 [MEDIUM]
├─ LOCATION: ./src/state_reconciliation.rs:81 reconcile_state() [PRODUCTION]
├─ IMPACT: -4 complexity, -1.5 risk
├─ COMPLEXITY: cyclomatic=9 (dampened: 4, factor: 0.51), entropy=0.28
├─ WHY THIS MATTERS: Approaching complexity threshold (9/16).  ← WRONG (uses raw)
├─ RECOMMENDED ACTION: Reduce complexity from 9 to ~10      ← WRONG (suggests increase!)
```

**Expected Output**:
```
├─ WHY THIS MATTERS: Moderate cognitive complexity (16). Cyclomatic complexity is manageable (adjusted: 4).
├─ RECOMMENDED ACTION: Focus on reducing cognitive complexity through early returns and guard clauses
```

## Objective

Update recommendation generation and output formatting to consistently use adjusted complexity when available, providing accurate and actionable guidance that aligns with the entropy-adjusted scores.

## Requirements

### Functional Requirements

1. **Recommendations Use Adjusted Complexity**
   - `generate_moderate_recommendation()` uses `adjusted_cyclomatic` from `DebtType::ComplexityHotspot`
   - All other recommendation generators use adjusted values
   - Fall back to raw cyclomatic if `adjusted_cyclomatic` is None

2. **Rationale Messages Use Adjusted Complexity**
   - "Approaching complexity threshold (X/Y)" uses adjusted cyclomatic
   - Messages acknowledge when adjusted complexity is below threshold
   - Distinguish between cyclomatic and cognitive drivers

3. **Recommended Actions Use Adjusted Complexity**
   - "Reduce complexity from X to Y" uses adjusted cyclomatic
   - Don't suggest reducing complexity that's already below threshold
   - Focus recommendations on the actual driver (cognitive vs cyclomatic)

4. **Output Formatting Shows Both Raw and Adjusted**
   - Display format: `cyclomatic=9 (adjusted: 4, entropy: 0.28)`
   - Make it clear which value is used for scoring
   - Show dampening factor for transparency

### Non-Functional Requirements

- **Consistency**: All recommendations use the same complexity value (adjusted when available)
- **Clarity**: Users understand why adjusted complexity differs from raw
- **Backward Compatibility**: Functions without entropy analysis work as before
- **Maintainability**: Single source of truth for complexity value selection

## Acceptance Criteria

- [ ] `generate_moderate_recommendation()` uses `adjusted_cyclomatic` when available
- [ ] All recommendation generators (nesting, branching, mixed, etc.) use adjusted values
- [ ] Rationale messages use adjusted cyclomatic in threshold comparisons
- [ ] Recommended actions use adjusted cyclomatic in "reduce from X to Y" messages
- [ ] Output shows both raw and adjusted complexity clearly
- [ ] Functions with adjusted complexity < threshold get appropriate recommendations
- [ ] All existing tests pass
- [ ] New integration test validates end-to-end output with adjusted complexity
- [ ] Documentation explains adjusted complexity usage in recommendations

## Technical Details

### Implementation Approach

#### 1. Create Helper Function for Complexity Selection

**File**: `src/priority/scoring/concise_recommendation.rs`

Add a pure helper function to extract the effective complexity:

```rust
/// Get the effective cyclomatic complexity (adjusted if available, else raw)
fn get_effective_cyclomatic(debt_type: &DebtType) -> u32 {
    match debt_type {
        DebtType::ComplexityHotspot {
            cyclomatic,
            adjusted_cyclomatic,
            ..
        } => adjusted_cyclomatic.unwrap_or(*cyclomatic),
        DebtType::TestingGap { cyclomatic, .. } => *cyclomatic,
        DebtType::DeadCode { cyclomatic, .. } => *cyclomatic,
        DebtType::TestComplexityHotspot { cyclomatic, .. } => *cyclomatic,
        _ => 0, // Not applicable for other debt types
    }
}

/// Check if complexity was entropy-adjusted
fn is_complexity_adjusted(debt_type: &DebtType) -> bool {
    match debt_type {
        DebtType::ComplexityHotspot {
            adjusted_cyclomatic,
            ..
        } => adjusted_cyclomatic.is_some(),
        _ => false,
    }
}
```

#### 2. Update `generate_moderate_recommendation()`

**File**: `src/priority/scoring/concise_recommendation.rs:456-503`

**Before**:
```rust
fn generate_moderate_recommendation(
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let target_complexity = 10;
    let complexity_reduction = cyclomatic.saturating_sub(target_complexity).max(5);

    // ... steps ...

    ActionableRecommendation {
        primary_action: format!(
            "Reduce complexity from {} to ~{}",
            cyclomatic, target_complexity  // ← Uses raw cyclomatic
        ),
        rationale: format!(
            "Approaching complexity threshold ({}/{}). \
             Preventive refactoring will keep code maintainable.",
            cyclomatic, cognitive  // ← Uses raw cyclomatic
        ),
        // ...
    }
}
```

**After**:
```rust
fn generate_moderate_recommendation(
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    // Use adjusted complexity if available (spec 183)
    let effective_cyclomatic = metrics.adjusted_complexity
        .map(|adj| adj.round() as u32)
        .unwrap_or(cyclomatic);

    let target_complexity = 10;
    let complexity_reduction = effective_cyclomatic.saturating_sub(target_complexity).max(5);

    // Generate appropriate rationale based on complexity driver
    let rationale = if effective_cyclomatic < target_complexity {
        // Adjusted complexity is actually below threshold - focus on cognitive
        format!(
            "Moderate cognitive complexity ({}) indicates nested logic. \
             Cyclomatic complexity is manageable (adjusted: {}).",
            cognitive, effective_cyclomatic
        )
    } else {
        // Both cyclomatic and cognitive need attention
        format!(
            "Approaching complexity threshold ({}/{}). \
             Preventive refactoring will keep code maintainable.",
            effective_cyclomatic, cognitive
        )
    };

    // Generate appropriate action based on complexity driver
    let primary_action = if effective_cyclomatic < target_complexity {
        // Focus on cognitive complexity (nesting, early returns)
        "Reduce cognitive complexity through early returns and guard clauses".to_string()
    } else {
        // Focus on cyclomatic complexity (extract functions)
        format!(
            "Reduce complexity from {} to ~{}",
            effective_cyclomatic, target_complexity
        )
    };

    // ... rest of function with adjusted steps ...

    ActionableRecommendation {
        primary_action,
        rationale,
        // ...
    }
}
```

**Key Changes**:
1. Calculate `effective_cyclomatic` from `metrics.adjusted_complexity`
2. Different rationale when adjusted complexity is below threshold
3. Different action when only cognitive complexity is high
4. Use `effective_cyclomatic` consistently throughout

#### 3. Update All Recommendation Generators

Apply similar pattern to all complexity recommendation functions:

**Files to Update**:
- `generate_nesting_recommendation()` (line 192-253)
- `generate_branching_recommendation()` (line 300-355)
- `generate_mixed_recommendation()` (line 357-409)
- `generate_chaotic_recommendation()` (line 411-454)
- `generate_testing_gap_steps()` (line 59-148)

**Pattern**:
```rust
fn generate_X_recommendation(..., metrics: &FunctionMetrics) -> ActionableRecommendation {
    // Extract effective complexity early
    let effective_cyclomatic = metrics.adjusted_complexity
        .map(|adj| adj.round() as u32)
        .unwrap_or(cyclomatic);

    // Use effective_cyclomatic in calculations and messages
    let complexity_reduction = effective_cyclomatic.saturating_sub(target);

    // Update rationale to use effective_cyclomatic
    let rationale = format!("... complexity {} ...", effective_cyclomatic);

    // Update impact estimates to use effective_cyclomatic
    // ...
}
```

#### 4. Update Rationale Section Formatting

**File**: `src/priority/formatter/sections.rs:285-294`

Enhance the rationale formatting to show adjustment when applicable:

```rust
fn format_rationale_section(context: &FormatContext) -> String {
    let mut rationale = context.rationale.clone();

    // Add entropy adjustment note if applicable (spec 183)
    if let Some(ref entropy) = context.complexity_info.entropy_details {
        if entropy.dampening_factor < 0.8 {
            rationale.push_str(&format!(
                " (Complexity dampened by {:.0}% due to low entropy)",
                (1.0 - entropy.dampening_factor) * 100.0
            ));
        }
    }

    format!(
        "{} {}",
        "├─ WHY THIS MATTERS:".bright_blue(),
        rationale
    )
}
```

#### 5. Update Complexity Display Format

**File**: `src/priority/formatter/sections.rs:98-126`

Enhance complexity section to emphasize adjusted value:

**Before**:
```rust
Some(format!(
    "{} cyclomatic={} (dampened: {}, factor: {:.2}), ...",
    "├─ COMPLEXITY:".bright_blue(),
    context.complexity_info.cyclomatic,
    entropy.adjusted_complexity,
    entropy.dampening_factor,
    // ...
))
```

**After**:
```rust
Some(format!(
    "{} cyclomatic={} → {} (entropy-adjusted, factor: {:.2}), ...",
    "├─ COMPLEXITY:".bright_blue(),
    context.complexity_info.cyclomatic,
    entropy.adjusted_complexity.to_string().bright_green().bold(),  // Emphasize adjusted
    entropy.dampening_factor,
    // ...
))
```

**Rationale**: Make it visually clear that the adjusted value is used for scoring.

### Architecture Changes

**Modified Modules**:
1. `src/priority/scoring/concise_recommendation.rs` - All recommendation generators
2. `src/priority/formatter/sections.rs` - Rationale and complexity formatting
3. `src/priority/scoring/recommendation_helpers.rs` - Helper functions (if exists)

**Data Flow**:
```
DebtType::ComplexityHotspot {
    cyclomatic: 9,
    cognitive: 16,
    adjusted_cyclomatic: Some(4),  ← From spec 182
}
    ↓
generate_moderate_recommendation()
    ↓ (uses adjusted_cyclomatic=4)
ActionableRecommendation {
    primary_action: "Reduce cognitive complexity...",  ← Uses adjusted value
    rationale: "Moderate cognitive (16), cyclomatic manageable (4)",  ← Uses adjusted
}
    ↓
format_rationale_section()
    ↓
Output: "WHY THIS MATTERS: Moderate cognitive (16), cyclomatic manageable (4)"
```

### Example Output Changes

**Input**:
```rust
DebtType::ComplexityHotspot {
    cyclomatic: 9,
    cognitive: 16,
    adjusted_cyclomatic: Some(4),
}
```

**Before Spec 183**:
```
#1 SCORE: 4.15 [MEDIUM]
├─ COMPLEXITY: cyclomatic=9 (dampened: 4, factor: 0.51), cognitive=16, entropy=0.28
├─ WHY THIS MATTERS: Approaching complexity threshold (9/16). Preventive refactoring will keep code maintainable.
├─ RECOMMENDED ACTION: Reduce complexity from 9 to ~10
```

**After Spec 183**:
```
#1 SCORE: 4.15 [MEDIUM]
├─ COMPLEXITY: cyclomatic=9 → 4 (entropy-adjusted, factor: 0.51), cognitive=16, entropy=0.28
├─ WHY THIS MATTERS: Moderate cognitive complexity (16) indicates nested logic. Cyclomatic complexity is manageable (adjusted: 4).
├─ RECOMMENDED ACTION: Reduce cognitive complexity through early returns and guard clauses
```

**Key Improvements**:
1. Rationale acknowledges adjusted complexity is below threshold
2. Action focuses on the real driver (cognitive complexity)
3. No suggestion to "reduce from 9 to 10" (nonsensical increase)
4. Output clearly shows which value is used (4, not 9)

## Dependencies

**Prerequisites**:
- **Spec 182**: Must be implemented first (provides `adjusted_cyclomatic` in `DebtType`)

**Affected Components**:
- All recommendation generation functions
- Output formatters (terminal, markdown, JSON)
- Documentation explaining recommendations

**Related Specifications**:
- **Spec 176**: Pattern-based complexity recommendations
- **Spec 177**: Role-aware complexity recommendations
- **Spec 178**: Fix moderate complexity recommendation logic

## Testing Strategy

### Unit Tests

**File**: `src/priority/scoring/concise_recommendation.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_moderate_recommendation_uses_adjusted_complexity() {
        let metrics = create_test_metrics(9, 16);
        metrics.adjusted_complexity = Some(4.15);

        let rec = generate_moderate_recommendation(9, 16, &metrics);

        // Should NOT suggest reducing from 9 to 10
        assert!(!rec.primary_action.contains("9 to"));

        // Should mention adjusted complexity in rationale
        assert!(rec.rationale.contains("adjusted: 4") || rec.rationale.contains("manageable"));

        // Should focus on cognitive complexity
        assert!(rec.primary_action.contains("cognitive") || rec.primary_action.contains("early returns"));
    }

    #[test]
    fn generate_moderate_recommendation_falls_back_to_raw() {
        let metrics = create_test_metrics(12, 18);
        metrics.adjusted_complexity = None;  // No adjustment

        let rec = generate_moderate_recommendation(12, 18, &metrics);

        // Should use raw cyclomatic in messages
        assert!(rec.rationale.contains("12"));
        assert!(rec.primary_action.contains("12 to"));
    }

    #[test]
    fn get_effective_cyclomatic_prefers_adjusted() {
        let debt = DebtType::ComplexityHotspot {
            cyclomatic: 9,
            cognitive: 16,
            adjusted_cyclomatic: Some(4),
        };

        assert_eq!(get_effective_cyclomatic(&debt), 4);
    }

    #[test]
    fn get_effective_cyclomatic_falls_back_to_raw() {
        let debt = DebtType::ComplexityHotspot {
            cyclomatic: 9,
            cognitive: 16,
            adjusted_cyclomatic: None,
        };

        assert_eq!(get_effective_cyclomatic(&debt), 9);
    }
}
```

### Integration Tests

**File**: `tests/adjusted_complexity_recommendation_test.rs`

```rust
#[test]
fn end_to_end_adjusted_complexity_recommendation() {
    let source = r#"
        pub fn reconcile_state(current: State, target: State) -> Result<Vec<Action>> {
            let mut actions = vec![];
            if current.mode != target.mode {
                if current.has_active_connections() {
                    if target.mode == Mode::Offline {
                        actions.push(drain_connections());
                        if current.has_pending_writes() {
                            actions.push(flush_writes());
                        }
                    }
                } else if target.allows_reconnect() {
                    actions.push(establish_connections());
                }
            }
            Ok(actions)
        }
    "#;

    let result = analyze_and_format(source, "test.rs", OutputFormat::Terminal);

    // Should show adjusted complexity in output
    assert!(result.contains("dampened:") || result.contains("adjusted:"));

    // Should NOT suggest "reduce from 9 to 10"
    assert!(!result.contains("9 to ~10"));

    // Should focus on cognitive complexity
    assert!(result.contains("cognitive") || result.contains("early returns"));

    // Rationale should mention adjusted complexity
    assert!(result.contains("manageable") || result.contains("adjusted: 4"));
}

#[test]
fn functions_without_adjustment_unchanged() {
    // Function without entropy analysis should behave as before
    let source = "fn simple(x: i32) -> i32 { x + 1 }";

    let before = analyze_and_format_v1(source, "test.rs");  // Old version
    let after = analyze_and_format_v2(source, "test.rs");   // With spec 183

    // Recommendations should be identical (no adjusted complexity)
    assert_eq!(before, after);
}
```

### Output Format Tests

**File**: `tests/output_format_tests.rs`

```rust
#[test]
fn complexity_section_shows_adjusted_value_prominently() {
    let context = FormatContext {
        complexity_info: ComplexityInfo {
            cyclomatic: 9,
            entropy_details: Some(EntropyDetails {
                adjusted_complexity: 4,
                dampening_factor: 0.51,
                entropy_score: 0.28,
            }),
            // ...
        },
        // ...
    };

    let output = format_complexity_section(&context).unwrap();

    // Should show "9 → 4" or similar
    assert!(output.contains("9") && output.contains("4"));

    // Should emphasize adjusted value (implementation detail)
    assert!(output.contains("adjusted") || output.contains("dampened"));
}
```

## Documentation Requirements

### Code Documentation

1. **Document `get_effective_cyclomatic()`**:
   ```rust
   /// Get the effective cyclomatic complexity for recommendations.
   ///
   /// Uses entropy-adjusted complexity when available (spec 183), falling back
   /// to raw cyclomatic complexity otherwise. This ensures recommendations are
   /// based on the actual cognitive load of the function, not just raw metrics.
   ///
   /// # Spec 183 Rationale
   /// Functions with low entropy (repetitive patterns) have dampened cyclomatic
   /// complexity that better reflects their understandability. Using adjusted
   /// complexity prevents recommending unnecessary refactoring for functions
   /// that are already easy to understand.
   ```

2. **Update recommendation function docstrings**:
   ```rust
   /// Generate moderate complexity recommendation.
   ///
   /// Uses entropy-adjusted cyclomatic complexity when available (spec 183).
   /// Distinguishes between cyclomatic and cognitive drivers to provide
   /// targeted refactoring guidance.
   ```

### User Documentation

**Update `book/src/entropy-analysis.md`**:

```markdown
## Entropy-Adjusted Recommendations

Debtmap uses entropy-adjusted complexity scores in recommendations to provide
accurate, actionable guidance.

### Example: Low Entropy Function

**Code**: `state_reconciliation.rs:81` (`reconcile_state`)
- Raw cyclomatic: 9
- Cognitive: 16
- Entropy: 0.28 (low - repetitive pattern)
- Adjusted cyclomatic: 4

**Recommendation** (using adjusted complexity):
```
WHY THIS MATTERS: Moderate cognitive complexity (16) indicates nested logic.
Cyclomatic complexity is manageable (adjusted: 4).

RECOMMENDED ACTION: Reduce cognitive complexity through early returns and guard clauses
```

**Rationale**: The adjusted complexity (4) is below the threshold, so the
recommendation focuses on the actual driver (cognitive complexity from nesting)
rather than suggesting unnecessary cyclomatic complexity reduction.

### How Adjustment Works

1. **Classification**: Uses adjusted complexity for threshold comparison (spec 182)
2. **Recommendations**: All messages use adjusted complexity (spec 183)
3. **Output**: Shows both raw and adjusted for transparency

**Visual Format**:
```
COMPLEXITY: cyclomatic=9 → 4 (entropy-adjusted, factor: 0.51), cognitive=16
                       ↑
                  Used for recommendations
```
```

**Update `book/src/troubleshooting.md`**:

```markdown
## "Reduce complexity from X to Y" suggests increasing complexity

**Symptom**: Recommendation says "Reduce complexity from 9 to ~10" (increase).

**Cause** (before spec 183): Recommendations used raw cyclomatic complexity
instead of entropy-adjusted complexity.

**Solution** (spec 183): Upgrade to version with spec 183 implemented.
Recommendations now use adjusted complexity and provide appropriate guidance
based on the actual complexity driver.

**Example**:
- Before: "Reduce complexity from 9 to ~10" (confusing)
- After: "Reduce cognitive complexity through early returns" (actionable)
```

## Implementation Notes

### Prioritizing Complexity Drivers

When generating recommendations, prioritize based on which metric is high:

1. **Adjusted cyclomatic < threshold AND cognitive < threshold**: No recommendation needed
2. **Adjusted cyclomatic < threshold AND cognitive > threshold**: Focus on cognitive (nesting, early returns)
3. **Adjusted cyclomatic > threshold AND cognitive < threshold**: Focus on cyclomatic (extract functions)
4. **Both high**: Address both with phased approach

### Helper Function for Driver Detection

```rust
enum ComplexityDriver {
    Cyclomatic,
    Cognitive,
    Both,
    Neither,
}

fn identify_complexity_driver(
    effective_cyclomatic: u32,
    cognitive: u32,
) -> ComplexityDriver {
    let cyclomatic_high = effective_cyclomatic > 10;
    let cognitive_high = cognitive > 15;

    match (cyclomatic_high, cognitive_high) {
        (true, true) => ComplexityDriver::Both,
        (true, false) => ComplexityDriver::Cyclomatic,
        (false, true) => ComplexityDriver::Cognitive,
        (false, false) => ComplexityDriver::Neither,
    }
}
```

### Backward Compatibility

Functions without `adjusted_complexity`:
- `get_effective_cyclomatic()` returns raw `cyclomatic`
- Recommendations generated as before
- No behavior change for unadjusted functions

### Testing Coverage

- Unit tests for each recommendation generator
- Integration tests for end-to-end output
- Regression tests for functions without adjustment
- Format tests for visual output

## Success Metrics

### Quantitative

- **Message accuracy**: 100% of recommendations use adjusted complexity when available
- **Nonsensical suggestions**: 0% of "reduce from X to Y" where X < Y
- **Consistency**: 100% of output sections use same complexity value
- **Test coverage**: >= 95% for modified code paths

### Qualitative

- Developers report recommendations are more actionable
- Reduced confusion about "approaching threshold" messages
- Better alignment between displayed metrics and recommended actions

### Validation

1. **Audit existing corpus**: Run on 100+ functions with entropy adjustment
2. **Message review**: Sample 50 recommendations to verify consistency
3. **User feedback**: Survey 5+ users on clarity and usefulness
4. **Dogfooding**: Use on debtmap itself to validate recommendations

## Open Questions

1. **Should we always show both raw and adjusted in recommendations?**
   - Current approach: Only mention adjusted when different from raw
   - Alternative: Always show both for transparency
   - Decision: Mention adjusted when it differs significantly (>20% difference)

2. **What threshold difference warrants mentioning adjustment?**
   - Current approach: Always use adjusted, mention in rationale
   - Alternative: Only mention if adjustment changes classification
   - Decision: Always use adjusted, explain in output format

3. **Should cognitive complexity ever be entropy-adjusted?**
   - Current approach: No (spec 183 scope)
   - Alternative: Research and implement in future spec
   - Decision: Out of scope for spec 183, revisit in spec 184+

## Future Enhancements

1. **Visual highlighting of adjusted values** in terminal output
2. **Trend analysis**: Show how entropy adjustment changes over time
3. **Configurable adjustment thresholds** per project
4. **Machine learning-based complexity adjustment** (beyond entropy)

## Related Work

- **Spec 182**: Use adjusted complexity in classification (prerequisite)
- **Spec 178**: Fix moderate complexity recommendation logic
- **Spec 177**: Role-aware complexity recommendations
- **Spec 176**: Pattern-based complexity recommendations
- **Entropy analysis**: `src/complexity/entropy_core.rs`

## References

- Shannon entropy: Shannon, C. E. (1948). "A Mathematical Theory of Communication"
- Cyclomatic complexity: McCabe, T. J. (1976). "A Complexity Measure"
- Cognitive complexity: SonarSource (2016). "Cognitive Complexity: A new way of measuring understandability"
- Debtmap entropy implementation: `book/src/entropy-analysis.md`
