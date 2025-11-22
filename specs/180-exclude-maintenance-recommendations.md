---
number: 180
title: Exclude Maintenance-Only Recommendations from Debt Reports
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-11-22
---

# Specification 180: Exclude Maintenance-Only Recommendations from Debt Reports

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Currently, debtmap can output "Maintain current low complexity" as a recommendation for functions that have low cyclomatic and cognitive complexity (typically < 8 cyclomatic, < 15 cognitive). These are informational recommendations indicating the code is already in good shape.

However, these maintenance-only recommendations are being included in debt reports alongside actual technical debt issues. This creates noise in the output and inflates the apparent number of issues, making it harder for users to focus on actual problems that need fixing.

**Example of the problem:**

```
Primary Action: Maintain current low complexity

Rationale: Function has low complexity (6/6). Continue following current
patterns to keep it maintainable.

Steps:
- Add tests to preserve behavior during future changes
```

This isn't a debt item - it's praise for good code. It shouldn't appear alongside testing gaps, complexity hotspots, or dead code issues.

## Objective

Filter out maintenance-only recommendations from debt reports so that debtmap only raises issues for code that actually needs attention, not code that's already in good shape.

## Requirements

### Functional Requirements

1. **Detection of Maintenance Recommendations**
   - Identify debt items with "Maintain current low complexity" as the primary action
   - Recognize the Low complexity tier pattern (cyclomatic < 8, cognitive < 15)
   - Support pattern-based detection for future similar maintenance recommendations

2. **Filtering at Appropriate Layer**
   - Filter maintenance recommendations before they're included in debt items
   - Alternatively, filter during debt item creation for ComplexityHotspot type
   - Ensure filtering happens consistently across all output formats (JSON, Markdown, HTML)

3. **Preserve Valid Low-Complexity Issues**
   - Still report low-complexity functions with other issues:
     - Testing gaps (even if complexity is low)
     - Dead code (unused functions regardless of complexity)
     - Security issues
     - Other debt types
   - Only filter pure maintenance recommendations with no actionable work

### Non-Functional Requirements

1. **Performance**: Filtering should have negligible performance impact
2. **Maintainability**: Clear separation between "this is good code" and "this needs work"
3. **Clarity**: Output should only contain actionable items
4. **Backwards Compatibility**: Don't break existing filters or suppression mechanisms

## Acceptance Criteria

- [ ] Functions with complexity below Low tier threshold (< 8 cyclomatic, < 15 cognitive) do NOT generate ComplexityHotspot debt items
- [ ] Functions with low complexity but other issues (testing gaps, dead code) are still reported
- [ ] Dashboard and reports show accurate count of actual issues (excluding maintenance recommendations)
- [ ] JSON output excludes maintenance-only recommendations
- [ ] Markdown output excludes maintenance-only recommendations
- [ ] HTML dashboard excludes maintenance-only recommendations
- [ ] Existing tests pass with updated filtering logic
- [ ] New tests verify maintenance recommendations are excluded

## Technical Details

### Implementation Approach

**Option 1: Filter at Debt Item Creation (Recommended)**

Modify the complexity hotspot detection logic to not create debt items for low-complexity functions:

```rust
// In src/priority/scoring/classification.rs or similar

fn should_report_complexity_hotspot(
    cyclomatic: u32,
    cognitive: u32,
    tier: ComplexityTier,
) -> bool {
    // Don't report Low tier as debt - it's already good
    !matches!(tier, ComplexityTier::Low)
}
```

**Option 2: Filter During Unified Analysis**

Filter out maintenance recommendations in the unified analysis builder:

```rust
// In src/builders/unified_analysis.rs

fn filter_maintenance_recommendations(items: Vec<DebtItem>) -> Vec<DebtItem> {
    items.into_iter()
        .filter(|item| !is_maintenance_only_recommendation(item))
        .collect()
}

fn is_maintenance_only_recommendation(item: &DebtItem) -> bool {
    matches!(&item.debt_type,
        DebtType::ComplexityHotspot { cyclomatic, cognitive, .. }
        if *cyclomatic < 8 && *cognitive < 15
    )
}
```

