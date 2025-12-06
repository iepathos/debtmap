---
number: 214
title: Integrate Entropy-Adjusted Complexity into Unified Scoring
category: optimization
priority: critical
status: draft
dependencies: []
created: 2025-12-05
---

# Specification 214: Integrate Entropy-Adjusted Complexity into Unified Scoring

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently calculates entropy-adjusted complexity through a sophisticated multi-phase dampening system but **does not use it** in the final unified score calculation. This represents a significant disconnect between what is measured and what drives prioritization.

### Current Entropy System (Unused)

The entropy analysis (`src/complexity/entropy_core.rs`) performs three phases:

1. **Phase 1: Effective Complexity Calculation** (`adjust_complexity`)
   - Reduces token entropy based on pattern repetition (up to 30% reduction)
   - Reduces based on branch similarity (up to 20% reduction)
   - `effective_complexity = token_entropy * pattern_factor * similarity_factor`

2. **Phase 2: Structural Dampening** (`apply_dampening`)
   - Applies nesting factor: `1.0 + (max_nesting * 0.1)`
   - Applies variable factor: `1.0 + (unique_variables * 0.01)`
   - `dampening_value = effective_complexity * nesting_factor * variable_factor`

3. **Phase 3: Complexity Adjustment** (`calculate_entropy_details`)
   - Normalizes dampening to 0.5-1.0 range
   - Calculates adjusted cyclomatic: `cyclomatic * dampening_factor`
   - Calculates adjusted cognitive: `cognitive * dampening_factor` (**calculated but unused!**)

### Current Scoring System (Ignores Entropy)

The unified scorer (`src/priority/unified_scorer.rs:216-226`) uses:
- **Raw cyclomatic and cognitive** (with only purity adjustment)
- Weighted complexity: `(cyclo/50)*100*0.3 + (cog/100)*100*0.7` (default weights)
- Passes to `calculate_complexity_factor(raw_complexity / 2.0)`

**Problem**: The entropy-dampened values in `EntropyDetails` are stored but never consulted during prioritization.

### Real-World Impact

Without entropy adjustment:
- **Pattern-matching code** with 10 similar branches gets full cyclomatic complexity of 10
- **Repetitive validation code** with identical structure gets penalized the same as genuinely complex logic
- **Boilerplate-heavy functions** score the same as algorithmically complex functions

With entropy adjustment (if integrated):
- Pattern matching (high pattern repetition) → dampening factor ~0.6 → complexity reduced to 6
- Genuinely complex logic (low repetition) → dampening factor ~0.95 → complexity stays ~9.5
- Better prioritization: focus on real complexity, not structural patterns

## Objective

Integrate entropy-adjusted complexity into the unified scoring system so that:
1. Pattern-heavy code receives lower complexity scores
2. Genuinely complex logic remains prioritized
3. The sophisticated entropy analysis actually influences prioritization
4. Users can see both raw and adjusted complexity to understand the adjustment

## Requirements

### Functional Requirements

1. **Use Entropy-Adjusted Complexity in Scoring**
   - Replace raw cyclomatic/cognitive with entropy-adjusted values in `normalize_complexity`
   - Apply entropy dampening **before** weighted complexity calculation
   - Preserve purity adjustment logic (apply before or after entropy, document choice)
   - Ensure dampening factor is available when entropy data exists

2. **Graceful Degradation**
   - If entropy data is unavailable (not calculated), fall back to raw complexity
   - Maintain backward compatibility with existing configurations
   - Don't break scoring for projects without entropy enabled

3. **Transparent Adjustments**
   - Store both raw and adjusted complexity in `UnifiedDebtItem`
   - Include dampening factor in output for transparency
   - Allow users to understand why complexity scores changed

4. **Configuration Control**
   - Add config option to enable/disable entropy-based dampening
   - Default to **enabled** for new projects
   - Allow tuning of dampening strength (pattern_weight, similarity_weight)

### Non-Functional Requirements

1. **Performance**: Entropy calculation already exists; integration should add negligible overhead
2. **Accuracy**: Entropy dampening should improve prioritization quality, not degrade it
3. **Testability**: All dampening logic must be pure functions with comprehensive tests
4. **Explainability**: Users must be able to see before/after complexity values

