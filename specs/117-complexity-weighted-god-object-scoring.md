---
number: 117
title: Complexity-Weighted God Object Scoring
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-18
---

# Specification 117: Complexity-Weighted God Object Scoring

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap v0.2.8's god object detector produces false positives when analyzing well-refactored code that follows functional programming principles. The current implementation counts raw method counts without considering individual function complexity, leading to incorrect prioritization.

**Real-World False Positive**:
- **File**: `src/cache/shared_cache.rs` (2196 lines, 107 functions)
- **Current Score**: 70.0 (CRITICAL - top recommendation)
- **Reality**: Well-designed cache with 98 low-complexity helper functions
- **Actual Complexity**: Most functions have cyclomatic complexity 1-3
- **Function Breakdown**:
  - 22 test functions (20% of count)
  - 65 pure helper functions (complexity ≤3)
  - 20 public API functions (complexity 1-5)
  - Average function size: 5-15 lines

**Current Behavior**:
```
#1 SCORE: 70.0 [CRITICAL - FILE - GOD OBJECT]
├─ ./src/cache/shared_cache.rs (2196 lines, 107 functions)
├─ WHY: This class violates single responsibility principle with 98 methods
```

**What Should Happen**:
Files with many simple functions (complexity 1-3) should score LOWER than files with fewer complex functions (complexity 17-33):

```
✅ GOOD: cache.rs - 100 functions @ complexity 1-3 each = weighted count ~30
❌ BAD: analysis.rs - 10 functions @ complexity 17+ each = weighted count ~170
```

**Why This is Critical**:
- Functional programming encourages breaking down complex functions into many simple pure functions
- Current detector **penalizes good refactoring** (100 simple functions scores worse than 10 complex ones)
- Users ignore real god objects because false positives appear first
- Items #4, #5, #6, #9 (complexity 17-33) should rank higher than #1 (complexity 1-3)

## Objective

Replace raw method counting with complexity-weighted scoring that accurately identifies true god objects while rewarding functional decomposition into simple, focused functions.

## Requirements

### Functional Requirements

1. **Complexity Weighting Formula**
   - Each function contributes to god object score proportional to its complexity
   - Weight = `max(1, cyclomatic_complexity / 3)^1.5`
   - Examples:
     - Complexity 1 → weight 0.33 (⅓ of a "standard" function)
     - Complexity 3 → weight 1.0 (baseline)
     - Complexity 9 → weight 3.0 (3× impact)
     - Complexity 17 → weight 8.2 (8× impact)
     - Complexity 33 → weight 22.9 (23× impact)

2. **Weighted Method Count Calculation**
   ```rust
   weighted_count = functions.iter()
       .map(|f| calculate_complexity_weight(f.cyclomatic_complexity))
       .sum()

   fn calculate_complexity_weight(complexity: u32) -> f64 {
       let normalized = (complexity as f64 / 3.0).max(1.0);
       normalized.powf(1.5)
   }
   ```

3. **Score Comparison Examples**
   - **shared_cache.rs** (current false positive):
     - Raw count: 107 functions
     - Weighted count: ~35 (most functions complexity 1-3)
     - New score: ~12 (not a god object)

   - **unified_analysis.rs** (actual problem):
     - Raw count: 15 functions
     - Weighted count: ~85 (many functions complexity 17-33)
     - New score: 65 (legitimate god object)

4. **Integration with Existing Metrics**
   - Maintain current scoring dimensions: methods, fields, responsibilities, LOC
   - Replace `method_count` with `weighted_method_count` in all calculations
   - Preserve backward compatibility for non-complexity data

### Non-Functional Requirements

1. **Performance**: Complexity calculation adds <5% overhead to analysis
2. **Accuracy**: Reduce false positive rate from ~40% to <10%
3. **Backward Compatibility**: Existing god object analysis continues to work
4. **Extensibility**: Support future weighting factors (purity, test coverage, etc.)

## Acceptance Criteria

