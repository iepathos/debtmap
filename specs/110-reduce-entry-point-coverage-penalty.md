---
number: 110
title: Reduce Entry Point Coverage Penalty in Scoring
category: compatibility
priority: medium
status: draft
dependencies: [109]
created: 2025-10-06
---

# Specification 110: Reduce Entry Point Coverage Penalty in Scoring

**Category**: compatibility
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 109 (weight rebalancing should be applied first)

## Context

Entry points (CLI handlers, API endpoints, main functions) naturally have lower measured coverage because:
1. They're often tested via **integration tests** rather than unit tests
2. Integration test coverage isn't captured by line-based coverage tools (lcov)
3. They coordinate other functions rather than containing complex logic
4. E2E tests exercise them but don't show up in coverage metrics

**Current Behavior:**
```rust
// src/priority/unified_scorer.rs:209
FunctionRole::EntryPoint => 1.5,  // Increases priority by 50%

// src/priority/unified_scorer.rs:248
let clamped_role_multiplier = role_multiplier.clamp(0.8, 1.2);  // Actually clamped to 1.2
```

**Inconsistency**: Code sets `EntryPoint` to 1.5x multiplier, but clamp limits it to 1.2x.

**Problem with Coverage-Driven Scoring:**
- Entry point with 0% unit coverage but 100% integration coverage → flagged as critical
- Entry point with cc=17 (legitimate orchestration complexity) → ranked #5 in debtmap analysis
- Role multiplier (1.2x-1.5x) **increases** penalty instead of reducing it

**Example from debtmap self-analysis:**
```
#5 handle_analyze() - Entry point, cc=17, 0% coverage → Score 25.6
   ↳ Main factors: 🔴 UNTESTED (0% coverage, weight: 50%), Moderate complexity
   ↳ Role Adjustment: ×1.50
```

**Expected**: Entry points should have **reduced** coverage penalties since integration tests cover them
**Actual**: Entry points get **increased** scores due to role multiplier acting as amplifier

## Objective

Reduce the coverage penalty for entry points by applying a coverage weight multiplier that recognizes integration test coverage. Entry points should not be penalized as heavily for missing unit test coverage when they're validated through integration tests.

## Requirements

### Functional Requirements

1. **Coverage Weight Reduction for Entry Points**
   - Apply 0.6x multiplier to coverage factor for entry points
   - Maintain normal complexity and dependency scoring
   - Only affect coverage component of scoring

2. **Role-Specific Coverage Adjustment**
   ```rust
   // Proposed implementation
   fn calculate_coverage_factor_for_role(
       coverage_factor: f64,
       role: FunctionRole
   ) -> f64 {
       match role {
           FunctionRole::EntryPoint => coverage_factor * 0.6,  // Reduce coverage penalty
           FunctionRole::Orchestrator => coverage_factor * 0.8, // Slight reduction
           _ => coverage_factor,  // No adjustment
       }
   }
   ```

3. **Remove Amplifying Role Multiplier**
   - Remove the clamped role multiplier (currently 0.8-1.2) from base score calculation
   - OR: Clarify that role multiplier only applies to non-coverage factors
   - Ensure entry points don't get double-penalized

4. **Entry Point Detection Validation**
   - Verify entry point detection is accurate
   - Don't reduce coverage penalty for misclassified functions
   - Consider confidence scoring for role detection

### Non-Functional Requirements

1. **Configurability**
   - Coverage multipliers should be configurable per role
   - Default values based on architectural patterns
   - Allow project-specific overrides

2. **Transparency**
   - Display coverage adjustment in verbose output
   - Show "0% unit coverage (integration tested)" for entry points
   - Explain why entry point has different scoring

3. **Validation**
   - Entry points should still be flagged if truly untested
   - Complex entry points should still appear in recommendations
   - God object entry points shouldn't be ignored

## Acceptance Criteria

- [ ] Entry points receive 0.6x coverage weight multiplier
- [ ] Orchestrators receive 0.8x coverage weight multiplier
- [ ] Other roles receive 1.0x (no adjustment)
- [ ] Role multiplier clamping removed or clarified
- [ ] Entry point `handle_analyze()` no longer in top 10 if it has integration test coverage
- [ ] Display shows "Entry point - lower unit coverage expected" indicator
- [ ] Configuration allows per-role coverage weight customization
- [ ] Tests validate entry point scoring adjustment
- [ ] Documentation explains architectural rationale

## Technical Details

### Implementation Approach

