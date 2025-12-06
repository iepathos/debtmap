---
number: 215
title: Improve TUI List View Display Clarity
category: optimization
priority: medium
status: draft
dependencies: [214]
created: 2025-12-05
---

# Specification 215: Improve TUI List View Display Clarity

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 214 (Entropy-Adjusted Complexity)

## Context

The TUI list view (`src/tui/results/list_view.rs`) currently displays debt items with redundant and unclear labels:

**Current Display** (line 268-291):
```
▸ #1    CRITICAL    Score:152.3  src/main.rs::analyze  (Cov:45% Comp:15)
  #2    HIGH        Score:84.1   lib.rs::process       (Cov:12% Comp:22)
```

### Problems

1. **Redundant "Score:" Label**
   - The word "Score:" appears before every numeric score
   - Users understand the number is a score from context (severity, position)
   - Wastes horizontal space (6 characters per line)
   - Adds visual noise without information value

2. **Unclear "Comp" Metric**
   - Shows raw `cyclomatic_complexity` from `item.cyclomatic_complexity`
   - Users don't know if this is:
     - Cyclomatic complexity (control flow paths)
     - Cognitive complexity (mental burden)
     - Entropy-adjusted complexity (dampened by patterns)
     - Weighted complexity factor (used in scoring)
   - No indication of whether this number drives the score
   - Missing context about entropy adjustments

3. **Missing Adjusted Complexity**
   - Spec 214 adds entropy-adjusted complexity fields
   - Users can't see the impact of entropy dampening
   - No way to understand why pattern-heavy code scores lower
   - Transparency is lost

### User Impact

**Current experience**:
- "Why is this function with Comp:15 scored lower than one with Comp:10?"
- "What does Comp actually mean?"
- "Is entropy dampening working? I can't tell."
- "Score: Score: Score: everywhere is distracting"

**Desired experience**:
- Clear, self-explanatory complexity metric
- Visible entropy adjustment (if applied)
- More horizontal space for function names
- Cleaner, less redundant display

## Objective

Improve TUI list view clarity by:
1. Removing the redundant "Score:" label
2. Replacing "Comp" with a clearer complexity indicator
3. Showing entropy-adjusted complexity when available
4. Maintaining all essential information in limited space

## Requirements

### Functional Requirements

1. **Remove "Score:" Label**
   - Display score as bare number: `152.3` instead of `Score:152.3`
   - Maintain right-aligned formatting for visual scanning
   - Preserve color coding based on severity

2. **Improve Complexity Display**
   - Replace ambiguous "Comp:N" with clearer indicator
   - Show which complexity metric is displayed
   - Indicate when entropy dampening is applied

3. **Display Entropy Adjustment**
   - When entropy dampening is available, show: `Cog:15→9` (raw→adjusted)
   - When no entropy data, show: `Cog:15` (raw only)
   - Use `cognitive_complexity` as primary metric (more meaningful than cyclomatic)
   - Make adjustment visually clear with `→` symbol

4. **Maintain Information Density**
   - Don't exceed current line width
   - Prioritize most important metrics
   - Keep coverage, complexity, and location visible

### Non-Functional Requirements

1. **Visual Clarity**: Reduced clutter, improved readability
2. **Consistency**: Same format across all list items
3. **Performance**: No impact on rendering speed
4. **Accessibility**: Clear even on small terminals (80 columns minimum)

## Acceptance Criteria

- [ ] "Score:" label removed from all list items
- [ ] Bare numeric score displayed with same formatting and alignment
- [ ] "Comp" replaced with "Cog" (cognitive complexity)
- [ ] When entropy-adjusted complexity available: display `Cog:15→9` format
- [ ] When no entropy data: display `Cog:15` format
- [ ] Coverage display unchanged: `Cov:45%`
- [ ] All changes fit within current line width budget
- [ ] Visual regression test passes (screenshot comparison)
- [ ] Works correctly on 80-column terminals
- [ ] Documentation updated with new format examples

## Technical Details

### Implementation Approach

#### 1. Modify `format_list_item` Function

**Current** (`src/tui/results/list_view.rs:242-301`):

