---
number: 262
title: Remove Recommendation Engine
category: foundation
priority: critical
status: draft
dependencies: []
created: 2024-12-19
---

# Specification 262: Remove Recommendation Engine

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently generates template-based recommendations like "Split into N modules by responsibility" and impact predictions like "-233 complexity reduction." These recommendations are:

1. **Generic and not actionable** - They cannot understand code semantics to provide specific guidance
2. **False precision** - Impact predictions are heuristic estimates that create unwarranted confidence
3. **Fighting a losing battle** - Static analysis cannot determine *how* to refactor code, only *what* is complex

The strategic pivot is to position debtmap as an "AI sensor" - providing accurate identification, quantified severity, and structural context to AI agents (Claude, Copilot, Cursor) that have semantic understanding and can determine appropriate fixes.

**Key insight**: Static analysis can accurately tell you WHERE problems are and HOW SEVERE they are. It cannot tell you HOW to fix them. Let AI do that.

## Objective

Remove all template-based recommendation generation, heuristic impact predictions, and "Split into N modules" suggestions from debtmap. The tool should focus on:

1. **Accurate identification** - What is the problem?
2. **Quantified severity** - How bad is it?
3. **Structural context** - What's related to it?
4. **Raw signals** - Complexity breakdown, coverage gaps, coupling metrics

## Requirements

### Functional Requirements

#### Code to Remove

1. **Recommendation Generation Files** (delete entirely or gut):
   - `src/priority/scoring/recommendation.rs` - Template recommendation generators
   - `src/priority/scoring/concise_recommendation.rs` - Concise recommendation templates
   - `src/priority/scoring/recommendation_complexity.rs` - Complexity-based recommendations
   - `src/priority/scoring/recommendation_helpers.rs` - Recommendation formatting helpers
   - `src/priority/scoring/recommendation_extended.rs` - Extended recommendation re-exports
   - `src/priority/scoring/recommendation_debt_specific.rs` - Debt-specific recommendations
   - `src/priority/scoring/rust_recommendations.rs` - Rust-specific recommendations
   - `src/priority/recommendations.rs` - God object recommendations
   - `src/organization/god_object/recommendation_generator.rs` - God object split suggestions
   - `src/organization/god_object/context_recommendations.rs` - Context-aware recommendations
   - `src/organization/god_object/recommender.rs` - Module split recommender
   - `src/organization/module_recommendations.rs` - Module decomposition plans
   - `src/organization/macro_recommendations.rs` - Macro-based recommendations
   - `src/analysis/diagnostics/recommendations.rs` - RecommendationEngine strategies

2. **Impact Prediction Functions** (remove):
   - `calculate_needed_test_cases()` - Test count predictions
   - `calculate_functions_to_extract()` - Function decomposition estimates
   - `calculate_reduction_percentage()` - Code reduction estimates
   - `estimate_effort_hours()` - Effort predictions
   - All `ImpactAssessment` and `ExpectedImpact` calculations

3. **Data Structures to Simplify**:
   - `ActionableRecommendation` - Remove or simplify to optional notes field
   - `DetailedRecommendation` - Remove
   - `ContextAwareRecommendation` - Remove
   - `FunctionalAwareRecommendation` - Remove
   - `TraitAwareRecommendation` - Remove
   - `ModuleRecommendation` - Remove
   - `DecompositionPlan` - Remove
   - `ImpactAssessment` - Remove

#### Code to Keep

All retained code follows the **Stillwater "Pure Core, Imperative Shell"** pattern:

1. **Identification Logic** (Pure Core - no I/O, testable in isolation):
   - All complexity calculation (cyclomatic, cognitive, nesting)
   - Coverage gap detection
   - God object detection (method counts, responsibilities, cohesion)
   - Coupling analysis (upstream/downstream dependencies)
   - Pattern detection (state machines, coordinators, pure functions)
   - Entropy analysis (false positive reduction)

2. **Severity Quantification** (Pure Core - deterministic calculations):
   - `UnifiedScore` calculation
   - Priority tier assignment (Critical/High/Medium/Low)
   - Debt score aggregation
   - Risk scoring

3. **Structural Context** (Pure Core - data transformations):
   - Call graph data
   - Dependency relationships
   - File-level coupling metrics
   - Purity propagation