## Acceptance Criteria

- [ ] `normalize_complexity` uses entropy-adjusted complexity when available
- [ ] Entropy dampening is applied to both cyclomatic and cognitive complexity
- [ ] Purity adjustment and entropy dampening interact correctly (order documented)
- [ ] `UnifiedDebtItem` stores both `raw_complexity` and `adjusted_complexity` fields
- [ ] `EntropyDetails` includes final `dampening_factor` used in scoring
- [ ] Configuration option `entropy_dampening.enabled` controls the feature (default: true)
- [ ] Configuration options `entropy_dampening.pattern_weight` and `entropy_dampening.similarity_weight` allow tuning
- [ ] Graceful fallback to raw complexity when entropy data is unavailable
- [ ] Comprehensive unit tests for dampening factor calculation
- [ ] Integration tests comparing scores with/without entropy dampening
- [ ] Documentation explains how entropy dampening affects scores
- [ ] TUI and JSON output show both raw and adjusted complexity

## Technical Details

### Implementation Approach

#### 1. Modify `normalize_complexity` Function

**Current**: `src/priority/unified_scorer.rs:532`

```rust
fn normalize_complexity(cyclomatic: u32, cognitive: u32, is_orchestrator: bool) -> f64 {
    // Uses raw cyclomatic and cognitive
    let weighted = WeightedComplexity::calculate(cyclomatic, cognitive, weights, &normalization);
    weighted.weighted_score / 10.0
}
```

**Proposed**: Add entropy parameter

```rust
fn normalize_complexity(
    cyclomatic: u32,
    cognitive: u32,
    entropy_details: Option<&EntropyDetails>,
    is_orchestrator: bool
) -> f64 {
    // Apply entropy dampening if available and enabled
    let (adjusted_cyclo, adjusted_cog) = if let Some(entropy) = entropy_details {
        if config.entropy_dampening.enabled {
            let factor = entropy.dampening_factor;
            (
                (cyclomatic as f64 * factor) as u32,
                (cognitive as f64 * factor) as u32
            )
        } else {
            (cyclomatic, cognitive)
        }
    } else {
        (cyclomatic, cognitive)
    };

    let weighted = WeightedComplexity::calculate(
        adjusted_cyclo,
        adjusted_cog,
        weights,
        &normalization
    );
    weighted.weighted_score / 10.0
}
```

#### 2. Update `calculate_unified_priority` Signature

**Current**: Doesn't pass entropy details to `normalize_complexity`

**Proposed**: Thread entropy details through

```rust
pub fn calculate_unified_priority(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    config: Option<&Config>,
) -> UnifiedScore {
    // Calculate entropy details
    let entropy_details = calculate_entropy_details(func);

    // Pass to normalize_complexity
    let raw_complexity = normalize_complexity(
        purity_adjusted_cyclomatic,
        purity_adjusted_cognitive,
        entropy_details.as_ref(),
        is_orchestrator_candidate,
    );

    // ... rest of scoring
}
```

#### 3. Extend `UnifiedDebtItem` Structure

Add fields to track adjustment:

```rust
pub struct UnifiedDebtItem {
    // Existing fields...
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,

    // New fields
    pub entropy_adjusted_cyclomatic: Option<u32>,
    pub entropy_adjusted_cognitive: Option<u32>,
    pub entropy_dampening_factor: Option<f64>,

    // Existing
    pub entropy_details: Option<EntropyDetails>,
}
```

#### 4. Fix Dead Code in `calculate_entropy_details`

**Current**: `src/priority/scoring/computation.rs:26`

```rust
let _adjusted_cognitive = (func.cognitive as f64 * dampening_factor) as u32;
```

**Proposed**: Remove unused variable, use in caller

```rust
// Return both adjusted values
EntropyDetails {
    entropy_score: entropy_score.token_entropy,
    pattern_repetition: entropy_score.pattern_repetition,
    original_complexity: func.cyclomatic,
    adjusted_complexity: adjusted_cyclomatic,
    dampening_factor,
    // NEW: Also store adjusted cognitive
    adjusted_cognitive: (func.cognitive as f64 * dampening_factor) as u32,
}
```