```rust
let line = Line::from(vec![
    Span::styled(indicator, Style::default().fg(theme.accent())),
    Span::styled(
        format!("#{:<4}", index + 1),
        Style::default().fg(theme.muted),
    ),
    Span::styled(
        format!("{:<10}", severity),
        Style::default().fg(severity_color),
    ),
    Span::styled(
        format!("Score:{:<7.1}", item.unified_score.final_score),  // REMOVE "Score:"
        Style::default().fg(theme.primary),
    ),
    Span::raw("  "),
    Span::styled(
        format!("{}::{}", file_name, item.location.function),
        Style::default().fg(theme.secondary()),
    ),
    Span::raw("  "),
    Span::styled(
        format!("(Cov:{} Comp:{})", coverage_str, complexity),  // IMPROVE THIS
        Style::default().fg(theme.muted),
    ),
]);
```

**Proposed**:

```rust
// Calculate complexity display string
let complexity_str = format_complexity_metric(item);

let line = Line::from(vec![
    Span::styled(indicator, Style::default().fg(theme.accent())),
    Span::styled(
        format!("#{:<4}", index + 1),
        Style::default().fg(theme.muted),
    ),
    Span::styled(
        format!("{:<10}", severity),
        Style::default().fg(severity_color),
    ),
    Span::styled(
        format!("{:<7.1}", item.unified_score.final_score),  // No "Score:" label
        Style::default().fg(theme.primary),
    ),
    Span::raw("  "),
    Span::styled(
        format!("{}::{}", file_name, item.location.function),
        Style::default().fg(theme.secondary()),
    ),
    Span::raw("  "),
    Span::styled(
        format!("(Cov:{} {})", coverage_str, complexity_str),
        Style::default().fg(theme.muted),
    ),
]);
```

#### 2. New Helper Function: `format_complexity_metric`

```rust
/// Format complexity metric for list view display
/// Shows entropy-adjusted complexity when available
fn format_complexity_metric(item: &UnifiedDebtItem) -> String {
    // Check if entropy-adjusted complexity is available (spec 214)
    if let Some(adjusted_cog) = item.entropy_adjusted_cognitive {
        let raw_cog = item.cognitive_complexity;

        // Show adjustment if there's a meaningful difference (>5%)
        let diff_pct = ((raw_cog as f64 - adjusted_cog as f64) / raw_cog as f64).abs();
        if diff_pct > 0.05 {
            format!("Cog:{}→{}", raw_cog, adjusted_cog)
        } else {
            // Negligible adjustment, just show raw
            format!("Cog:{}", raw_cog)
        }
    } else {
        // No entropy data, show raw cognitive complexity
        format!("Cog:{}", item.cognitive_complexity)
    }
}
```

**Alternative: Show cyclomatic with cognitive**

If horizontal space permits (depends on function name length):

```rust
fn format_complexity_metric(item: &UnifiedDebtItem) -> String {
    if let Some(adjusted_cog) = item.entropy_adjusted_cognitive {
        let raw_cog = item.cognitive_complexity;
        if (raw_cog - adjusted_cog) > 2 {  // Meaningful reduction
            format!("Cyc:{} Cog:{}→{}",
                item.cyclomatic_complexity,
                raw_cog,
                adjusted_cog)
        } else {
            format!("Cyc:{} Cog:{}",
                item.cyclomatic_complexity,
                raw_cog)
        }
    } else {
        format!("Cyc:{} Cog:{}",
            item.cyclomatic_complexity,
            item.cognitive_complexity)
    }
}
```

**Decision**: Use cognitive-only format initially to save space. Cyclomatic can be shown in detail view.

#### 3. Update Detail View (Optional Enhancement)

While in list view we show compact format, detail view can show full breakdown:

```
Complexity Metrics:
  Cyclomatic:        15
  Cognitive:         22
  Entropy-Adjusted:  14 (36% reduction)
  Dampening Factor:  0.64
  Pattern Repetition: 0.72 (high)
```

This is out of scope for this spec but noted for future enhancement.

### Display Format Examples

#### Example 1: No Entropy Data

```
▸ #1    CRITICAL    152.3  src/main.rs::analyze      (Cov:45% Cog:15)
  #2    HIGH         84.1  lib.rs::process_data      (Cov:12% Cog:22)
  #3    MEDIUM       42.3  parser.rs::parse_tokens   (Cov:78% Cog:8)
```

