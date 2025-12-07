# Score Double-Counting Analysis

## Problem Statement

The list view shows `total_debt_score: 66435` but the user observes:
- 94 total issues displayed
- Most critical score is 1277 (item #1)
- After item #17, scores are less than 100

**Simple Math**: Even if all 94 items scored 100 (which they don't), that would only be 9,400. The reported 66,435 doesn't match.

## Root Cause Investigation

### Location: `src/priority/mod.rs::calculate_total_impact()`

```rust
pub fn calculate_total_impact(&mut self) {
    // ...
    let mut total_debt_score = 0.0;

    for item in &self.items {
        // Sum up all final scores as the total debt score
        total_debt_score += item.unified_score.final_score;  // ← Function-level items
        // ...
    }

    // Add file-level impacts
    for file_item in &self.file_items {
        total_debt_score += file_item.score;  // ← File-level items
        // ...
    }

    // ...
    self.total_debt_score = total_debt_score;
}
```

## The Bug

**The total_debt_score sums BOTH function-level items AND file-level items.**

### What This Means

1. **Function-level items** (`self.items`): Individual function debt like high complexity, impure functions, etc.
2. **File-level items** (`self.file_items`): File-level debt like god objects, test gaps

### The Double-Counting Scenario

If a file has:
- A god object issue (file-level): score 1000
- 10 complex functions in that same file (function-level): 10 × 50 = 500

**Displayed to user**: 11 issues (1 god object + 10 functions)
**Reported total_debt_score**: 1000 + 500 = 1500

This is correct IF the file-level and function-level issues are independent.

But if there are many file-level items, the total score can be much higher than expected.

## Hypothesis

Given the numbers:
- 94 displayed items
- 66,435 total score
- Average per item would be: 66,435 ÷ 94 = **706.7**

But the user says most items are under 100 after #17.

**This suggests:**
1. There are high-scoring file-level items NOT being displayed in the main list
2. OR file-level items ARE displayed but have very high scores
3. OR there's a mismatch between what's being counted in `items` vs what's displayed

## Investigation Steps

### Step 1: Check what's in the list view

The list view at `src/tui/results/list_view.rs` displays items from `app.filtered_items()`:

```rust
format!("{:.0}", analysis.total_debt_score)  // ← Shows 66,435
```

But the list displays items from:
```rust
let visible_items = app.filtered_items();
```

### Step 2: Verify filtered_items

Need to check if `filtered_items()` includes both function-level and file-level items, or just one type.

### Step 3: Check file_items

File-level items (god objects, test gaps) might have very high scores that aren't being displayed in the main list.

## Likely Scenarios

### Scenario A: File-level items not displayed
- The 94 items shown are function-level items (from `self.items`)
- File-level items (`self.file_items`) are NOT shown in the list
- But their scores ARE included in `total_debt_score`
- This would explain the discrepancy

### Scenario B: File-level items have huge scores
- The 94 items include both types
- A few god object items have scores in the thousands/tens of thousands
- The user is only looking at the first page and misses the high-scoring items

### Scenario C: Aggregation bug
- Multiple debt types for the same location (e.g., complexity + purity + mutation)
- Each creates a separate `UnifiedDebtItem` but they're at the same location
- When displayed, they might be collapsed or shown as one
- But when scoring, each is counted separately

## Verification Questions

1. **How many file_items vs function items?**
   - `analysis.file_items.len()` vs `analysis.items.len()`

2. **What are the top file-level scores?**
   - Sort `file_items` by score and check top 10

3. **Are file items shown in the list?**
   - Check if `filtered_items()` includes `file_items`

4. **Is there location-based deduplication?**
   - Multiple items at same location counted separately in score but shown as one?

## Proposed Fix (Pending Investigation)

### Option 1: Don't Double-Count
If file-level and function-level issues represent overlapping concerns:
```rust
// Only count function-level items
for item in &self.items {
    total_debt_score += item.unified_score.final_score;
}
// Don't add file_items scores - they're aggregations of function issues
```

### Option 2: Separate Scoring
Keep both but make it clear:
```rust
self.function_debt_score = function_score;
self.file_debt_score = file_score;
self.total_debt_score = function_score + file_score;
```

Then display: "Functions: 5,000 | Files: 61,435 | Total: 66,435"

### Option 3: Only Count What's Displayed
```rust
// Only sum scores for items that will be shown to the user
total_debt_score = displayed_items
    .iter()
    .map(|i| i.score)
    .sum();
```

## Next Steps

1. Add debug logging to see:
   - `items.len()` vs `file_items.len()`
   - Top 10 scores from each
   - Sample of what's in each collection

2. Check `filtered_items()` implementation:
   - Does it combine both types?
   - Does it deduplicate by location?

3. Verify user's specific case:
   - Run debtmap on their codebase
   - Check actual counts and scores
   - Identify which scenario applies

## Conclusion

The 66,435 score likely comes from summing both function-level and file-level debt scores. If file-level items aren't prominently displayed or have very high scores, this would explain the user's confusion.

**The key question**: Should file-level scores be added to function-level scores, or are they already aggregations OF those function scores?
