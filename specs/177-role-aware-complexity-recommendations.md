---
number: 177
title: Role-Aware Complexity Recommendations
category: foundation
priority: high
status: draft
dependencies: [176, 117, 110]
created: 2025-11-16
---

# Specification 177: Role-Aware Complexity Recommendations

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 176 (Entropy Fix), Spec 117 (Semantic Classification), Spec 110 (Orchestration Adjustment)

## Context

Debtmap has sophisticated pattern detection that identifies function roles:
- **Orchestrator**: Coordinates multiple concerns (spec 117, 110, 126)
- **PatternMatch**: Visitor/interpreter patterns with match statements
- **IOWrapper**: Constructors, accessors, enum converters (spec 122, 124, 125)
- **Debug**: Diagnostic functions (spec 119)
- **PureLogic**: Business logic requiring high test coverage
- **EntryPoint**: Main entry points

However, **role information doesn't flow to complexity recommendations**. This causes inappropriate advice:

### Real-World Example from Prodigy

**Function**: `execute_remaining_steps()` (orchestrator coordinating 5 concerns)
- **Role Detected**: Orchestrator ✅
- **Score Adjusted**: -20% reduction ✅
- **Recommendation**: "Reduce nesting from 4 to 2 levels" ❌ (generic, ignores orchestration pattern)

**Better Recommendation** (role-aware):
```
├─ WHY THIS MATTERS: This orchestrator coordinates 5 concerns (checkpoint,
   progress, iteration, errors, saving). Nesting from error handling is expected.
├─ RECOMMENDED ACTION: Improve orchestration structure while preserving coordination
   - Extract complex helper logic (not core coordination)
   - Add documentation listing orchestrated concerns
   - Impact: -10 to -15 cognitive complexity (not -35)
```

### Problem Statement

The disconnect between pattern detection and recommendations leads to:
1. **Inappropriate advice** for orchestrators ("split this coordination logic")
2. **Missing context** about why complexity is justified
3. **Overpromised impact** (claiming -35 cognitive when only -10 is realistic)
4. **User confusion** about whether to follow recommendations

### Evidence

From "debtmap-questionable-recommendations.md" analysis:
- 50% of top 10 recommendations target essential complexity
- Orchestrators get treated like god objects
- Pattern matchers get "split function" advice when structure is correct
- Users document why they're ignoring recommendations

## Objective

Wire existing pattern detection to recommendation generation so that:
1. Orchestrators get orchestration-aware refactoring advice
2. Pattern matchers get visitor-pattern-appropriate guidance
3. IO wrappers get minimal or no refactoring recommendations
4. Pure logic gets current comprehensive recommendations
5. Impact estimates reflect realistic improvements for each role

## Requirements

### Functional Requirements

**FR1: Pass Role to Complexity Recommendation Generation**
- `generate_complexity_steps()` must receive `FunctionRole` parameter
- Role must be available from debt item creation through recommendation
- All existing tests must continue to pass with role parameter added

**FR2: Role + Pattern Matching Logic**
- Recommendations must consider BOTH complexity pattern AND function role
- Match on `(ComplexityPattern, FunctionRole)` tuple
- Provide specific guidance for each combination

**FR3: Role-Specific Recommendations for Orchestrators**
- **HighNesting + Orchestrator**: "Extract helpers, document concerns"
- **HighBranching + Orchestrator**: "Extract phases, preserve coordination"
- **MixedComplexity + Orchestrator**: "Two-phase: extract helpers first, then phases"
- **Chaotic + Orchestrator**: "Standardize error handling patterns"

**FR4: Role-Specific Recommendations for Pattern Matchers**
- **HighBranching + PatternMatch**: "Structure is correct, add tests per variant"
- **MixedComplexity + PatternMatch**: "Consider trait-based visitor for extensibility"
- Do not recommend splitting match statements

**FR5: Minimal Recommendations for IO Wrappers**
- **Any pattern + IOWrapper**: Lower priority or skip recommendation
- Focus on "ensure tests exist" not "refactor structure"
- Acknowledge these are thin adapters

**FR6: Realistic Impact Estimation**
- Orchestrator complexity reduction: 10-15 points (not 30-35)
- Pattern matcher: 0-5 points (structure is correct)
- IO wrapper: 0 points (no refactoring recommended)
- Pure logic: Use current impact estimates

### Non-Functional Requirements

