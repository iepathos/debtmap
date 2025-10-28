---
number: 153
title: Remove Leftover Formatting Artifacts from Output
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-10-28
---

# Specification 153: Remove Leftover Formatting Artifacts from Output

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The debtmap output currently contains leftover formatting artifacts that appear to be remnants from when emojis were used in the output. These artifacts make the output harder to read and don't add meaningful information:

**Example Current Output**:
```
#6 SCORE: 59.9 [CRITICAL - FILE - GOD OBJECT]
└─ ./src/cache/shared_cache/mod.rs (1201 lines, 69 functions)
└─ WHY: This struct violates single responsibility principle...
└─ ACTION: URGENT: 1201 lines, 69 functions! Split by data flow...
  - RECOMMENDED SPLITS (2 modules):
  -  [M] mod_construction.rs - Construction (6 methods, ~120 lines) [[*][*] Medium]
       -> Methods: new(), new_with_cache_dir(), new_with_location(), ... +3 more
  -  [M] mod_utilities.rs - Utilities (40 methods, ~800 lines) [[*][*] Medium]
       -> Methods: fmt(), determine_pruning_config(), should_prune_after_insertion(), ... +37 more
```

**Problematic Artifacts**:
1. `[M]` prefix before module names (appears to stand for "Module")
2. `[[*][*][*] High]`, `[[*][*] Medium]`, `[[*] Low]` priority indicators (leftover emoji placeholders)

These formatting artifacts:
- Clutter the output with non-informative markup
- Make it harder to quickly scan and read recommendations
- Don't convey useful semantic information
- Appear to be placeholders for removed emoji indicators

## Objective

Clean up the debtmap output by removing leftover formatting artifacts (`[M]` and `[[*][*]...]` priority indicators) while preserving the actual information content. The output should be cleaner and easier to read without sacrificing any meaningful data.

## Requirements

### Functional Requirements

**Remove `[M]` Prefix**:
- Remove the `[M]` prefix from module split recommendations
- Keep the module name and responsibility information
- Maintain the visual hierarchy and indentation

**Simplify Priority Indicators**:
- Replace `[[*][*][*] High]` with `High Priority`
- Replace `[[*][*] Medium]` with `Medium Priority`
- Replace `[[*] Low]` with `Low Priority`
- OR remove priority indicators entirely if not providing value

**Preserve Information Content**:
- Keep all semantic information (module names, line counts, method counts, priority levels)
- Maintain visual hierarchy and tree structure
- Preserve readability of recommendations

**Clean Output Format**:
- Use plain text without unnecessary markup
- Focus on clarity and scannability
- Consistent formatting across all recommendations

### Non-Functional Requirements

- **Backward Compatibility**: Output format changes should not break parsers (if any exist)
- **Performance**: No performance impact from formatting changes
- **Maintainability**: Simpler formatting code is easier to maintain
- **Consistency**: Apply same cleanup to all output sections that use these artifacts

## Acceptance Criteria

- [ ] `[M]` prefix removed from all module split recommendations
- [ ] `[[*][*][*] High]`, `[[*][*] Medium]`, `[[*] Low]` replaced with cleaner priority indicators
- [ ] All semantic information (counts, names, priorities) preserved
- [ ] Visual hierarchy maintained (indentation, tree structure)
- [ ] Output is easier to read and scan
- [ ] No regression in information content
- [ ] Test suite validates output format changes
- [ ] Documentation updated if output format is documented

## Technical Details

### Implementation Approach

**Phase 1: Remove `[M]` Prefix**

Location: `src/priority/formatter.rs:860`

**Before**:
```rust
writeln!(
    output,
    "  {}  [M] {}.{} - {} ({} methods, ~{} lines) [{}]",
    branch,
    split.suggested_name,
    extension,
    split.responsibility,
    split.methods_to_move.len(),
    split.estimated_lines,
    priority_indicator
)
```

**After**:
```rust
writeln!(
    output,
    "  {}  {}.{} - {} ({} methods, ~{} lines) {}",
    branch,
    split.suggested_name,
    extension,
    split.responsibility,
    split.methods_to_move.len(),
    split.estimated_lines,
    priority_indicator
)
```

**Phase 2: Simplify Priority Indicators**

Location: `src/priority/formatter.rs:851-855`

**Before**:
```rust
let priority_indicator = match split.priority {
    crate::priority::file_metrics::Priority::High => "[*][*][*] High",
    crate::priority::file_metrics::Priority::Medium => "[*][*] Medium",
    crate::priority::file_metrics::Priority::Low => "[*] Low",
};
```