- [ ] `calculate_god_object_score()` accepts `weighted_method_count` parameter
- [ ] `calculate_complexity_weight()` function implements exponential scaling formula
- [ ] God object detector aggregates complexity from all functions in file
- [ ] Files with 100 functions @ complexity 1-3 score lower than files with 10 functions @ complexity 17
- [ ] `shared_cache.rs` no longer appears as #1 god object (scores <20)
- [ ] `unified_analysis.rs` and similar high-complexity files rank higher (scores >50)
- [ ] Existing test suite passes with weighted scoring
- [ ] New tests verify complexity weighting behavior
- [ ] Documentation updated with complexity weighting explanation
- [ ] False positive rate reduced to <10% on benchmark codebase

## Technical Details

### Implementation Approach

1. **Phase 1: Add Complexity Aggregation**
   ```rust
   // In god_object_detector.rs
   pub struct FunctionComplexityInfo {
       name: String,
       cyclomatic_complexity: u32,
       cognitive_complexity: u32,
       is_test: bool,
   }

   pub struct ComplexityWeightedAnalysis {
       raw_method_count: usize,
       weighted_method_count: f64,
       avg_complexity: f64,
       max_complexity: u32,
       high_complexity_count: usize, // complexity > 10
   }
   ```

2. **Phase 2: Update Scoring Formula**
   ```rust
   // In god_object_analysis.rs
   pub fn calculate_god_object_score_weighted(
       weighted_method_count: f64,  // Changed from usize
       field_count: usize,
       responsibility_count: usize,
       lines_of_code: usize,
       avg_complexity: f64,         // New parameter
       thresholds: &GodObjectThresholds,
   ) -> f64 {
       // Use weighted count instead of raw count
       let method_factor = (weighted_method_count / thresholds.max_methods as f64).min(3.0);

       // Add complexity bonus/penalty
       let complexity_factor = if avg_complexity < 3.0 {
           0.7  // Reward simple functions
       } else if avg_complexity > 10.0 {
           1.5  // Penalize complex functions
       } else {
           1.0
       };

       let base_score = method_factor * field_factor * responsibility_factor * size_factor;
       base_score * complexity_factor * 50.0
   }
   ```

3. **Phase 3: Extract Complexity from AST**
   ```rust
   impl TypeVisitor {
       fn extract_function_complexity(&mut self, item_fn: &ItemFn) -> u32 {
           // Use existing cyclomatic complexity analyzer
           // Fall back to heuristic if analyzer not available
           let complexity = if let Some(analyzer) = &self.complexity_analyzer {
               analyzer.calculate_cyclomatic_complexity(item_fn)
           } else {
               estimate_complexity_from_ast(item_fn)
           };
           complexity
       }
   }
   ```

### Architecture Changes

**New Modules**:
- `src/organization/complexity_weighting.rs` - Complexity weight calculation
- `src/organization/function_complexity_extractor.rs` - Extract complexity from AST

**Modified Modules**:
- `src/organization/god_object_detector.rs` - Aggregate weighted complexity
- `src/organization/god_object_analysis.rs` - Update scoring formula
- `src/priority/scoring/rust_recommendations.rs` - Use weighted scores

**Data Flow**:
```
AST Parse → Extract Functions → Calculate Complexity → Apply Weights → Aggregate Score
     ↓              ↓                     ↓                  ↓              ↓
  ItemFn    FunctionInfo         cyclomatic=17        weight=8.2      weighted_sum
```

### Data Structures

```rust
#[derive(Debug, Clone)]
pub struct ComplexityWeight {
    pub raw_value: u32,
    pub weighted_value: f64,
    pub weight_multiplier: f64,
}

impl ComplexityWeight {
    pub fn calculate(cyclomatic_complexity: u32) -> Self {
        let normalized = (cyclomatic_complexity as f64 / 3.0).max(1.0);
        let weighted_value = normalized.powf(1.5);

        Self {
            raw_value: cyclomatic_complexity,
            weighted_value,
            weight_multiplier: weighted_value / cyclomatic_complexity as f64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WeightedGodObjectMetrics {
    pub raw_method_count: usize,
    pub weighted_method_count: f64,
    pub complexity_distribution: Vec<(u32, usize)>, // (complexity_level, count)
    pub avg_complexity: f64,
    pub median_complexity: u32,
    pub high_complexity_functions: Vec<String>, // Functions with complexity > 15
}
```