**NFR1: Backward Compatibility**
- Existing recommendations for PureLogic must not change significantly
- Score calculations remain unchanged (already adjusted by spec 110)
- Output format remains consistent

**NFR2: Code Clarity**
- Pattern + role matching logic must be explicit and readable
- Use `match (pattern, role)` not nested if-else
- Document each combination with rationale

**NFR3: Test Coverage**
- Unit tests for each (pattern, role) combination
- Integration tests with real orchestrators from Prodigy codebase
- Regression tests ensuring PureLogic recommendations unchanged

## Acceptance Criteria

- [ ] `generate_complexity_steps()` accepts `role: FunctionRole` parameter
- [ ] `generate_concise_recommendation()` passes role to complexity generation
- [ ] Orchestrator + HighNesting generates "extract helpers" recommendation
- [ ] Orchestrator + HighBranching generates "extract phases" recommendation
- [ ] PatternMatch + HighBranching generates "structure is correct" recommendation
- [ ] IOWrapper + any pattern generates minimal recommendation
- [ ] PureLogic + any pattern uses current recommendations (unchanged)
- [ ] Impact estimates reflect realistic improvements per role
- [ ] All existing unit tests pass with role parameter added
- [ ] Integration test validates Prodigy orchestrators get appropriate advice
- [ ] Documentation updated explaining role-aware recommendations
- [ ] No breaking changes to output format or score calculation

## Technical Details

### Implementation Approach

**Phase 1: Wire Role Through Call Chain**

```rust
// 1. Update function signature
pub fn generate_complexity_steps(
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
    role: FunctionRole,  // ← NEW PARAMETER
) -> ActionableRecommendation {
    let complexity_metrics = ComplexityMetrics {
        cyclomatic,
        cognitive,
        nesting: metrics.nesting,
        entropy_score: metrics.entropy_score.as_ref().map(|e| e.token_entropy),
    };

    let pattern = ComplexityPattern::detect(&complexity_metrics);

    // 2. Match on both pattern and role
    generate_role_aware_recommendation(pattern, role, cyclomatic, cognitive, metrics)
}

// 3. Update caller
pub fn generate_concise_recommendation(
    debt_type: &DebtType,
    metrics: &FunctionMetrics,
    role: FunctionRole,
    coverage: &Option<TransitiveCoverage>,
) -> ActionableRecommendation {
    match debt_type {
        DebtType::ComplexityHotspot { cyclomatic, cognitive } =>
            generate_complexity_steps(*cyclomatic, *cognitive, metrics, role),  // ← PASS ROLE
        // ... other cases
    }
}
```

**Phase 2: Implement Role-Aware Recommendation Generation**

```rust
fn generate_role_aware_recommendation(
    pattern: ComplexityPattern,
    role: FunctionRole,
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    match (pattern, role) {
        // ORCHESTRATORS: Context-aware advice
        (ComplexityPattern::HighNesting { nesting_depth, cognitive_score, ratio },
         FunctionRole::Orchestrator) => {
            generate_orchestrator_nesting_recommendation(nesting_depth, cognitive_score, ratio, metrics)
        },

        (ComplexityPattern::HighBranching { branch_count, .. },
         FunctionRole::Orchestrator) => {
            generate_orchestrator_branching_recommendation(branch_count, cyclomatic, metrics)
        },

        (ComplexityPattern::MixedComplexity { .. },
         FunctionRole::Orchestrator) => {
            generate_orchestrator_mixed_recommendation(cyclomatic, cognitive, metrics)
        },

        (ComplexityPattern::ChaoticStructure { entropy, .. },
         FunctionRole::Orchestrator) => {
            generate_orchestrator_chaotic_recommendation(entropy, cyclomatic, cognitive, metrics)
        },

        // PATTERN MATCHERS: Acknowledge correct structure
        (ComplexityPattern::HighBranching { branch_count, .. },
         FunctionRole::PatternMatch) => {
            generate_pattern_match_recommendation(branch_count, cyclomatic, metrics)
        },

        (ComplexityPattern::MixedComplexity { .. },
         FunctionRole::PatternMatch) => {
            generate_pattern_match_extensibility_recommendation(cyclomatic, cognitive, metrics)
        },

        // IO WRAPPERS: Minimal recommendations
        (_, FunctionRole::IOWrapper) => {
            generate_io_wrapper_recommendation(cyclomatic, cognitive, metrics)
        },

        // PURE LOGIC: Use existing pattern-based recommendations
        (pattern, FunctionRole::PureLogic | FunctionRole::EntryPoint |
                  FunctionRole::Debug | FunctionRole::Unknown) => {
            generate_pattern_recommendation(pattern, cyclomatic, cognitive, metrics)
        },
    }
}
```