**Option A - Simplified Text**:
```rust
let priority_indicator = match split.priority {
    crate::priority::file_metrics::Priority::High => "[High Priority]",
    crate::priority::file_metrics::Priority::Medium => "[Medium Priority]",
    crate::priority::file_metrics::Priority::Low => "[Low Priority]",
};
```

**Option B - Minimal** (preferred for clean output):
```rust
let priority_indicator = match split.priority {
    crate::priority::file_metrics::Priority::High => "[High]",
    crate::priority::file_metrics::Priority::Medium => "[Medium]",
    crate::priority::file_metrics::Priority::Low => "[Low]",
};
```

**Option C - Remove Entirely** (if priority isn't adding value):
```rust
// Remove priority indicator entirely if it's not actionable
// Just show: "mod_construction.rs - Construction (6 methods, ~120 lines)"
```

**Recommended**: Option B for balance between information and clarity.

**Phase 3: Expected Output**

**After Changes**:
```
  - RECOMMENDED SPLITS (2 modules):
  -  mod_construction.rs - Construction (6 methods, ~120 lines) [Medium]
       -> Methods: new(), new_with_cache_dir(), new_with_location(), ... +3 more
  -  mod_utilities.rs - Utilities (40 methods, ~800 lines) [Medium]
       -> Methods: fmt(), determine_pruning_config(), should_prune_after_insertion(), ... +37 more
```

Much cleaner and easier to read!

### Architecture Changes

**Modified Module**: `src/priority/formatter.rs`
- Update `format_split_recommendations()` function (lines 851-868)
- Simplify priority indicator formatting
- Remove `[M]` prefix from output format string

**No Breaking Changes**:
- This is purely output formatting
- No data structure or API changes
- No impact on analysis logic

### Alternative Approaches Considered

**Alternative 1: Keep Emojis**
- Use actual emojis instead of `[*]` placeholders
- Rejected: Per project guidelines, avoid emojis unless explicitly requested

**Alternative 2: Color-Coded Output**
- Use ANSI colors for priority levels (High=Red, Medium=Yellow, Low=Green)
- Deferred: Would require terminal capability detection, adds complexity

**Alternative 3: Verbose Priority Text**
- Use full text like "High Priority - Address Urgently"
- Rejected: Too verbose, makes output harder to scan

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/priority/formatter.rs` - output formatting

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_does_not_contain_m_prefix() {
        let recommendation = create_test_split_recommendation();
        let output = format_split_recommendations(&recommendation);

        // Should not contain [M] prefix
        assert!(!output.contains("[M]"));
    }

    #[test]
    fn output_does_not_contain_asterisk_priority_indicators() {
        let recommendation = create_test_split_recommendation();
        let output = format_split_recommendations(&recommendation);

        // Should not contain old-style priority indicators
        assert!(!output.contains("[*][*][*]"));
        assert!(!output.contains("[[*][*]"));
        assert!(!output.contains("[[*]"));
    }

    #[test]
    fn output_contains_clean_priority_indicators() {
        let high_priority_rec = create_test_split_with_priority(Priority::High);
        let output = format_split_recommendations(&high_priority_rec);

        // Should contain clean priority indicator
        assert!(output.contains("[High]"));
        assert!(!output.contains("[*]"));
    }

    #[test]
    fn output_preserves_semantic_information() {
        let recommendation = create_test_split_recommendation();
        let output = format_split_recommendations(&recommendation);

        // Should still contain all important information
        assert!(output.contains("mod_construction.rs"));
        assert!(output.contains("6 methods"));
        assert!(output.contains("~120 lines"));
        assert!(output.contains("Construction"));
    }
}
```

### Integration Tests

```rust
#[test]
fn full_output_has_clean_formatting() {
    // Run debtmap on test project
    let output = run_debtmap_on_test_project();

    // Check for absence of artifacts
    assert!(!output.contains("[M]"), "Should not contain [M] prefix");
    assert!(!output.contains("[*][*]"), "Should not contain asterisk priority indicators");

    // Check for presence of clean formatting
    assert!(output.contains("RECOMMENDED SPLITS"));
    assert!(output.contains("[High]") || output.contains("[Medium]") || output.contains("[Low]"));
}
```

### Manual Testing

Test debtmap output on various codebases:
```bash
# Test on debtmap's own codebase
cargo run -- src/

# Verify output doesn't contain [M] or [[*][*]...]
cargo run -- src/ | grep -c "\[M\]"  # Should be 0
cargo run -- src/ | grep -c "\[\*\]"  # Should be 0
```

## Documentation Requirements

### User Documentation

If output format is documented in README.md, update examples:

**Update README.md Output Examples**:
```markdown
## Example Output

Debtmap identifies technical debt and provides actionable recommendations:

```
#6 SCORE: 59.9 [CRITICAL - FILE - GOD OBJECT]
└─ ./src/cache/shared_cache/mod.rs (1201 lines, 69 functions)
└─ WHY: This struct violates single responsibility principle...
└─ ACTION: URGENT: 1201 lines, 69 functions! Split by data flow...
  - RECOMMENDED SPLITS (2 modules):
  -  mod_construction.rs - Construction (6 methods, ~120 lines) [Medium]
       -> Methods: new(), new_with_cache_dir(), new_with_location(), ... +3 more
  -  mod_utilities.rs - Utilities (40 methods, ~800 lines) [Medium]
       -> Methods: fmt(), determine_pruning_config(), ... +37 more
```
```

### Code Documentation

Update function documentation in `src/priority/formatter.rs`:

```rust
/// Formats module split recommendations with clean, readable output.
///
/// Priority levels are indicated as [High], [Medium], or [Low] for clarity.
/// All semantic information (method counts, line estimates, responsibilities)
/// is preserved while removing unnecessary formatting artifacts.
fn format_split_recommendations(...) -> String {
    // ...
}
```

## Implementation Notes

### Design Decisions

**Why Remove `[M]` Prefix?**
- Doesn't add information (it's obvious these are modules from context)
- Makes output cluttered and harder to scan
- Appears to be leftover from earlier formatting system

