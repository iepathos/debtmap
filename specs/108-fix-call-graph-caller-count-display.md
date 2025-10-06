---
number: 108
title: Fix Call Graph Caller Count Display Inconsistency
category: compatibility
priority: high
status: draft
dependencies: []
created: 2025-10-06
---

# Specification 108: Fix Call Graph Caller Count Display Inconsistency

**Category**: compatibility
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The debtmap output displays inconsistent caller information in different sections of the priority recommendations. Specifically:

- **"Dependency Score" section** (line 430 in `formatter_verbosity.rs`): Shows `item.unified_score.dependency_factor as u32` (a normalized score 0-10)
- **"CALLS:" section** (line 608): Shows `item.upstream_callers.len()` (the actual caller count)
- **"CALLERS:" detail listing** (line 570-576): Shows the actual list of upstream callers

This creates confusing output where a function might show "0 callers" in the dependency score line but then list 14 actual callers in the CALLS section.

**Example from real output:**
```
├─ Dependency Score: 10.0 × 15% = 1.50 (0 callers)   ← WRONG: Shows normalized score instead of count
...
├─ CALLS: 14 callers, 0 callees                       ← CORRECT: Shows actual count
│  ├─ CALLERS: ContextRuleEngine::parse_config_rule, FunctionVisitor::has_test_name_pattern, ...
```

This bug undermines user trust in the analysis and makes the "No callers detected - may be dead code" warning appear for functions that are actually heavily used.

## Objective

Fix the caller count display in the dependency score section to show the actual number of upstream callers (`item.upstream_callers.len()`) instead of the normalized dependency factor score.

## Requirements

### Functional Requirements

1. **Correct Caller Count Display**
   - Display `item.upstream_callers.len()` in the dependency score parenthetical
   - Ensure consistency across all verbosity levels
   - Maintain the same display format

2. **Consistency Validation**
   - Verify all sections display the same caller count
   - Ensure "No callers detected" warning only shows when `upstream_callers.len() == 0`
   - Validate against test cases with known caller counts

### Non-Functional Requirements

1. **Backward Compatibility**
   - Maintain the same output format (only change the displayed number)
   - No changes to data structures or scoring logic
   - No impact on JSON output or other formatters

2. **Testing**
   - Add regression test for caller count display
   - Verify against real-world examples from debtmap self-analysis
   - Ensure consistency across verbosity levels 0, 1, and 2

## Acceptance Criteria

- [ ] Dependency score line shows actual caller count from `upstream_callers.len()`
- [ ] Caller count matches between "Dependency Score" and "CALLS:" sections
- [ ] "No callers detected" warning only appears when `upstream_callers.len() == 0`
- [ ] Test case added that validates caller count consistency
- [ ] Verified against debtmap self-analysis (ContextMatcher::any should show 14 callers in both places)
- [ ] All existing tests still pass
- [ ] No regression in other output formatters (markdown, JSON)

## Technical Details

### Implementation Approach

1. **Single Line Fix**
   ```rust
   // File: src/priority/formatter_verbosity.rs
   // Line 430 (approximately)

   // BEFORE:
   item.unified_score.dependency_factor as u32

   // AFTER:
   item.upstream_callers.len()
   ```

2. **Verification Points**
   - Check `format_basic_call_graph()` at line 608 - already correct
   - Check `format_call_graph_section()` at line 570-576 - already correct
   - Check markdown formatter for similar issues
   - Check JSON output (should be unaffected)

### Architecture Changes

None required - this is a display-only bug fix.

### Data Structures

No changes to data structures. The correct data already exists in `UnifiedDebtItem`:
```rust
pub struct UnifiedDebtItem {
    // ...
    pub upstream_dependencies: usize,        // Used in dependency_factor calculation
    pub upstream_callers: Vec<String>,       // Should be used for display
    pub downstream_callees: Vec<String>,
    // ...
}
```

### APIs and Interfaces