**Phase 3: Implement Orchestrator-Specific Recommendations**

```rust
/// Generate recommendation for orchestrator with high nesting
fn generate_orchestrator_nesting_recommendation(
    nesting: u32,
    cognitive: u32,
    ratio: f64,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    // Realistic impact: orchestrators can reduce 10-15 points, not 30-35
    let realistic_reduction = ((cognitive as f64 * 0.2) as u32).min(15).max(10);

    let steps = vec![
        ActionStep {
            description: "Extract complex helper logic (preserve orchestration flow)".to_string(),
            impact: format!("-{} cognitive complexity", realistic_reduction),
            difficulty: Difficulty::Medium,
            commands: vec![
                "# Extract error handling helpers".to_string(),
                "# Extract validation logic".to_string(),
                "# Keep coordination in main function".to_string(),
            ],
        },
        ActionStep {
            description: "Add architectural documentation".to_string(),
            impact: "Improved maintainability".to_string(),
            difficulty: Difficulty::Easy,
            commands: vec![
                "/// This orchestrator coordinates:".to_string(),
                "/// 1. [Concern 1]".to_string(),
                "/// 2. [Concern 2]".to_string(),
                "/// ... [List all concerns]".to_string(),
            ],
        },
        ActionStep {
            description: "Verify coordination logic preserved".to_string(),
            impact: "Ensured correctness".to_string(),
            difficulty: Difficulty::Easy,
            commands: vec![
                "cargo test --all".to_string(),
            ],
        },
    ];

    ActionableRecommendation {
        primary_action: format!(
            "Improve orchestration structure (realistic impact: -{})",
            realistic_reduction
        ),
        rationale: format!(
            "This orchestrator coordinates multiple concerns. Deep nesting (depth {}) \
             is common in orchestration. Cognitive/Cyclomatic ratio of {:.1}x confirms \
             orchestration pattern. Focus on extracting helpers, not splitting coordination.",
            nesting, ratio
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(2.0),
    }
}

/// Generate recommendation for pattern matcher
fn generate_pattern_match_recommendation(
    branch_count: u32,
    cyclomatic: u32,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    let steps = vec![
        ActionStep {
            description: "Add tests for each pattern variant".to_string(),
            impact: format!("Coverage for {} branches", branch_count),
            difficulty: Difficulty::Medium,
            commands: vec![
                format!("# Write {} test cases (one per variant)", branch_count),
            ],
        },
        ActionStep {
            description: "Consider trait-based visitor if variants > 15".to_string(),
            impact: "Better extensibility (if needed)".to_string(),
            difficulty: Difficulty::Hard,
            commands: vec![
                "# Only if pattern matching becomes unwieldy".to_string(),
            ],
        },
    ];

    ActionableRecommendation {
        primary_action: "Structure is appropriate for pattern matching".to_string(),
        rationale: format!(
            "This visitor/pattern matcher handles {} variants using match statement. \
             High branching ({} branches) is expected and acceptable for this pattern. \
             Focus on test coverage, not refactoring structure.",
            branch_count, cyclomatic
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(steps),
        estimated_effort_hours: Some(1.0),
    }
}

/// Generate minimal recommendation for IO wrapper
fn generate_io_wrapper_recommendation(
    cyclomatic: u32,
    cognitive: u32,
    _metrics: &FunctionMetrics,
) -> ActionableRecommendation {
    ActionableRecommendation {
        primary_action: "Ensure test coverage exists".to_string(),
        rationale: format!(
            "This I/O wrapper has complexity {}/{}. As a thin adapter, \
             structural complexity is acceptable. Focus on ensuring tests exist.",
            cyclomatic, cognitive
        ),
        implementation_steps: vec![],
        related_items: vec![],
        steps: Some(vec![
            ActionStep {
                description: "Add basic functionality tests".to_string(),
                impact: "Verified correct behavior".to_string(),
                difficulty: Difficulty::Easy,
                commands: vec!["# Test happy path and error cases".to_string()],
            },
        ]),
        estimated_effort_hours: Some(0.5),
    }
}
```