### APIs and Interfaces

**Public API**:
```rust
impl GodObjectDetector {
    /// Analyze file with complexity weighting
    pub fn analyze_comprehensive_weighted(
        &self,
        path: &Path,
        ast: &syn::File,
    ) -> WeightedGodObjectAnalysis;

    /// Calculate complexity weight for a single function
    pub fn calculate_complexity_weight(complexity: u32) -> f64;

    /// Get complexity distribution for debugging
    pub fn get_complexity_distribution(&self) -> Vec<(String, u32)>;
}
```

**Internal APIs**:
```rust
fn aggregate_weighted_complexity(functions: &[FunctionComplexityInfo]) -> f64;
fn calculate_complexity_penalty(avg_complexity: f64) -> f64;
fn estimate_complexity_from_ast(item_fn: &ItemFn) -> u32;
```

## Dependencies

- **Prerequisites**: None (standalone optimization)
- **Affected Components**:
  - `src/organization/god_object_detector.rs`
  - `src/organization/god_object_analysis.rs`
  - `src/priority/scoring/rust_recommendations.rs`
- **External Dependencies**: None (uses existing complexity analyzers)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_complexity_weight_calculation() {
    assert_eq!(calculate_complexity_weight(1), 0.33);
    assert_eq!(calculate_complexity_weight(3), 1.0);
    assert_eq!(calculate_complexity_weight(9), 3.0);
    assert_eq!(calculate_complexity_weight(17), 8.2);
}

#[test]
fn test_weighted_vs_raw_count_simple_functions() {
    // 100 functions @ complexity 1 should score better than 10 @ complexity 17
    let simple_weighted = aggregate_weighted(vec![1; 100]);
    let complex_weighted = aggregate_weighted(vec![17; 10]);
    assert!(simple_weighted < complex_weighted);
}

#[test]
fn test_shared_cache_no_longer_false_positive() {
    let analysis = analyze_file("src/cache/shared_cache.rs");
    assert!(analysis.god_object_score < 20.0);
}
```

### Integration Tests

```rust
#[test]
fn test_end_to_end_weighted_scoring() {
    let results = run_debtmap_on_codebase("tests/fixtures/debtmap-self");

    // Verify shared_cache.rs is NOT in top 10
    let top_10 = results.take(10);
    assert!(!top_10.iter().any(|r| r.file.contains("shared_cache.rs")));

    // Verify high-complexity files ARE in top 10
    assert!(top_10.iter().any(|r| r.file.contains("unified_analysis.rs")));
}
```

### Performance Tests

```rust
#[bench]
fn bench_weighted_complexity_calculation(b: &mut Bencher) {
    let functions = load_test_functions(1000);
    b.iter(|| aggregate_weighted_complexity(&functions));
}
```

### User Acceptance

- [ ] Run debtmap on self and verify `shared_cache.rs` not in top 5
- [ ] Verify high-complexity files rank appropriately
- [ ] False positive rate <10% on 5 test codebases

## Documentation Requirements

### Code Documentation

```rust
/// Calculate complexity-weighted god object score.
///
/// Unlike raw method counting, this function weights each method by its
/// cyclomatic complexity, ensuring that 100 simple functions (complexity 1-3)
/// score better than 10 complex functions (complexity 17+).
///
/// # Weighting Formula
///
/// Each function contributes: `max(1, complexity / 3)^1.5`
///
/// Examples:
/// - Complexity 1 → weight 0.33 (simple helper)
/// - Complexity 3 → weight 1.0 (baseline)
/// - Complexity 17 → weight 8.2 (needs refactoring)
/// - Complexity 33 → weight 22.9 (critical problem)
///
/// # Arguments
///
/// * `weighted_method_count` - Sum of complexity weights for all functions
/// * `avg_complexity` - Average cyclomatic complexity across functions
///
/// # Returns
///
/// God object score (0-100+). Scores >70 indicate definite god objects.
pub fn calculate_god_object_score_weighted(
    weighted_method_count: f64,
    avg_complexity: f64,
    // ... other params
) -> f64
```

### User Documentation

Update `ARCHITECTURE.md`:

```markdown
## God Object Detection with Complexity Weighting

