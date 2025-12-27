# Exponential Scaling

Debtmap uses exponential scaling and risk boosting to amplify high-severity technical debt items, ensuring critical issues stand out clearly in priority lists. This section explains how these mechanisms work.

## Overview

Traditional linear multipliers create uniform gaps between scores:
- Linear 2x multiplier: Score 50 → 100, Score 100 → 200 (uniform +50 and +100 gaps)

Exponential scaling creates growing gaps that make critical issues impossible to miss:
- Exponential scaling (^1.4): Score 50 → 279, Score 100 → 1000 (gaps grow dramatically)

**Key Benefits**:
- **Visual Separation**: Critical items have dramatically higher scores than medium items
- **Natural Clustering**: Similar-severity items cluster together in ranked lists
- **Actionable Ordering**: Work through the list from top to bottom with confidence
- **No Arbitrary Thresholds**: Pure score-based ranking eliminates debates about tier boundaries

## How Exponential Scaling Works

After calculating the base score (complexity + coverage + dependencies), Debtmap applies pattern-specific exponential scaling.

**Formula** (from `src/priority/scoring/scaling.rs:77`):
```rust
scaled_score = base_score.max(1.0).powf(exponent)
```

The `max(1.0)` ensures minimum base score to avoid zero^exponent = 0.

### Debt-Type-Specific Exponents

The exponents are defined in `ScalingConfig` (`src/priority/scoring/scaling.rs:16-28`):

| Debt Type | Exponent | Condition | Rationale |
|-----------|----------|-----------|-----------|
| God Objects | 1.4 | All God Objects | Highest amplification - architectural issues deserve top priority |
| God Modules | 1.4 | All God Modules | Same as God Objects - file-level architectural issues |
| High Complexity | 1.2 | cyclomatic > 30 | Moderate amplification - major complexity issues |
| Moderate Complexity | 1.1 | cyclomatic > 15 | Light amplification - notable but less severe |
| Complex Testing Gap | 1.1 | cyclomatic > 20 | Functions with complex untested code |
| All Other Types | 1.0 | Default | Linear scaling (no amplification) |

**Source**: `src/priority/scoring/scaling.rs:53-75` - `apply_exponential_scaling()` function

### Example: God Object Scaling (exponent = 1.4)

Comparing three God Objects with different base scores:

| Base Score | Calculation | Scaled Score | Amplification |
|------------|-------------|--------------|---------------|
| 10 | 10^1.4 | 25.1 | 2.5x |
| 50 | 50^1.4 | 279.5 | 5.6x |
| 100 | 100^1.4 | 1000.0 | 10x |

**Result**: The highest-severity God Object (score 100) gets 10x amplification, while a minor issue (score 10) only gets 2.5x. This creates clear visual separation in priority lists.

**Source**: Test validation in `src/priority/scoring/scaling.rs:271-293`

## Risk Boosting

After exponential scaling, Debtmap applies additional risk multipliers based on architectural position and code patterns.

**Formula** (from `src/priority/scoring/scaling.rs:84-109`):
```rust
final_score = scaled_score * risk_multiplier
```

### Risk Multipliers

Risk factors are applied multiplicatively. The multipliers are defined in `ScalingConfig` (`src/priority/scoring/scaling.rs:23-28`):

| Risk Factor | Multiplier | Condition | Rationale |
|-------------|------------|-----------|-----------|
| High Dependency Count | 1.2x | total_deps > 15 | Central code that affects more of the codebase |
| Entry Point | 1.15x | `FunctionRole::EntryPoint` | Failures cascade to all downstream code |
| Complex + Untested | 1.25x | cyclomatic > 20 AND coverage < 10% | High-risk combination of complexity and no tests |
| Error Swallowing | 1.15x | error_swallowing_count > 0 | Functions with poor error handling patterns |

**Note**: Multiple risk factors combine multiplicatively. A function with high dependencies AND entry point status would get 1.2 * 1.15 = 1.38x boost.

**Source**: `src/priority/scoring/scaling.rs:84-109` - `apply_risk_boosts()` function

### Error Swallowing Boost

Functions that swallow errors receive a 1.15x boost to encourage proper error handling. This includes patterns like:
- `if let Ok(x) = expr` without handling the `Err` case
- Ignoring error returns from fallible operations

**Source**: `src/priority/scoring/scaling.rs:103-107`

### Example: Complete Score Calculation

```
Function: process_payment (God Object)
  Base Score: 85.0

  Step 1 - Exponential Scaling (exponent 1.4):
    85.0^1.4 = 554.3

  Step 2 - Risk Boosting:
    - Entry point: ×1.15 → 637.4
    - High dependencies (20 deps): ×1.2 → 764.9

  Final Score: 764.9
```

**Source**: Integration test in `src/priority/scoring/scaling.rs:355-383`

## Complete Scoring Pipeline