**Option 3: Filter at Output Layer**

Filter in each output writer (less preferred - duplicates logic):

```rust
// In src/io/writers/*.rs

let actionable_items = debt_items.into_iter()
    .filter(|item| item.recommendation.primary_action != "Maintain current low complexity")
    .collect();
```

### Complexity Tier Thresholds

From `concise_recommendation.rs`:

```rust
enum RecommendationComplexityTier {
    Low,      // cyclo < 8, cognitive < 15
    Moderate, // cyclo 8-14, cognitive 15-24
    High,     // cyclo 15-24, cognitive 25-39
    VeryHigh, // cyclo >= 25, cognitive >= 40
}
```

**Decision**: Only filter `Low` tier. `Moderate` tier recommendations are still valuable preventive guidance.

### Edge Cases

1. **Low complexity with other issues**: Don't filter
   - Low complexity + testing gap → Report (testing is the issue)
   - Low complexity + dead code → Report (unused code is the issue)

2. **Adjusted complexity**: Use adjusted cyclomatic if available
   ```rust
   let effective_cyclomatic = adjusted_cyclomatic.unwrap_or(cyclomatic);
   ```

3. **Pattern detection override**: If a specific pattern is detected (dispatcher, state machine, etc.), those have their own recommendations - don't filter based on raw complexity alone

### Data Flow

```
Function Metrics
    ↓
Complexity Analysis
    ↓
Tier Classification (Low/Moderate/High/VeryHigh)
    ↓
[NEW] Filter Low Tier ComplexityHotspot
    ↓
Create Debt Items (only Moderate+ complexity)
    ↓
Generate Recommendations
    ↓
Output (JSON/Markdown/HTML)
```

## Dependencies

**Prerequisites**: None - this is a pure filtering improvement

**Affected Components**:
- `src/priority/scoring/classification.rs` - Where complexity tiers are determined
- `src/priority/scoring/concise_recommendation.rs` - Where recommendations are generated
- `src/builders/unified_analysis.rs` - Where debt items are aggregated
- `src/io/writers/*` - Output formatters (indirectly affected by fewer items)

**External Dependencies**: None

## Testing Strategy

### Unit Tests

1. **Test Low Complexity Filtering**
   ```rust
   #[test]
   fn test_low_complexity_not_reported_as_debt() {
       let metrics = create_function_metrics(5, 10); // Low tier
       let debt_items = generate_debt_items(&metrics);

       // Should NOT have ComplexityHotspot debt
       assert!(!debt_items.iter().any(|item|
           matches!(item.debt_type, DebtType::ComplexityHotspot { .. })
       ));
   }
   ```

2. **Test Low Complexity with Other Issues**
   ```rust
   #[test]
   fn test_low_complexity_with_testing_gap_still_reported() {
       let metrics = create_function_metrics_with_coverage(5, 10, 0.3);
       let debt_items = generate_debt_items(&metrics);

       // Should have TestingGap debt
       assert!(debt_items.iter().any(|item|
           matches!(item.debt_type, DebtType::TestingGap { .. })
       ));

       // Should NOT have ComplexityHotspot debt
       assert!(!debt_items.iter().any(|item|
           matches!(item.debt_type, DebtType::ComplexityHotspot { .. })
       ));
   }
   ```

3. **Test Moderate Complexity Still Reported**
   ```rust
   #[test]
   fn test_moderate_complexity_still_reported() {
       let metrics = create_function_metrics(10, 18); // Moderate tier
       let debt_items = generate_debt_items(&metrics);

       // SHOULD have ComplexityHotspot debt (preventive)
       assert!(debt_items.iter().any(|item|
           matches!(item.debt_type, DebtType::ComplexityHotspot { .. })
       ));
   }
   ```

### Integration Tests

1. **End-to-End Output Verification**
   - Run debtmap on sample codebase with mix of complexity levels
   - Verify JSON output excludes Low tier complexity items
   - Verify Markdown report excludes "Maintain current low complexity"
   - Verify HTML dashboard doesn't show Low tier items