Character savings: 6 characters per line (removed "Score:")

#### Example 2: With Entropy Adjustment

```
▸ #1    CRITICAL    152.3  src/main.rs::analyze      (Cov:45% Cog:22→14)
  #2    HIGH         84.1  lib.rs::process_data      (Cov:12% Cog:18)
  #3    MEDIUM       42.3  parser.rs::validate       (Cov:78% Cog:25→8)
```

Shows:
- Line 1: Pattern-heavy code, reduced from 22 to 14
- Line 2: No meaningful adjustment (< 5% difference)
- Line 3: Highly repetitive validation, reduced from 25 to 8

#### Example 3: Long Function Names (Width Constraint)

```
▸ #1    CRITICAL    152.3  validation_engine.rs::validate_input_parameters  (Cov:45% Cog:22→14)
```

Still fits in 100 columns (current width budget).

### Architecture Changes

**Modified Components**:
- `src/tui/results/list_view.rs` - Update `format_list_item`

**New Components**:
- `format_complexity_metric` helper function (in `list_view.rs`)

**Affected Components**:
- None (purely display change)

### Data Dependencies

**Requires from Spec 214**:
- `UnifiedDebtItem.entropy_adjusted_cognitive: Option<u32>`
- `UnifiedDebtItem.entropy_adjusted_cyclomatic: Option<u32>` (optional)
- `UnifiedDebtItem.entropy_dampening_factor: Option<f64>` (for detail view)

**Fallback**:
- If Spec 214 not implemented: Show `Cog:N` (raw cognitive)
- Graceful degradation: No `→` arrow if entropy data missing

## Dependencies

**Prerequisites**:
- **Spec 214**: Provides entropy-adjusted complexity fields
  - If 214 not implemented: this spec still provides value (removes "Score:", shows "Cog" instead of "Comp")
  - Full benefit requires 214 for entropy adjustment display

**Affected Components**:
- TUI rendering system
- List view formatting

**External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_format_complexity_no_entropy() {
    let item = create_test_item(15, None);
    assert_eq!(format_complexity_metric(&item), "Cog:15");
}

#[test]
fn test_format_complexity_with_meaningful_adjustment() {
    let item = create_test_item(22, Some(14));
    assert_eq!(format_complexity_metric(&item), "Cog:22→14");
}

#[test]
fn test_format_complexity_negligible_adjustment() {
    let item = create_test_item(20, Some(19));
    // Less than 5% difference
    assert_eq!(format_complexity_metric(&item), "Cog:20");
}

#[test]
fn test_score_label_removed() {
    let line = format_list_item(&item, 0, false, &theme);
    let text = line.to_string();
    assert!(!text.contains("Score:"));
    assert!(text.contains("152.3"));
}
```

### Visual Regression Tests

1. **Screenshot Comparison**
   - Capture TUI screenshots before/after
   - Verify alignment and formatting
   - Check color coding preserved

2. **Terminal Size Tests**
   - Test on 80-column terminal
   - Test on 120-column terminal
   - Test on 200-column terminal
   - Verify no line wrapping or truncation

### Integration Tests

```rust
#[test]
fn test_list_view_rendering() {
    let app = create_test_app();
    let mut terminal = create_test_terminal();

    render(&mut terminal, &app);

    // Verify output format
    let buffer = terminal.backend().buffer();
    // Check for expected format patterns
}
```

### Manual Testing Checklist

- [ ] Display with 0 items
- [ ] Display with 1 item
- [ ] Display with 100+ items
- [ ] Display with very long function names
- [ ] Display with entropy adjustment
- [ ] Display without entropy data
- [ ] Scrolling works correctly
- [ ] Selection highlighting works
- [ ] Colors are correct
- [ ] Alignment is preserved

## Documentation Requirements

### Code Documentation

1. **Inline comments**:
   ```rust
   // Format complexity metric with entropy adjustment if available
   // Shows "Cog:22→14" when entropy reduces complexity by >5%
   // Shows "Cog:22" when no entropy or negligible adjustment
   ```

2. **Function docs**:
   ```rust
   /// Format complexity metric for compact list view display.
   ///
   /// Uses cognitive complexity as the primary metric since it better
   /// represents mental burden than cyclomatic complexity.
   ///
   /// When entropy-adjusted complexity is available (from spec 214):
   /// - Shows adjustment if reduction > 5%: "Cog:22→14"
   /// - Otherwise shows raw value: "Cog:22"
   ///
   /// # Examples
   /// ```
   /// let item = create_item_with_entropy(22, 14);
   /// assert_eq!(format_complexity_metric(&item), "Cog:22→14");
   /// ```
   ```

### User Documentation

Update README with new format examples:

```markdown
## TUI List View Format