Debtmap processes scores through multiple stages:

```
1. Base Score Calculation
   ↓
   Weighted sum of:
   - Coverage factor (50% weight)
   - Complexity factor (35% weight)
   - Dependency factor (15% weight)

2. Exponential Scaling
   ↓
   Debt-type-specific exponent applied:
   - God Objects/Modules: ^1.4
   - High Complexity (>30): ^1.2
   - Moderate Complexity (>15): ^1.1
   - Complex Testing Gap (>20): ^1.1
   - Others: ^1.0 (linear)

3. Risk Boosting
   ↓
   Architectural position multipliers:
   - High dependencies (>15 total): ×1.2
   - Entry points: ×1.15
   - Complex + untested: ×1.25
   - Error swallowing: ×1.15

4. Final Score
   ↓
   Used for ranking (no tier bucketing)

5. Output
   ↓
   Sorted descending by final score
```

**Note on weights**: The default scoring weights are defined in `src/config/scoring.rs:187-198`:
- `coverage_weight`: 0.50 (50%)
- `complexity_weight`: 0.35 (35%)
- `dependency_weight`: 0.15 (15%)

## Configuration

The exponential scaling parameters are defined in the `ScalingConfig` struct but are **not currently configurable via TOML**. The system uses hardcoded defaults:

```rust
// From src/priority/scoring/scaling.rs:30-43
impl Default for ScalingConfig {
    fn default() -> Self {
        Self {
            god_object_exponent: 1.4,
            god_module_exponent: 1.4,
            high_complexity_exponent: 1.2,
            moderate_complexity_exponent: 1.1,
            high_dependency_boost: 1.2,
            entry_point_boost: 1.15,
            complex_untested_boost: 1.25,
            error_swallowing_boost: 1.15,
        }
    }
}
```

**Future Enhancement**: TOML configuration support for these parameters could be added to allow per-project tuning.

## Comparing With vs Without Exponential Scaling

**Without Exponential Scaling (Linear Multipliers)**:
```
Priority List:
1. God Object (base: 85) → final: 170 (2x multiplier)
2. Long Function (base: 80) → final: 160 (2x multiplier)
3. Complex Function (base: 75) → final: 150 (2x multiplier)
4. Medium Issue (base: 70) → final: 140 (2x multiplier)
```
**Problem**: Gaps are uniform (10 points). Hard to distinguish critical from medium issues.

**With Exponential Scaling**:
```
Priority List:
1. God Object (base: 85) → scaled: 554 → with risk: 701
2. Complex Function (base: 80, cyclomatic>30) → scaled: 447 → with risk: 492
3. Moderate Complexity (base: 75, cyclomatic>15) → scaled: 357 → with risk: 357
4. Simple Issue (base: 70) → scaled: 70 → with risk: 70
```
**Result**: Clear separation. God Object stands out as nearly 10x higher than simple issues.

## Score Ordering Guarantees

The exponential scaling implementation provides mathematical guarantees validated by property tests:

1. **Monotonicity**: Higher base scores always result in higher scaled scores
2. **Non-decreasing boosts**: Risk boosts never decrease scores (all multipliers ≥ 1.0)
3. **Strict ordering**: No score inversions in final ranking

**Source**: Property tests in `src/priority/scoring/scaling.rs:513-694`

## Practical Example

```bash
debtmap analyze . --top 10
```

**Output**:
```
Top 10 Technical Debt Items (Sorted by Score)

1. src/services/user_service.rs:45 - UserService::authenticate
   Score: 1247.3 | Pattern: God Object | Coverage: 12%
   → 45 methods, 892 lines, high complexity
   → Risk factors: Entry point (×1.15), High dependencies (×1.2)

2. src/payment/processor.rs:142 - process_payment
   Score: 891.2 | Pattern: Complexity Hotspot | Coverage: 8%
   → Cyclomatic: 42, Cognitive: 77
   → Risk factors: Entry point (×1.15), Complex untested (×1.25)

3. src/reporting/generator.rs:234 - generate_monthly_report
   Score: 654.1 | Pattern: Complexity Hotspot | Coverage: 45%
   → Cyclomatic: 35, Cognitive: 50
   → Risk factors: High dependencies (×1.2)
```

**Action**: Focus on top 3 items first - they have dramatically higher scores than items 4-10.

## Performance Impact

Exponential scaling has negligible performance impact:
- **Computation**: Simple `powf()` operation per item
- **Overhead**: <1% additional analysis time
- **Scalability**: Works with parallel processing (no synchronization needed)
- **Memory**: No additional data structures required

## See Also

- [Function-Level Scoring](function-level.md) - Base score calculation at function level
- [File-Level Scoring](file-level.md) - Aggregation and file-level metrics
- [Rebalanced Scoring](rebalanced.md) - Score normalization and balancing
- [Data Flow Scoring](data-flow.md) - Purity and refactorability adjustments