4. **Output Formatting** (Imperative Shell - I/O at boundaries):
   - JSON serialization
   - Markdown generation
   - Terminal output
   - TUI rendering

### Non-Functional Requirements

- No breaking changes to CLI arguments (recommendation fields become optional/empty)
- JSON output schema should remain backward compatible with empty recommendation fields
- Performance should improve (less computation for unused recommendations)
- Code size should decrease significantly (~3000+ LOC removal estimated)

## Acceptance Criteria

- [ ] All recommendation generation functions removed or stubbed
- [ ] `ActionableRecommendation` struct simplified or removed
- [ ] Impact prediction calculations removed
- [ ] "Split into N modules" suggestions no longer generated
- [ ] JSON output has empty/null recommendation fields (backward compatible)
- [ ] Terminal output shows metrics without action/suggestion lines
- [ ] All tests updated to reflect removed functionality
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes with no new warnings
- [ ] Code compiles and runs successfully

## Technical Details

### Implementation Approach

**Phase 1: Stub the interfaces**
1. Modify `ActionableRecommendation` to have empty/default implementations
2. Update callers to handle empty recommendations gracefully
3. Ensure JSON/markdown output handles missing recommendations

**Phase 2: Remove generation code**
1. Delete recommendation generation files one by one
2. Remove unused imports and dead code
3. Run `cargo test` after each deletion to catch breakage

**Phase 3: Clean up data structures**
1. Simplify `UnifiedDebtItem` to remove recommendation field (or make optional)
2. Remove unused types and traits
3. Update serialization to handle optional/missing fields

### Files to Modify (not delete)

1. `src/priority/scoring/mod.rs` - Remove recommendation module exports
2. `src/priority/mod.rs` - Remove recommendations module
3. `src/organization/god_object/mod.rs` - Remove recommendation modules
4. `src/organization/mod.rs` - Remove module_recommendations
5. `src/output/unified/func_item.rs` - Handle missing recommendations
6. `src/output/unified/file_item.rs` - Handle missing recommendations
7. `src/output/json.rs` - Output null/empty recommendation fields
8. `src/priority/formatter_markdown/*.rs` - Remove recommendation sections

### Backward Compatibility

JSON output should remain structurally compatible:
```json
{
  "recommendation": null,
  // OR
  "recommendation": {
    "action": null,
    "priority": null,
    "implementation_steps": []
  }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: Priority scoring, output formatters, TUI (spec 265)
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Remove recommendation-specific tests, update integration tests
- **Integration Tests**: Verify output formats still work with empty recommendations
- **Performance Tests**: Verify analysis is faster without recommendation generation
- **User Acceptance**: Verify CLI output is clean without recommendation clutter

## Documentation Requirements

- **Code Documentation**: Update module docs to reflect new purpose
- **User Documentation**: See spec 266 for documentation pivot
- **Architecture Updates**: Update ARCHITECTURE.md to remove recommendation flow

## Implementation Notes

### Stillwater Architecture Alignment

After removal, the remaining codebase should cleanly follow:
- **Pure Core**: All scoring, complexity, and analysis functions are pure (no I/O, deterministic)
- **Imperative Shell**: File reading, output writing, and TUI rendering at boundaries
- **Composition Over Complexity**: Small, focused scoring functions composed together

Recommendation generation violated this by mixing heuristic "interpretation" with data. Removing it clarifies the boundary.

### Incremental Approach

Start by making recommendations optional/empty, verify everything works, then delete the generation code. This reduces risk of cascading breakage.

### Watch for Dead Code

After removing recommendation generators, use `cargo clippy` and unused import warnings to identify newly-dead code that should also be removed.

### Test Coverage

Many recommendation-specific tests will be deleted. Ensure core identification and scoring tests remain and pass.

## Migration and Compatibility

- Existing users relying on recommendation fields in JSON will get null/empty values
- CLI output will be cleaner without action/suggestion lines
- No configuration changes required
- No database migrations

## Estimated Scope

- **Files to delete**: ~15 files
- **LOC to remove**: ~3000-4000 lines
- **Files to modify**: ~20 files
- **LOC to modify**: ~500-800 lines
- **Risk level**: Medium (many files touched but changes are deletions)