**Why Simplify Priority Indicators?**
- `[[*][*][*]]` looks like placeholder text that wasn't replaced
- Actual text (`[High]`, `[Medium]`, `[Low]`) is clearer
- Reduces visual noise while preserving information

**Why Not Use Emojis?**
- Project guidelines discourage emoji use
- Not all terminals/environments render emojis consistently
- Plain text is more universally readable

### Refactoring Opportunity

While making these changes, consider:
1. Extracting priority formatting to a separate function
2. Making output format configurable (JSON, plain text, verbose, etc.)
3. Adding tests for output formatting

## Migration and Compatibility

### Breaking Changes

**Minimal Impact**:
- Output format change only
- No API or data structure changes
- If users are parsing output (not recommended), they'll need minor updates

**Recommended Migration**:
- Users should not parse free-form text output
- For programmatic access, recommend using JSON output mode (if available)
- Or add structured output format in future spec

## Expected Impact

### Before (Current)
```
  - RECOMMENDED SPLITS (2 modules):
  -  [M] mod_construction.rs - Construction (6 methods, ~120 lines) [[*][*] Medium]
       -> Methods: new(), new_with_cache_dir(), new_with_location(), ... +3 more
  -  [M] mod_utilities.rs - Utilities (40 methods, ~800 lines) [[*][*] Medium]
       -> Methods: fmt(), determine_pruning_config(), should_prune_after_insertion(), ... +37 more
```

### After (Cleaned)
```
  - RECOMMENDED SPLITS (2 modules):
  -  mod_construction.rs - Construction (6 methods, ~120 lines) [Medium]
       -> Methods: new(), new_with_cache_dir(), new_with_location(), ... +3 more
  -  mod_utilities.rs - Utilities (40 methods, ~800 lines) [Medium]
       -> Methods: fmt(), determine_pruning_config(), should_prune_after_insertion(), ... +37 more
```

**Improvements**:
- 10 fewer characters per line (less visual clutter)
- Easier to scan and read
- More professional appearance
- No loss of information content

### User Benefits

- **Clarity**: Easier to read and understand recommendations
- **Professionalism**: Output looks polished and intentional
- **Scannability**: Less visual noise makes it easier to find important information
- **Maintainability**: Simpler formatting code is easier to maintain

## Success Metrics

- [ ] `[M]` prefix completely removed from output
- [ ] `[[*][*]...]` priority indicators replaced with clean text
- [ ] All information content preserved
- [ ] Visual hierarchy maintained
- [ ] Tests pass with updated output format
- [ ] User feedback: Output is easier to read
- [ ] No regression in functionality