**Option 1: Coverage Weight Multiplier (Recommended)**
```rust
// File: src/priority/unified_scorer.rs

// Add after line 241:
let role_coverage_multiplier = match role {
    FunctionRole::EntryPoint => 0.6,     // Integration tested, lower unit coverage expected
    FunctionRole::Orchestrator => 0.8,   // Often tested via callers
    _ => 1.0,                            // Normal coverage expectations
};

let adjusted_coverage_factor = coverage_factor * role_coverage_multiplier;

// Then use adjusted_coverage_factor in scoring:
let coverage_multiplier = if func.is_test {
    0.1
} else if adjusted_coverage_factor < 0.3 {
    1.0 + (adjusted_coverage_factor * 3.0)
} else {
    1.0 + (adjusted_coverage_factor * 1.5)
};
```

**Option 2: Separate Entry Point Scoring Path**
```rust
let base_score = if func.role == FunctionRole::EntryPoint {
    // Entry points: reduce coverage weight, increase complexity weight
    let entry_weights = (0.25, 0.50, 0.25); // (coverage, complexity, deps)
    calculate_base_score_custom(
        coverage_factor,
        complexity_factor,
        dependency_factor,
        entry_weights
    )
} else {
    // Normal scoring (40/40/20 from spec 109)
    calculate_base_score(coverage_factor, complexity_factor, dependency_factor)
};
```

**Option 3: Integration Coverage Bonus**
```rust
// If entry point has high call graph connectivity (indirect coverage indicator)
let integration_coverage_estimate = estimate_integration_coverage(func, call_graph);

let effective_coverage = if role == FunctionRole::EntryPoint {
    coverage_pct.max(integration_coverage_estimate)
} else {
    coverage_pct
};
```

### Recommended Approach

**Option 1** (Coverage Weight Multiplier) is recommended because:
- Simple, clear, and maintainable
- Doesn't require call graph analysis
- Configurable per role
- Transparent in scoring breakdown
- Doesn't create special-case scoring paths

### Architecture Changes

Minor changes to `unified_scorer.rs`:
- Add role-based coverage weight multiplier
- Update scoring calculation to use adjusted coverage
- Add configuration structure for coverage multipliers

### Data Structures

```rust
// Add to config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleCoverageWeights {
    pub entry_point: f64,      // Default: 0.6
    pub orchestrator: f64,     // Default: 0.8
    pub pure_logic: f64,       // Default: 1.0
    pub io_wrapper: f64,       // Default: 1.0
    pub pattern_match: f64,    // Default: 1.0
    pub unknown: f64,          // Default: 1.0
}

impl Default for RoleCoverageWeights {
    fn default() -> Self {
        Self {
            entry_point: 0.6,
            orchestrator: 0.8,
            pure_logic: 1.0,
            io_wrapper: 1.0,
            pattern_match: 1.0,
            unknown: 1.0,
        }
    }
}
```

### APIs and Interfaces

No breaking API changes. Internal scoring logic modifications only.

## Dependencies

- **Prerequisites**:
  - Spec 109 (weight rebalancing) - should be applied first to establish baseline
- **Affected Components**:
  - `src/priority/unified_scorer.rs` (coverage weight adjustment)
  - `src/config.rs` (add role coverage weights configuration)
  - `src/priority/formatter_verbosity.rs` (display adjustment indicator)
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Test entry point gets 0.6x coverage weight
  - Test orchestrator gets 0.8x coverage weight
  - Test other roles get 1.0x (no change)
  - Verify configuration loading

- **Integration Tests**:
  - Run debtmap on itself, verify `handle_analyze()` not in top 10
  - Test CLI entry points aren't over-penalized
  - Validate complex entry points still flagged appropriately

- **Regression Tests**:
  - Ensure non-entry-point scoring unchanged
  - Verify god objects still prioritized
  - Check pure logic functions maintain normal scoring

## Test Cases

```rust
#[test]
fn test_entry_point_coverage_adjustment() {
    let entry_point_func = create_test_function(
        FunctionRole::EntryPoint,
        17,   // cyclomatic complexity
        0.0,  // 0% coverage
        10    // callers
    );

    let pure_logic_func = create_test_function(
        FunctionRole::PureLogic,
        17,   // same complexity
        0.0,  // same coverage
        10    // same callers
    );

    let entry_score = calculate_unified_score(&entry_point_func, true);
    let logic_score = calculate_unified_score(&pure_logic_func, true);

    // Entry point should score LOWER due to coverage adjustment
    assert!(entry_score.final_score < logic_score.final_score);

    // Coverage factor should be adjusted
    assert_eq!(entry_score.coverage_weight_multiplier, 0.6);
    assert_eq!(logic_score.coverage_weight_multiplier, 1.0);
}

#[test]
fn test_entry_point_with_coverage_not_overly_penalized() {
    let entry_point = create_test_function(
        FunctionRole::EntryPoint,
        12,   // moderate complexity
        0.5,  // 50% coverage (might be integration tested)
        5     // moderate callers
    );

    let score = calculate_unified_score(&entry_point, true);

    // Should not rank in critical tier
    assert!(score.final_score < 20.0);
}

#[test]
fn test_complex_entry_point_still_flagged() {
    let complex_entry = create_test_function(
        FunctionRole::EntryPoint,
        25,   // very high complexity
        0.1,  // minimal coverage
        15    // many callers
    );

    let score = calculate_unified_score(&complex_entry, true);

    // Should still be flagged due to complexity, even with coverage adjustment
    assert!(score.final_score > 15.0);
}
```