### Architecture Changes

**Modified Files**:
1. `src/priority/scoring/concise_recommendation.rs`:
   - Add `role` parameter to `generate_complexity_steps()`
   - Implement `generate_role_aware_recommendation()`
   - Add orchestrator-specific generators
   - Add pattern-match-specific generators

2. `src/priority/scoring/mod.rs`:
   - Update exports if needed

3. `src/priority/scoring/recommendation_helpers.rs`:
   - Add helper functions for impact estimation per role

**New Files** (optional for organization):
- `src/priority/scoring/orchestrator_recommendations.rs`: Orchestrator-specific logic
- `src/priority/scoring/pattern_match_recommendations.rs`: Pattern matcher logic

### Data Structures

No new data structures needed. Existing enums are sufficient:
- `FunctionRole` (already exists)
- `ComplexityPattern` (already exists)
- `ActionableRecommendation` (already exists)

### APIs and Interfaces

**Public API Changes**:

```rust
// BEFORE:
pub fn generate_complexity_steps(
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
) -> ActionableRecommendation

// AFTER:
pub fn generate_complexity_steps(
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
    role: FunctionRole,  // ← NEW
) -> ActionableRecommendation
```

**Internal API Additions**:

```rust
// New internal functions
fn generate_role_aware_recommendation(...) -> ActionableRecommendation
fn generate_orchestrator_nesting_recommendation(...) -> ActionableRecommendation
fn generate_orchestrator_branching_recommendation(...) -> ActionableRecommendation
fn generate_orchestrator_mixed_recommendation(...) -> ActionableRecommendation
fn generate_orchestrator_chaotic_recommendation(...) -> ActionableRecommendation
fn generate_pattern_match_recommendation(...) -> ActionableRecommendation
fn generate_pattern_match_extensibility_recommendation(...) -> ActionableRecommendation
fn generate_io_wrapper_recommendation(...) -> ActionableRecommendation
```

## Dependencies

**Prerequisites**:
- Spec 176: Entropy fix (ensures correct pattern detection)
- Spec 117: Semantic classification (provides role detection)
- Spec 110: Orchestration adjustment (provides score reduction)

**Affected Components**:
- `src/priority/scoring/concise_recommendation.rs`: Main changes
- `src/priority/scoring/recommendation.rs`: May need updates
- `src/priority/unified_scorer.rs`: Passes role through pipeline
- Tests in `tests/concise_recommendations_*.rs`: Need role parameter

**External Dependencies**: None

## Testing Strategy

### Unit Tests

**Test 1: Role Parameter Wiring**

```rust
#[test]
fn test_generate_complexity_steps_accepts_role() {
    let metrics = create_test_metrics(15, 50, 4);

    // Should compile and run with role parameter
    let rec = generate_complexity_steps(
        15,
        50,
        &metrics,
        FunctionRole::Orchestrator,  // ← NEW
    );

    assert!(!rec.primary_action.is_empty());
}
```

**Test 2: Orchestrator + HighNesting**

```rust
#[test]
fn test_orchestrator_high_nesting_recommendation() {
    let metrics = create_orchestrator_metrics(15, 65, 4);

    let rec = generate_complexity_steps(
        15,
        65,
        &metrics,
        FunctionRole::Orchestrator,
    );

    // Should mention orchestration, not generic "reduce nesting"
    assert!(rec.rationale.contains("orchestrator") || rec.rationale.contains("orchestration"));
    assert!(rec.primary_action.contains("orchestration") ||
            rec.primary_action.contains("helper") ||
            rec.primary_action.contains("coordination"));

    // Impact should be realistic (10-15), not overpromised (30-35)
    if let Some(steps) = &rec.steps {
        if let Some(first_step) = steps.first() {
            // Extract number from "-15 cognitive" or similar
            let impact_str = &first_step.impact;
            if impact_str.contains("cognitive") {
                assert!(!impact_str.contains("-30") && !impact_str.contains("-35"),
                    "Impact should be realistic, not overpromised: {}", impact_str);
            }
        }
    }
}
```

**Test 3: PatternMatch + HighBranching**

