---
number: 168
title: Fix File-Level Context Scoring Application
category: optimization
priority: critical
status: draft
dependencies: [166]
created: 2025-11-03
---

# Specification 168: Fix File-Level Context Scoring Application

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: [166 - Test File Detection and Context-Aware Scoring]

## Context

**Problem**: Spec 166 test file detection logic works correctly, but context adjustments are **not applied to file-level scores**.

**Current Behavior**:
```
#1 SCORE: 86.5 [CRITICAL]
└─ ./src/cook/workflow/git_context_diff_tests.rs (354 lines, 7 functions)
```

Test file detected correctly ✅, but score is **86.5 instead of ~17** ❌

**Root Cause Analysis**:

From `src/priority/file_metrics.rs:474`:
```rust
pub fn from_metrics(metrics: FileDebtMetrics) -> Self {
    let score = metrics.calculate_score();  // ← NO CONTEXT PASSED!

    FileDebtItem {
        metrics,
        score,  // ← Raw score without test file reduction
        ...
    }
}
```

Compare to **function-level items** in `src/priority/unified_analysis_utils.rs:204`:
```rust
let adjusted_score = apply_context_adjustments(item.unified_score.final_score, context);
```

Function scores ARE adjusted ✅
File scores ARE NOT adjusted ❌

**Impact on Prodigy Analysis**:
- `#1 SCORE: 86.5` - git_context_diff_tests.rs (should be ~17)
- `#3 SCORE: 65.6` - git_context_uncommitted_tests.rs (should be ~13)
- `#5 SCORE: 50.1` - git_context_commit_tests.rs (should be ~10)

3 of top 5 recommendations are test file false positives that should be ranked ~50-100th.

## Objective

Apply file context adjustments to file-level debt scores, ensuring test files receive the same 60-80% score reduction that function-level items already get.

## Requirements

### Functional Requirements

1. **Pass Context to Score Calculation**

   Modify `FileDebtItem::from_metrics()` to accept optional file context:
   ```rust
   pub fn from_metrics(
       metrics: FileDebtMetrics,
       context: Option<&FileContext>
   ) -> Self
   ```

2. **Apply Context Adjustments**

   Use existing `apply_context_adjustments()` function:
   ```rust
   let base_score = metrics.calculate_score();
   let score = if let Some(ctx) = context {
       apply_context_adjustments(base_score, ctx)
   } else {
       base_score
   };
   ```

3. **Update All Call Sites**

   Find and update all locations that call `from_metrics()` to pass context:
   - File debt aggregation
   - God object analysis output
   - Any test/mock construction

4. **Preserve Existing Behavior**

   When context is `None`, use unadjusted score (backward compatibility)

### Non-Functional Requirements

1. **Performance**: No performance regression (<1% overhead)
2. **Correctness**: File scores match function scores for same context
3. **Maintainability**: Clear API - `Option<&FileContext>` parameter
4. **Testing**: Comprehensive tests for all context types

## Acceptance Criteria

- [ ] `FileDebtItem::from_metrics()` accepts `Option<&FileContext>` parameter
- [ ] Test files (confidence >0.8) get 80% score reduction
- [ ] Probable test files (confidence 0.5-0.8) get 40% reduction
- [ ] Production files (no context or Production) get no reduction
- [ ] Generated files get 90% reduction
- [ ] All call sites updated to pass context
- [ ] Prodigy test files score ~17, ~13, ~10 (not 86.5, 65.6, 50.1)
- [ ] Test files move from top 5 to outside top 50
- [ ] Function-level and file-level scores consistent for same file
- [ ] Unit tests verify score reduction for all context types
- [ ] Integration test on prodigy codebase validates correct ranking
- [ ] No compilation errors or warnings
- [ ] All existing tests pass

## Technical Details

### Implementation Approach

**Phase 1: Modify Signature**

```rust
// src/priority/file_metrics.rs:474

impl FileDebtItem {
    pub fn from_metrics(
        metrics: FileDebtMetrics,
        context: Option<&FileContext>
    ) -> Self {
        let base_score = metrics.calculate_score();

        // Apply context-aware adjustments
        let score = if let Some(ctx) = context {
            use crate::priority::scoring::file_context_scoring::apply_context_adjustments;
            apply_context_adjustments(base_score, ctx)
        } else {
            base_score
        };

        let recommendation = metrics.generate_recommendation();
        let impact = FileImpact {
            complexity_reduction: metrics.avg_complexity * metrics.function_count as f64 * 0.2,
            maintainability_improvement: (metrics.max_complexity as f64 - metrics.avg_complexity)
                * 10.0,
            test_effort: metrics.uncovered_lines as f64 * 0.1,
        };

        FileDebtItem {
            metrics,
            score,  // ← Now includes context adjustment!
            priority_rank: 0,
            recommendation,
            impact,
        }
    }
}
```