#### 5. Configuration Schema

Add to `Config` structure:

```rust
pub struct Config {
    // Existing fields...

    pub entropy_dampening: Option<EntropyDampeningConfig>,
}

pub struct EntropyDampeningConfig {
    pub enabled: bool,              // Default: true
    pub pattern_weight: f64,        // Default: 0.3 (max 30% reduction)
    pub similarity_weight: f64,     // Default: 0.2 (max 20% reduction)
}
```

### Architecture Changes

**Modified Components**:
- `src/priority/unified_scorer.rs` - Update `normalize_complexity` and `calculate_unified_priority`
- `src/priority/scoring/computation.rs` - Fix `calculate_entropy_details` to return cognitive adjustment
- `src/priority/unified_scorer.rs` (structs) - Extend `UnifiedDebtItem` with adjustment fields
- `src/complexity/entropy_core.rs` - Expose `EntropyScore.adjusted_cognitive` field
- `src/config.rs` - Add `EntropyDampeningConfig`

**New Components**:
- None (uses existing entropy infrastructure)

### Data Structures

**Extended `EntropyDetails`**:

```rust
pub struct EntropyDetails {
    pub entropy_score: f64,
    pub pattern_repetition: f64,
    pub original_complexity: u32,
    pub adjusted_complexity: u32,
    pub dampening_factor: f64,
    pub adjusted_cognitive: u32,  // NEW
}
```

**Extended `UnifiedDebtItem`**:

```rust
pub struct UnifiedDebtItem {
    // ... existing 20+ fields

    // Complexity fields
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,

    // NEW: Entropy-adjusted values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy_adjusted_cyclomatic: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy_adjusted_cognitive: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy_dampening_factor: Option<f64>,
}
```

### Integration Points

1. **Scoring Pipeline**: `calculate_unified_priority` → `normalize_complexity` → `calculate_complexity_factor`
2. **Entropy Pipeline**: `EntropyAnalyzer::calculate` → `calculate_entropy_details` → `apply_dampening`
3. **Output**: `UnifiedDebtItem` → JSON/TUI display
4. **Configuration**: `Config` → runtime behavior

## Dependencies

**Prerequisites**: None (uses existing entropy infrastructure)

**Affected Components**:
- Unified scoring system
- Entropy analysis system
- Configuration system
- Output formatters (JSON, TUI)
- Test suites for scoring

**External Dependencies**: None

## Testing Strategy

### Unit Tests

1. **Dampening Factor Calculation**
   ```rust
   #[test]
   fn test_entropy_dampening_reduces_pattern_complexity() {
       // High pattern repetition (0.8) should reduce complexity
       let entropy_score = EntropyScore {
           pattern_repetition: 0.8,
           branch_similarity: 0.6,
           token_entropy: 0.5,
           // ...
       };
       let factor = calculate_dampening_factor(&entropy_score);
       assert!(factor < 0.7); // Should be significantly reduced
   }
   ```

2. **Graceful Fallback**
   ```rust
   #[test]
   fn test_scoring_without_entropy_uses_raw_complexity() {
       let score = calculate_unified_priority(&func, &graph, None, None);
       assert_eq!(score.complexity_factor, expected_raw_score);
   }
   ```

3. **Configuration Override**
   ```rust
   #[test]
   fn test_entropy_dampening_disabled_via_config() {
       let config = Config {
           entropy_dampening: Some(EntropyDampeningConfig {
               enabled: false,
               ..Default::default()
           }),
           ..Default::default()
       };
       // Should use raw complexity even if entropy available
   }
   ```

### Integration Tests

1. **Score Comparison**
   - Analyze same codebase with/without entropy dampening
   - Verify pattern-heavy functions score lower
   - Verify complex logic scores remain high

2. **Real-World Validation**
   - Run on debtmap's own codebase
   - Verify scoring changes are sensible
   - Check that top-priority items are still meaningful

### Property Tests