Debtmap uses complexity-weighted scoring to avoid false positives when
analyzing well-refactored functional code.

### How It Works

Instead of counting methods directly, each function contributes to the
score based on its cyclomatic complexity:

- **Simple functions** (complexity 1-3): Low weight, encouraged
- **Medium functions** (complexity 4-9): Normal weight
- **Complex functions** (complexity 10+): High weight, discouraged

### Examples

**Good Code** (not flagged as god object):
```rust
// 100 functions, each with complexity 1-3
// Weighted count: ~35
// Score: 12 (below threshold)
```

**God Object** (correctly flagged):
```rust
// 10 functions, each with complexity 17-33
// Weighted count: ~150
// Score: 65 (above threshold)
```
```

### Architecture Updates

Add section to `ARCHITECTURE.md`:

```markdown
### Complexity Weighting System

**Location**: `src/organization/complexity_weighting.rs`

**Purpose**: Prevent false positives in god object detection by weighting
functions by complexity rather than raw count.

**Formula**: `weight = max(1, cyclomatic_complexity / 3)^1.5`

**Integration**: Used by `GodObjectDetector` to calculate weighted method counts.
```

## Implementation Notes

### Gotchas

1. **Test Functions**: Exclude functions with `#[test]` or `#[cfg(test)]` from both raw and weighted counts
2. **Pure Functions**: Consider additional weight reduction for pure functions (see Spec 118)
3. **Edge Cases**: Handle files with no complexity data (fall back to raw count with warning)
4. **Threshold Tuning**: May need to adjust `max_methods` threshold from 20 to 30-40 for weighted counts

### Best Practices

1. **Always Log Both Counts**: Show both raw and weighted counts in output for transparency
2. **Expose Complexity Distribution**: Help users understand why a file is/isn't a god object
3. **Gradual Rollout**: Add `--complexity-weighted` flag initially, make default after validation
4. **Benchmark**: Test on 10+ real codebases before making default

### Example Output

```
#1 SCORE: 65.0 [CRITICAL - FILE - GOD OBJECT]
├─ ./src/builders/unified_analysis.rs (1200 lines, 15 functions)
├─ COMPLEXITY: weighted=85.3 (raw=15), avg=17.2, max=33
├─ WHY: High complexity functions indicate god object anti-pattern
│  ├─ 3 functions with complexity >30
│  ├─ 5 functions with complexity 15-30
│  ├─ 7 functions with complexity <15
├─ ACTION: Refactor high-complexity functions first...
```

## Migration and Compatibility

### Breaking Changes

None. New weighted scoring is opt-in initially via `--complexity-weighted` flag.

### Migration Path

1. **Phase 1 (v0.2.9)**: Add `--complexity-weighted` flag, keep raw count as default
2. **Phase 2 (v0.3.0)**: Make weighted scoring default, add `--raw-count` flag for legacy behavior
3. **Phase 3 (v0.4.0)**: Remove `--raw-count` flag, fully migrate to weighted scoring

### Compatibility Considerations

- **JSON Output**: Add new fields `weighted_method_count`, `avg_complexity` without removing `method_count`
- **Existing Scripts**: Continue to work with raw `method_count` field
- **Thresholds**: Provide both raw and weighted thresholds in configuration