**Phase 2: Find All Call Sites**

```bash
rg "FileDebtItem::from_metrics" -A 1 -B 1
```

Expected locations:
1. God object analysis aggregation
2. File debt collection
3. Test utilities

**Phase 3: Update Each Call Site**

Example pattern:
```rust
// Before
let file_item = FileDebtItem::from_metrics(metrics);

// After
let context = file_contexts.get(&metrics.path);
let file_item = FileDebtItem::from_metrics(metrics, context);
```

**Phase 4: Add Tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::FileContext;

    #[test]
    fn test_file_item_with_test_context_reduces_score() {
        let metrics = create_test_metrics(); // base score ~86.5

        let test_context = FileContext::Test {
            confidence: 0.95,
            test_framework: Some("rust-std".to_string()),
            test_count: 7,
        };

        let item = FileDebtItem::from_metrics(metrics.clone(), Some(&test_context));

        // Should be reduced by 80% (multiplied by 0.2)
        assert!(item.score < 20.0);
        assert!(item.score > 15.0); // ~17.3 expected
    }

    #[test]
    fn test_file_item_without_context_unchanged() {
        let metrics = create_test_metrics(); // base score ~86.5

        let item = FileDebtItem::from_metrics(metrics.clone(), None);

        assert_eq!(item.score, metrics.calculate_score());
    }

    #[test]
    fn test_file_item_with_production_context_unchanged() {
        let metrics = create_test_metrics();

        let prod_context = FileContext::Production;
        let item = FileDebtItem::from_metrics(metrics.clone(), Some(&prod_context));

        assert_eq!(item.score, metrics.calculate_score());
    }

    #[test]
    fn test_file_item_with_generated_context_reduces_90_percent() {
        let metrics = create_test_metrics();

        let gen_context = FileContext::Generated {
            generator: "protoc".to_string(),
        };
        let item = FileDebtItem::from_metrics(metrics.clone(), Some(&gen_context));

        // Should be reduced by 90% (multiplied by 0.1)
        let base_score = metrics.calculate_score();
        assert!((item.score - base_score * 0.1).abs() < 0.5);
    }
}
```

### Architecture Changes

1. **Modified Method Signature**:
   ```rust
   // Before
   pub fn from_metrics(metrics: FileDebtMetrics) -> Self

   // After
   pub fn from_metrics(metrics: FileDebtMetrics, context: Option<&FileContext>) -> Self
   ```

2. **New Import**: Add `use crate::priority::scoring::file_context_scoring::apply_context_adjustments;`

3. **Call Site Updates**: All locations creating `FileDebtItem` must pass context

### Expected Score Changes

Based on prodigy analysis:

| File | Current Score | Expected Score | Reduction |
|------|---------------|----------------|-----------|
| git_context_diff_tests.rs | 86.5 | ~17.3 | 80% |
| git_context_uncommitted_tests.rs | 65.6 | ~13.1 | 80% |
| git_context_commit_tests.rs | 50.1 | ~10.0 | 80% |

Expected new top 5:
1. executor.rs (69.4) - Legitimate complexity
2. mapreduce/coordinator/executor.rs (57.4) - Legitimate
3. execute_command_by_type() (25.8) - Function-level
4. execute_mapreduce() (21.8) - Function-level
5. WorkflowCommand::to_command() (21.3) - Function-level

Test files drop to ~rank 50-100.

## Dependencies

- **Spec 166**: Provides `FileContext` and `apply_context_adjustments()`
- **Spec 167**: File-level score breakdown will show adjustment application

## Testing Strategy

### Unit Tests

```rust
// src/priority/file_metrics.rs

#[test]
fn test_context_adjustment_reduces_test_file_score() { ... }

#[test]
fn test_no_context_uses_base_score() { ... }

#[test]
fn test_production_context_no_reduction() { ... }

#[test]
fn test_generated_context_90_percent_reduction() { ... }

#[test]
fn test_probable_test_file_40_percent_reduction() { ... }
```

### Integration Tests

Run debtmap on prodigy and verify:
```bash
cargo run -- analyze ../prodigy --top 10 > output.txt

# Verify test files NOT in top 10
! grep -q "git_context.*tests.rs" output.txt | head -10