Each line shows:
- Index number (#1, #2, ...)
- Severity (CRITICAL, HIGH, MEDIUM, LOW)
- **Debt Score** (numeric value, higher = more urgent)
- Function location (file::function)
- **Coverage** (test coverage percentage)
- **Cognitive Complexity** (mental burden metric)
  - `Cog:22→14` - Entropy-adjusted (pattern-heavy code)
  - `Cog:22` - Raw complexity (no adjustment)

Example:
```
▸ #1    CRITICAL    152.3  src/main.rs::analyze      (Cov:45% Cog:22→14)
  #2    HIGH         84.1  lib.rs::process_data      (Cov:12% Cog:18)
```

The arrow (→) indicates entropy dampening has reduced the complexity
score due to detected code patterns or repetition.
```

### Architecture Updates

No ARCHITECTURE.md changes needed (display-only change).

## Implementation Notes

### Color Coding Preservation

Maintain existing color scheme:
- Score: `theme.primary` (white/bright)
- Severity: Color-coded by level (red, yellow, green)
- Function location: `theme.secondary()` (cyan/blue)
- Metrics (Cov, Cog): `theme.muted` (gray/dim)
- Selection indicator: `theme.accent()` (bright)

### Alignment Strategy

**Current alignment**:
```
"Score:{:<7.1}"  // "Score:152.3  " (13 chars including label)
```

**New alignment**:
```
"{:<7.1}"        // "152.3  " (7 chars, saves 6)
```

Use saved space for:
- Longer function names (more visible)
- Entropy adjustment indicator (→)
- Better visual breathing room

### Alternative Formats Considered

1. **Show both cyclomatic and cognitive**: `Cyc:15 Cog:22→14`
   - **Rejected**: Too wide, clutters display
   - Cognitive is more meaningful, show cyclomatic in detail view

2. **Show percentage reduction**: `Cog:22 (-36%)`
   - **Rejected**: Less intuitive than showing actual values
   - Arrow format is clearer: `22→14`

3. **Use different symbol**: `Cog:22⇒14` or `Cog:22⟶14`
   - **Rejected**: May not render correctly in all terminals
   - Simple `→` is widely supported

4. **Show dampening factor**: `Cog:22 (×0.64)`
   - **Rejected**: Factor less intuitive than adjusted value
   - Show factor in detail view instead

## Migration and Compatibility

### Breaking Changes

**None**. This is purely a display change:
- No API changes
- No data structure changes (uses fields from spec 214)
- No configuration changes
- No behavioral changes

### Visual Changes

Users will see:
- Cleaner list view (less clutter)
- Clearer complexity metric (Cog instead of Comp)
- Visible entropy adjustments (when available)

**Migration**: Automatic on next run.

### Backward Compatibility

- Works with or without spec 214 implemented
- Gracefully degrades if entropy data unavailable
- No user action required

## Success Metrics

1. **Clarity**:
   - User feedback confirms clearer understanding of complexity
   - Less confusion about "Comp" metric
   - Entropy adjustments are visible and understandable

2. **Usability**:
   - More function name visible due to space savings
   - Easier to scan list for high-complexity items
   - Visual clutter reduced

3. **Correctness**:
   - All complexity values match detail view
   - Entropy adjustments match calculated values
   - No rendering errors or alignment issues

4. **Performance**:
   - No measurable rendering slowdown
   - Smooth scrolling maintained
   - Terminal compatibility preserved

## Future Enhancements (Out of Scope)

1. **Configurable display format** - Let users choose what metrics to show
2. **Color-coded complexity** - Red for high, green for low
3. **Hover tooltips** - Explain metrics on hover (if terminal supports)
4. **Column customization** - Reorder or hide columns
5. **Compact mode** - Ultra-compact format for narrow terminals

These are noted for future specifications but not included in this scope.