No API changes - this is purely a formatting fix.

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/priority/formatter_verbosity.rs` (line ~430)
  - Potentially `src/priority/formatter.rs` if similar bug exists
  - Potentially `src/priority/formatter_markdown.rs` if similar bug exists
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**:
  - Add test case that validates caller count consistency
  - Create test with known upstream callers (e.g., 14 callers)
  - Verify the dependency score line shows the correct count

- **Integration Tests**:
  - Run debtmap self-analysis and verify ContextMatcher::any shows 14 in both places
  - Check multiple functions with varying caller counts (0, 1, 14, 19, etc.)
  - Ensure "dead code" warning appears only for 0 callers

- **Regression Tests**:
  - Ensure all existing output format tests still pass
  - Verify JSON output unchanged
  - Verify markdown output consistency

## Test Case Example

```rust
#[test]
fn test_caller_count_display_consistency() {
    let item = create_test_item_with_callers(14); // 14 upstream callers
    let output = format_priority_item(&item, &ColoredFormatter::new(false), 0);

    // Should show "14 callers" in dependency score line
    assert!(output.contains("(14 callers)"));

    // Should also show "14 callers" in CALLS section
    assert!(output.contains("CALLS: 14 callers"));

    // Should NOT show dead code warning
    assert!(!output.contains("No callers detected"));
}

#[test]
fn test_dead_code_warning_only_for_zero_callers() {
    let item = create_test_item_with_callers(0); // 0 upstream callers
    let output = format_priority_item(&item, &ColoredFormatter::new(false), 0);

    // Should show "0 callers" in dependency score line
    assert!(output.contains("(0 callers)"));

    // If there are callees, should show dead code warning
    if item.downstream_callees.len() > 0 {
        assert!(output.contains("No callers detected - may be dead code"));
    }
}
```

## Documentation Requirements

- **Code Documentation**:
  - Add comment explaining why we use `upstream_callers.len()` and not `dependency_factor`
  - Document the relationship between caller count and dependency score

- **User Documentation**:
  - Update CHANGELOG.md with bug fix entry
  - Add to release notes for next version

- **Architecture Updates**:
  - No architecture documentation changes needed (bug fix only)

## Implementation Notes

### Root Cause Analysis

The bug was introduced when displaying the dependency score breakdown. The developer likely copy-pasted from the dependency factor calculation and forgot to change it to the actual caller count for display purposes.

**Why this happened:**
- `dependency_factor` is a normalized score (0-10) based on caller count
- For display, users expect to see the raw count, not the normalized score
- The confusion arose because both are related to callers/dependencies

**Relationship:**
```rust
// Scoring (internal):
dependency_factor = normalize_caller_count(upstream_callers.len()) // 0-10

// Display (user-facing):
"(N callers)" where N = upstream_callers.len()  // Actual count
```

### Similar Issues to Check

1. **Check all formatters** for this pattern:
   - `src/priority/formatter.rs`
   - `src/priority/formatter_markdown.rs`
   - `src/io/writers/enhanced_markdown/mod.rs`

2. **Search for other score displays** that might have similar bugs:
   - Complexity score display (should show complexity value, not factor)
   - Coverage score display (should show percentage, not factor)

### Edge Cases

1. **0 Callers**: Should show "(0 callers)" and potentially dead code warning
2. **1 Caller**: Should show "(1 caller)" - singular form handled by CALLS section
3. **Many Callers (>100)**: Should show actual count, might need formatting
4. **Entry Points**: External entry points have 0 measured callers but aren't dead code
   - Future enhancement: Distinguish between "truly dead" and "entry point"

## Migration and Compatibility

**No Breaking Changes:**
- Output format remains the same, only the displayed number changes
- JSON structure unchanged
- No API or data structure changes
- Existing tests should pass (or will catch the bug if they were checking the wrong value)

**User Impact:**
- Positive impact: Users will see consistent caller counts
- No negative impact: Only fixes incorrect information
- Trust restored: Eliminates confusing discrepancies in output

## Future Enhancements

After this fix, consider:

1. **Add Call Graph Validation**
   - Automated check that caller count matches across all display sections
   - CI test that runs debtmap self-analysis and validates consistency

2. **Entry Point Detection**
   - Mark functions as entry points when appropriate (main, test functions, etc.)
   - Change "No callers detected" to "Entry point - no internal callers" for clarity

3. **Dead Code Confidence**
   - Distinguish between "definitely dead" (0 callers, 0 callees) and "potentially dead" (0 callers, some callees)
   - Add confidence level to dead code warnings

4. **Comprehensive Display Testing**
   - Add property-based tests that verify display consistency
   - Ensure all score displays match their underlying data