# Verify test files scored correctly
grep "git_context_diff_tests.rs" output.txt | grep "SCORE: 1[0-9]\.[0-9]"
```

### Validation Checklist

- [ ] Compile without errors
- [ ] All unit tests pass
- [ ] Integration test on prodigy shows correct ranking
- [ ] Test files outside top 10
- [ ] No regression on other projects (debtmap self-analysis)

## Documentation Requirements

### Code Documentation

```rust
/// Create a FileDebtItem from metrics with optional file context adjustments.
///
/// # Arguments
///
/// * `metrics` - File debt metrics containing raw calculations
/// * `context` - Optional file context for score adjustments
///
/// # Context Adjustments
///
/// - Test files (confidence >0.8): 80% reduction
/// - Probable test files (0.5-0.8): 40% reduction
/// - Generated files: 90% reduction
/// - Production files: No adjustment
///
/// # Example
///
/// ```
/// let metrics = calculate_file_metrics(&path, &analysis);
/// let context = detect_file_context(&path, &metrics);
/// let item = FileDebtItem::from_metrics(metrics, Some(&context));
/// // item.score now includes context adjustment
/// ```
pub fn from_metrics(
    metrics: FileDebtMetrics,
    context: Option<&FileContext>
) -> Self { ... }
```

### User Documentation

Update README.md:

```markdown
## Test File Detection (Spec 166)

Debtmap automatically detects test files and reduces their debt scores to avoid
false positives:

- **Rust**: Files matching `*_tests.rs`, `*_test.rs`, or containing `#[test]`
- **Python**: Files matching `test_*.py` or `*_test.py`
- **JavaScript/TypeScript**: Files matching `*.test.js` or `*.spec.ts`

Test files receive:
- 80% score reduction (high confidence >0.8)
- 40% score reduction (probable 0.5-0.8)

This applies to **both file-level and function-level** debt items.
```

### Architecture Updates

Update ARCHITECTURE.md:

```markdown
## File Context Scoring (Spec 166 + 168)

File context detection (spec 166) identifies test files using multiple signals.
Score adjustments are applied at **file creation time** in `FileDebtItem::from_metrics()`:

1. Detect file context → `FileContext` enum
2. Calculate base score → `metrics.calculate_score()`
3. Apply adjustment → `apply_context_adjustments(base_score, context)`
4. Store in `FileDebtItem.score`

This ensures test files score appropriately low throughout the analysis pipeline.
```

## Implementation Notes

### Call Site Search

To find all locations:
```bash
rg "FileDebtItem::from_metrics" -l
rg "from_metrics\(" src/priority/file_metrics.rs -A 5
```

### Backward Compatibility

Using `Option<&FileContext>` maintains backward compatibility:
- Old code: `from_metrics(metrics)` → compile error (intentional - force update)
- New code: `from_metrics(metrics, None)` → works, no adjustment
- Context code: `from_metrics(metrics, Some(&ctx))` → adjusted

**Alternative** (if backward compat needed):
```rust
// Keep old signature
pub fn from_metrics(metrics: FileDebtMetrics) -> Self {
    Self::from_metrics_with_context(metrics, None)
}

// Add new signature
pub fn from_metrics_with_context(
    metrics: FileDebtMetrics,
    context: Option<&FileContext>
) -> Self { ... }
```

### Edge Cases

1. **Missing context data**: Use `None` - no adjustment (safe default)
2. **Ambiguous context** (0.4-0.6 confidence): No adjustment (already handled in function)
3. **Multiple contexts**: Use primary context from detection
4. **Context mismatch** (test file but Production context): Trust detection

## Migration and Compatibility

### Breaking Changes

**API Change**:
```rust
// Before (old signature)
FileDebtItem::from_metrics(metrics)

// After (new signature)
FileDebtItem::from_metrics(metrics, context)
```

This is **intentionally breaking** to force code review of all call sites.

### Migration Steps

1. Update signature in `file_metrics.rs`
2. Find all call sites: `rg "from_metrics"`
3. For each call site, determine if context is available:
   - **Available**: Pass `Some(&context)`
   - **Not available**: Pass `None` (temporary - should be fixed later)
4. Run tests, fix compilation errors
5. Validate output on prodigy

### Rollout Plan

**Phase 1**: Update signature + basic call sites
**Phase 2**: Ensure context passed everywhere
**Phase 3**: Remove `None` fallbacks (all sites have context)

## Success Metrics

**Before Fix** (current prodigy output):
- Test files in top 5: 3 (60%)
- Test file #1 score: 86.5
- User confusion: High

**After Fix** (expected):
- Test files in top 5: 0 (0%)
- Test file #1 score: ~17 (rank ~50)
- User confidence: High

**Validation Criteria**:
- ✅ Test files score 80% lower
- ✅ Test files outside top 10
- ✅ Production files unchanged
- ✅ No false negatives (production files mis-scored as tests)

## Related Issues

- Closes spec 166 implementation gap
- Completes test file scoring feature
- Prepares for spec 167 score breakdown (will show adjustment)
- Fixes false positive recommendations on all test codebases

## Future Enhancements (Not in Scope)

- Adaptive reduction based on test quality metrics
- Custom reduction percentages via config
- Different adjustments for unit vs integration tests
- Context-aware recommendations (currently only score adjustment)