```rust
#[test]
fn test_pattern_match_high_branching_recommendation() {
    let metrics = create_pattern_match_metrics(21, 51, 3);

    let rec = generate_complexity_steps(
        21,
        51,
        &metrics,
        FunctionRole::PatternMatch,
    );

    // Should acknowledge structure is correct
    assert!(rec.rationale.contains("pattern match") ||
            rec.rationale.contains("visitor") ||
            rec.rationale.contains("acceptable") ||
            rec.rationale.contains("appropriate"));

    // Should not recommend splitting
    assert!(!rec.primary_action.to_lowercase().contains("split"));
    assert!(!rec.primary_action.to_lowercase().contains("extract functions"));

    // Should focus on tests
    if let Some(steps) = &rec.steps {
        assert!(steps.iter().any(|s| s.description.contains("test")));
    }
}
```

**Test 4: IOWrapper Minimal Recommendation**

```rust
#[test]
fn test_io_wrapper_minimal_recommendation() {
    let metrics = create_io_wrapper_metrics(5, 8, 1);

    let rec = generate_complexity_steps(
        5,
        8,
        &metrics,
        FunctionRole::IOWrapper,
    );

    // Should have minimal recommendation
    assert!(rec.primary_action.contains("test") ||
            rec.primary_action.contains("coverage") ||
            rec.primary_action.contains("appropriate"));

    // Should acknowledge it's a wrapper
    assert!(rec.rationale.contains("wrapper") ||
            rec.rationale.contains("adapter") ||
            rec.rationale.contains("acceptable"));
}
```

**Test 5: PureLogic Unchanged**

```rust
#[test]
fn test_pure_logic_recommendations_unchanged() {
    let metrics = create_pure_logic_metrics(18, 54, 4);

    let rec_with_role = generate_complexity_steps(
        18,
        54,
        &metrics,
        FunctionRole::PureLogic,
    );

    // Should use same recommendations as before (from pattern detection)
    // This ensures backward compatibility
    assert!(!rec_with_role.primary_action.is_empty());

    // Compare to current behavior (will match pattern-based recommendations)
    let pattern = ComplexityPattern::detect(&ComplexityMetrics {
        cyclomatic: 18,
        cognitive: 54,
        nesting: 4,
        entropy_score: None,
    });

    // Should match pattern-based recommendation
    match pattern {
        ComplexityPattern::HighNesting { .. } => {
            assert!(rec_with_role.primary_action.contains("nesting"));
        },
        ComplexityPattern::HighBranching { .. } => {
            assert!(rec_with_role.primary_action.contains("split") ||
                    rec_with_role.primary_action.contains("extract"));
        },
        _ => {},
    }
}
```

### Integration Tests

**Test 6: Prodigy Orchestrators Get Appropriate Advice**

```rust
#[test]
fn test_prodigy_orchestrators_get_contextual_recommendations() {
    // Use real Prodigy functions identified in questionable recommendations doc
    let test_cases = vec![
        ("execute_remaining_steps", 15, 65, 4, FunctionRole::Orchestrator),
        ("setup_environment", 22, 45, 2, FunctionRole::Orchestrator),
        ("evaluate_expression", 21, 51, 3, FunctionRole::PatternMatch),
    ];

    for (name, cyclo, cog, nest, role) in test_cases {
        let mut metrics = create_test_metrics(cyclo, cog, nest);
        metrics.name = name.to_string();

        let rec = generate_complexity_steps(cyclo, cog, &metrics, role);

        match role {
            FunctionRole::Orchestrator => {
                assert!(
                    rec.rationale.contains("orchestrat") ||
                    rec.rationale.contains("coordinat"),
                    "{} should get orchestration-aware advice, got: {}",
                    name, rec.rationale
                );
            },
            FunctionRole::PatternMatch => {
                assert!(
                    rec.rationale.contains("pattern") ||
                    rec.rationale.contains("visitor") ||
                    rec.rationale.contains("appropriate"),
                    "{} should get pattern-match-aware advice, got: {}",
                    name, rec.rationale
                );
            },
            _ => {},
        }
    }
}
```

### Performance Tests

No performance impact expected - this adds conditional logic but no I/O or heavy computation.

### Validation with Real Data

**Manual Validation**:
1. Run debtmap on Prodigy with updated recommendations
2. Verify orchestrators get orchestration-aware advice
3. Compare to "questionable recommendations" document
4. Confirm 80%+ of questionable items now get appropriate advice

## Documentation Requirements

### Code Documentation