```rust
proptest! {
    #[test]
    fn entropy_dampening_never_increases_complexity(
        cyclo in 1u32..100,
        cog in 1u32..100,
        pattern_rep in 0.0f64..1.0,
        branch_sim in 0.0f64..1.0
    ) {
        let entropy = create_entropy_score(pattern_rep, branch_sim);
        let (adjusted_cyclo, adjusted_cog) = apply_dampening(cyclo, cog, &entropy);

        prop_assert!(adjusted_cyclo <= cyclo);
        prop_assert!(adjusted_cog <= cog);
    }
}
```

## Documentation Requirements

### Code Documentation

1. **Function-level docs**:
   - Document `normalize_complexity` parameter meaning
   - Explain dampening factor calculation
   - Clarify interaction between purity and entropy adjustments

2. **Inline comments**:
   - Explain why entropy dampening is applied before/after purity
   - Document fallback behavior
   - Note performance characteristics

### User Documentation

1. **README update**:
   - Explain entropy-adjusted scoring
   - Show before/after examples
   - Document configuration options

2. **Configuration guide**:
   ```toml
   [entropy_dampening]
   enabled = true
   pattern_weight = 0.3  # Max 30% reduction for patterns
   similarity_weight = 0.2  # Max 20% reduction for similarity
   ```

3. **Scoring documentation**:
   - How entropy affects complexity scores
   - What types of code benefit from dampening
   - When to disable entropy dampening

### Architecture Updates

Update `ARCHITECTURE.md`:
- Document entropy integration in scoring pipeline
- Explain dampening formula
- Show data flow diagram

## Implementation Notes

### Order of Adjustments

**Decision**: Apply entropy dampening **before** purity adjustment.

**Rationale**:
- Entropy measures code structure (patterns, repetition)
- Purity measures functional semantics
- Structure should be adjusted first, then semantic bonus applied
- Example: Pure pattern-matching function gets both dampening and purity bonus

**Formula**:
```
raw_complexity = cyclomatic + cognitive
entropy_adjusted = raw_complexity * dampening_factor
purity_adjusted = entropy_adjusted * purity_bonus
weighted = (purity_adjusted_cyclo * 0.3 + purity_adjusted_cog * 0.7)
complexity_factor = weighted / 2.0
```

### Dampening Strength Tuning

Initial defaults (from current implementation):
- `pattern_weight: 0.3` - Max 30% reduction from patterns
- `similarity_weight: 0.2` - Max 20% reduction from similarity

**Validation approach**:
1. Run on multiple codebases
2. Verify sensible scoring changes
3. Adjust weights if needed
4. Document tuning methodology

### Edge Cases

1. **No entropy data**: Fall back to raw complexity
2. **Entropy disabled**: Use raw complexity
3. **Zero dampening factor**: Set minimum of 0.5 (don't reduce to zero)
4. **Invalid entropy values**: Log warning, use raw complexity

## Migration and Compatibility

### Breaking Changes

**None**. This is backward-compatible:
- Existing scores use raw complexity (current behavior)
- New scores use entropy-adjusted complexity (if available and enabled)
- Configuration default enables feature for new projects
- Existing configs without entropy section use raw complexity

### Migration Path

1. **No action required** for existing users
2. **Optional**: Add entropy configuration to enable adjustment
3. **Recommended**: Compare scores before/after to validate changes

### Compatibility Considerations

- **JSON output**: New fields are optional, old parsers ignore them
- **TUI**: Shows both raw and adjusted complexity
- **CLI**: Existing flags continue to work
- **Configuration**: Old configs without entropy section work unchanged

## Success Metrics

1. **Correctness**:
   - All tests pass
   - No regressions in existing scores when entropy disabled
   - Sensible score changes when entropy enabled

2. **Quality**:
   - Pattern-heavy code scores 20-40% lower
   - Complex logic scores remain within 5% of original
   - User feedback confirms better prioritization

3. **Performance**:
   - Scoring overhead < 1% (entropy already calculated)
   - No memory regressions

4. **Adoption**:
   - Feature enabled by default in new projects
   - Documentation clear and actionable
   - Configuration easy to understand and tune