## Documentation Requirements

- **Code Documentation**:
  - Document role coverage weight multipliers
  - Explain why entry points have reduced coverage penalty
  - Add examples of integration-tested entry points

- **User Documentation**:
  - Update README with entry point scoring behavior
  - Add FAQ entry: "Why don't entry points need 100% unit coverage?"
  - Document configuration options

- **Architecture Updates**:
  - Add design decision: integration vs unit test coverage
  - Document role-based scoring adjustments
  - Explain architectural layer testing strategies

## Implementation Notes

### Rationale for 0.6x Multiplier

**Why 0.6 for entry points?**
- 0% unit coverage → treated as ~40% effective coverage
- Assumes integration tests provide some indirect validation
- Still penalizes truly untested entry points
- Balances with complexity and dependency factors

**Why 0.8 for orchestrators?**
- Orchestration functions often tested via higher-level tests
- Less critical than pure business logic
- More critical than entry points (have business logic)

**Why 1.0 for pure logic?**
- Pure logic should have unit tests
- No excuse for missing coverage
- High testability - easy to test in isolation

### Entry Point Characteristics

Entry points typically:
- **Coordinate** other functions (orchestration)
- **Validate** inputs (boundary protection)
- **Transform** external data to internal formats
- **Route** to appropriate handlers

These are tested effectively via:
- Integration tests (API calls, CLI invocations)
- E2E tests (full user workflows)
- Smoke tests (basic functionality)

Unit tests for entry points often:
- Mock too much (not testing real integration)
- Test framework boilerplate (low value)
- Duplicate integration test coverage

### False Positive Mitigation

**Risk**: Entry points that are truly untested get hidden

**Mitigations**:
1. **Complexity threshold**: High-complexity entry points still flagged
2. **Dependency tracking**: Entry points with no callers flagged as dead code
3. **Integration coverage**: Future enhancement to detect E2E coverage
4. **Confidence scoring**: Flag low-confidence entry point detection

### Configuration Example

```toml
# .debtmap.toml

[role_coverage_weights]
entry_point = 0.6     # Integration tested, lower unit coverage expected
orchestrator = 0.8    # Often tested via callers
pure_logic = 1.0      # Should have unit tests
io_wrapper = 1.0      # Should have unit tests
pattern_match = 1.0   # Should have unit tests
unknown = 1.0         # Default behavior
```

### Display Example

```
#12 SCORE: 18.2 [🟡 PARTIAL COVERAGE] [HIGH]
   ↳ Main factors: 🟡 PARTIAL COVERAGE (47.1%, weight: 40% × 0.6 = 24%), Moderate complexity
├─ SCORE CALCULATION:
│  ├─ Weighted Sum Model:
│  ├─ Coverage Score: 5.3 × 24% = 1.27 (gap: 52.9%, coverage: 47.1%, role adj: 0.6x)
│  ├─ Complexity Score: 8.5 × 40% = 3.40 (entropy-adjusted from 17)
│  ├─ Dependency Score: 2.0 × 20% = 0.40 (1 callers)
│  └─ Final Score: 18.2
├─ LOCATION: ./src/commands/analyze.rs:62 handle_analyze()
├─ WHY: Entry point - integration tested, lower unit coverage expected
```

## Migration and Compatibility

**Breaking Changes:**
- None (internal scoring logic only)

**User Impact:**
- **Positive**: Entry points stop dominating recommendations
- **Positive**: More intuitive prioritization for layered architectures
- **Neutral**: Scores change (expected behavior)

**Rollout Strategy:**
- Include with spec 109 weight rebalancing
- Combined release provides holistic scoring improvement
- Release as minor version (e.g., 0.2.6)

## Future Enhancements

1. **Integration Coverage Detection**
   - Parse integration test files
   - Track which entry points are tested
   - Show "integration tested" badge

2. **Confidence-Based Adjustment**
   - High confidence entry point detection → 0.6x adjustment
   - Low confidence → 1.0x (no adjustment)
   - Reduce false negatives

3. **Layered Architecture Support**
   - Different coverage expectations per layer
   - Controller layer: 0.6x
   - Service layer: 1.0x
   - Domain layer: 1.2x (higher expectations)

4. **Call Graph Integration**
   - Estimate transitive coverage via call graph
   - Entry point with 0% direct but 80% transitive → lower penalty
   - More sophisticated integration coverage estimation

5. **Test Type Detection**
   - Distinguish unit vs integration vs E2E tests
   - Entry point with E2E coverage → no penalty
   - Entry point with no E2E coverage → full penalty