1. **Add module-level documentation** explaining role-aware recommendations:
```rust
//! # Role-Aware Complexity Recommendations (Spec 177)
//!
//! This module generates complexity recommendations that consider both
//! the complexity pattern AND the function's semantic role.
//!
//! ## Design Rationale
//!
//! Orchestrators, pattern matchers, and I/O wrappers have different
//! appropriate complexity profiles:
//! - Orchestrators: High nesting from coordination is expected
//! - Pattern matchers: High branching from exhaustive matching is correct
//! - I/O wrappers: Minimal complexity is expected but not enforced
//!
//! ## Recommendation Strategy
//!
//! - Orchestrator + HighNesting → Extract helpers, preserve coordination
//! - PatternMatch + HighBranching → Structure is correct, add tests
//! - IOWrapper + any → Minimal recommendation
//! - PureLogic + any → Full pattern-based recommendations
```

2. **Document each (pattern, role) combination** with examples

3. **Add impact estimation formulas** with justification

### User Documentation

1. **Update output format guide** (`docs/output-format-guide.md`):
   - Explain role-aware recommendations
   - Show examples for each role type
   - Clarify why orchestrators get different advice

2. **Update FAQ** (`docs/faq.md`):
   - Q: "Why does my orchestrator have high complexity?"
   - A: "Orchestrators coordinate multiple concerns, so nesting is expected..."

3. **Add examples** to `book/src/recommendations.md`

### Architecture Updates

No ARCHITECTURE.md updates needed - this is an enhancement within existing recommendation system.

## Implementation Notes

### Gotchas

1. **Impact Estimation Must Be Realistic**
   - Orchestrators: 10-15 points max (not 30-35)
   - Pattern matchers: 0-5 points (structure is correct)
   - Track actual reductions in real refactorings

2. **Backward Compatibility for PureLogic**
   - Ensure existing recommendations don't change
   - Tests must verify PureLogic gets same advice as before

3. **Role Detection Must Be Accurate**
   - Relies on spec 117 classification
   - If classification is wrong, recommendation will be wrong
   - Consider adding "confidence" to role classification

### Best Practices

1. **Use Exhaustive Pattern Matching**
   ```rust
   match (pattern, role) {
       // Explicitly handle all combinations
       (ComplexityPattern::HighNesting { .. }, FunctionRole::Orchestrator) => ...,
       (ComplexityPattern::HighNesting { .. }, FunctionRole::PatternMatch) => ...,
       // ... all combinations
       (_, _) => default_recommendation()  // Catch-all
   }
   ```

2. **Provide Concrete Examples**
   - Don't just say "extract helpers"
   - Show what a helper extraction looks like
   - Reference common patterns (early returns, guard clauses, etc.)

3. **Test with Real Code**
   - Use Prodigy functions from questionable recommendations doc
   - Validate advice matches what experienced developers would recommend
   - Get feedback from users

## Migration and Compatibility

### Breaking Changes

**None** - this is backward compatible:
- Existing recommendations for PureLogic unchanged
- Output format unchanged
- Score calculations unchanged (already adjusted by spec 110)

### Migration Requirements

**None** - no data migration needed, all changes are code-level.

### Compatibility

Fully compatible with existing debtmap features:
- Works with entropy dampening (spec 176)
- Works with orchestration score adjustment (spec 110)
- Works with semantic classification (spec 117)
- No conflicts with other recommendation types

## Success Metrics

1. **Appropriateness**: 80%+ of orchestrators get orchestration-aware advice
2. **Clarity**: User feedback confirms recommendations make sense
3. **Impact Accuracy**: Estimated reductions match actual refactoring results
4. **Backward Compatibility**: 100% of PureLogic tests pass unchanged
5. **Test Coverage**: 100% of (pattern, role) combinations tested

## Rollback Plan

If issues discovered:
1. **Quick rollback**: Revert `generate_complexity_steps()` signature
2. **Fallback**: Make role parameter optional with default = PureLogic
3. **Investigation**: Analyze which roles get inappropriate advice
4. **Fix**: Adjust specific (pattern, role) combinations

## References

- **Spec 176**: Entropy vs effective complexity fix
- **Spec 117**: Semantic function classification
- **Spec 110**: Orchestration score adjustment
- **Spec 126**: Data flow classification
- **Questionable Recommendations Doc**: debtmap-questionable-recommendations.md
- **Related Issue**: 50% of top 10 recommendations are inappropriate