2. **Count Accuracy**
   - Verify summary statistics don't include filtered maintenance items
   - Verify category breakdowns are accurate
   - Verify priority distributions exclude Low tier complexity

### Regression Tests

1. **Existing Behavior Preservation**
   - All existing tests for other debt types pass unchanged
   - Filter config still works correctly
   - Suppression rules still work correctly

## Documentation Requirements

### Code Documentation

- Document the filtering logic with clear comments explaining why Low tier is excluded
- Add documentation to `ComplexityTier` enum about reporting thresholds
- Update function documentation for debt item generation

### User Documentation

- Update README or user guide to explain that low-complexity functions aren't reported
- Add FAQ entry: "Why doesn't debtmap report my simple functions?"
  - Answer: "Functions with complexity below thresholds (cyclo < 8, cognitive < 15) are already maintainable and don't need refactoring"

### Architecture Updates

- Document the filtering decision in ARCHITECTURE.md if it exists
- Note the threshold values and rationale

## Implementation Notes

### Recommended Approach

1. **Phase 1**: Add filtering at debt item creation
   - Modify complexity hotspot detection to skip Low tier
   - Add unit tests for filtering logic

2. **Phase 2**: Verify output formats
   - Test JSON output
   - Test Markdown output
   - Test HTML dashboard

3. **Phase 3**: Update documentation
   - Code comments
   - User-facing docs

### Code Locations

Key files to modify:

1. **src/priority/scoring/classification.rs** or **src/priority/scoring/computation.rs**
   - Add `should_report_complexity_hotspot()` function
   - Check tier before creating ComplexityHotspot debt items

2. **src/priority/scoring/concise_recommendation.rs**
   - Already has tier classification - reuse this
   - Document that Low tier won't create debt items

3. **Tests**
   - Add test cases in existing test modules
   - Verify behavior across different complexity levels

### Performance Considerations

- Filtering is O(1) per item (simple tier check)
- No performance impact on analysis or scoring
- Slightly faster output generation (fewer items to process)

## Migration and Compatibility

### Breaking Changes

None - this is purely a reduction in output noise. Users will see:
- Fewer reported issues (more accurate count of actual problems)
- Cleaner reports focused on actionable items

### Backwards Compatibility

- Existing JSON consumers might see fewer items - this is intentional and beneficial
- Existing test suites expecting Low tier items will need updates
- Suppression rules become simpler (no need to suppress "good code")

### Migration Path

No migration needed for users. For developers:

1. Update test expectations to not expect Low tier complexity items
2. Review any custom filters that might have been working around this issue
3. Update documentation and examples

## Success Metrics

1. **Accuracy**: Reports only contain actionable technical debt
2. **Clarity**: Users can immediately see what needs fixing
3. **Signal-to-Noise**: Higher ratio of real issues to total items
4. **User Satisfaction**: Fewer false positives in debt reports

## Open Questions

1. **Should we provide an opt-in flag to show Low tier items?**
   - Probably not needed - if users want to see all functions, they can use complexity reports
   - Debt reports should focus on debt, not praise

2. **Should Low tier functions with unusually high nesting still be reported?**
   - Probably yes - nesting depth is a separate concern
   - But this should be detected as a different pattern, not generic Low tier

3. **Should the threshold be configurable?**
   - Could add `--min-complexity-threshold` flag
   - But defaults should work for 95% of users
   - Defer this to future enhancement

## Future Enhancements

1. **Positive Feedback Report**: Separate report showing well-maintained code
   - "Code Health Report" listing Low tier functions as exemplars
   - Could be useful for team reviews and knowledge sharing

2. **Trend Analysis**: Track when functions move from Moderate → Low tier
   - Show refactoring progress over time
   - Celebrate complexity reductions

3. **Configurable Tiers**: Allow users to customize tier thresholds
   - Different projects/languages might have different standards
   - Config file: `complexity_thresholds.toml`
