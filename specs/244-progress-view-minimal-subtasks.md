---
number: 244
title: Minimal Sub-task Display for Progress View
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-12-06
---

# Specification 244: Minimal Sub-task Display for Progress View

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The current progress view TUI uses dotted leaders to connect sub-task names with their status/metrics, creating visual noise that violates the "futuristic zen minimalism" design philosophy established in DESIGN.md:

```
    Analyzing functions ▸▸▸▸▸▸▸▸▸▸············ 125/450
    Extracting metrics ······························· done
    Pending sub-task ·································
```

**Problems:**
- **Violates "Clarity Through Restraint"** - Dots are decorative, not informational
- **Contradicts "Space as a Design Element"** - Fills space instead of using it for visual hierarchy
- **Creates visual clutter** - Dots compete with actual progress information (animated arrows, counts)
- **Redundant indicators** - Progress bar + numeric count + dots is excessive
- **Inconsistent with detail view** - Detail view already moved to aligned columns without dots

The dotted leaders serve no functional purpose:
- Progress is already conveyed by numeric count (125/450)
- Activity is shown by animated arrows (▸▸▸)
- Status is clear from text ("done") or presence of count

This creates visual fatigue during long-running analyses and detracts from the calm, focused aesthetic intended by the design.

## Objective

Simplify sub-task display to use **right-aligned metrics with pure whitespace** as the visual connector, eliminating all decorative elements while preserving clarity and readability. This aligns the progress view with the zen minimal aesthetic applied to the detail view.

## Requirements

### Functional Requirements

1. **Right-Aligned Status Display**
   - Active sub-tasks show numeric progress right-aligned: `"125/450"`
   - Completed sub-tasks show right-aligned text: `"done"`
   - Pending sub-tasks show name only (no trailing indicators)

2. **Whitespace-Based Layout**
   - Use calculated spacing to right-align metrics
   - No decorative characters (dots, lines, etc.) between name and metric
   - Maintain 4-space indentation for sub-tasks
   - Preserve clean left edge (names) and right edge (metrics/status)

3. **Remove Visual Progress Bar**
   - Eliminate animated arrow progress bar (▸▸▸▸)
   - Remove light shade empty indicators (░░░░)
   - Rely solely on numeric progress count for precision
   - Reduce visual complexity and information density

4. **Preserve Animation for Stage Icons**
   - Keep animated stage icons (✓, ▸, ·) at the main stage level
   - Remove sub-task level animations to reduce visual noise
   - Focus attention on high-level pipeline progression

### Non-Functional Requirements

1. **Visual Hierarchy**
   - Sub-task names clearly visible on left
   - Metrics/status clearly visible on right
   - Whitespace creates natural visual grouping
   - Easy to scan both names and status vertically

2. **Consistency**
   - Match detail view design language (aligned columns, no decoration)
   - Maintain same spacing conventions (4-space indent)
   - Use consistent color scheme (theme.metric_style() for counts, theme.completed_style() for "done")

3. **Responsive Behavior**
   - Handle varying terminal widths gracefully
   - Maintain alignment even with long sub-task names
   - Never truncate or wrap metrics (they're short: "done" or "125/450")

4. **Performance**
   - No performance regression from current implementation
   - Simpler rendering logic (fewer spans, no calculations for dots/bars)

## Acceptance Criteria

- [ ] Active sub-tasks display name on left, numeric count right-aligned with whitespace gap
- [ ] Completed sub-tasks display name on left, "done" right-aligned with whitespace gap
- [ ] Pending sub-tasks display name only (no trailing dots or indicators)
- [ ] No dotted leaders or decorative characters between name and metric
- [ ] No visual progress bars (animated arrows or empty space indicators)
- [ ] Right edge of metrics aligns consistently across all sub-tasks
- [ ] 4-space indentation maintained for all sub-tasks
- [ ] Metrics use theme.metric_style() for counts, theme.completed_style() for "done"
- [ ] Layout adapts to terminal width without breaking alignment
- [ ] Visual design matches detail view minimalist aesthetic
- [ ] Code is simpler than previous implementation (fewer spans, less calculation)

## Technical Details

### Implementation Approach

Modify `render_subtask_line()` function in `src/tui/renderer.rs` to:

1. Calculate spacing needed to right-align metrics
2. Build line with name + whitespace + metric (no intermediate decorations)
3. Handle three status cases simply: completed, active, pending
4. Remove all progress bar and dot rendering logic

### Current Implementation (Lines 193-247 in renderer.rs)

```rust
fn render_subtask_line(
    subtask: &SubTask,
    frame: u64,
    theme: &Theme,
    width: u16,
) -> Line<'static> {
    let indent = "    ";
    let name_with_indent = format!("{}{}", indent, subtask.name);

    match subtask.status {
        StageStatus::Completed => {
            // Dotted leader to "done" - REMOVE
            let dots_needed = width.saturating_sub((name_with_indent.len() + 10) as u16) as usize;
            Line::from(vec![
                Span::raw(name_with_indent),
                Span::raw(" "),
                Span::styled("·".repeat(dots_needed), theme.dotted_leader_style()),
                Span::styled(" done", theme.completed_style()),
            ])
        }
        StageStatus::Active => {
            if let Some((current, total)) = subtask.progress {
                // Progress bar with dots - REMOVE
                let progress = current as f64 / total as f64;
                let bar_width: usize = 30;
                let filled = (progress * bar_width as f64) as usize;
                let empty = bar_width.saturating_sub(filled);
                let arrow = match (frame / 3) % 3 {
                    0 => "▸", 1 => "▹", _ => "▸",
                };
                Line::from(vec![
                    Span::raw(name_with_indent),
                    Span::raw(" "),
                    Span::styled(arrow.repeat(filled), theme.arrow_style()),
                    Span::styled("·".repeat(empty), theme.dotted_leader_style()),
                    Span::raw(" "),
                    Span::styled(format!("{}/{}", current, total), theme.metric_style()),
                ])
            } else {
                Line::from(Span::raw(name_with_indent))
            }
        }
        StageStatus::Pending => {
            // Trailing dots - REMOVE
            let dots_needed = width.saturating_sub((name_with_indent.len() + 4) as u16) as usize;
            Line::from(vec![
                Span::raw(name_with_indent),
                Span::styled("·".repeat(dots_needed), theme.dotted_leader_style()),
            ])
        }
    }
}
```

### Proposed Implementation

```rust
fn render_subtask_line(
    subtask: &SubTask,
    _frame: u64,  // No longer used (no animations)
    theme: &Theme,
    width: u16,
) -> Line<'static> {
    const INDENT: &str = "    ";
    let name_with_indent = format!("{}{}", INDENT, subtask.name);

    match subtask.status {
        StageStatus::Completed => {
            // Right-align "done" with whitespace
            let metric = "done";
            let spacing_needed = width
                .saturating_sub((name_with_indent.len() + metric.len()) as u16)
                as usize;

            Line::from(vec![
                Span::raw(name_with_indent),
                Span::raw(" ".repeat(spacing_needed)),
                Span::styled(metric, theme.completed_style()),
            ])
        }
        StageStatus::Active => {
            if let Some((current, total)) = subtask.progress {
                // Right-align numeric count with whitespace
                let metric = format!("{}/{}", current, total);
                let spacing_needed = width
                    .saturating_sub((name_with_indent.len() + metric.len()) as u16)
                    as usize;

                Line::from(vec![
                    Span::raw(name_with_indent),
                    Span::raw(" ".repeat(spacing_needed)),
                    Span::styled(metric, theme.metric_style()),
                ])
            } else {
                // No progress data - show name only
                Line::from(Span::raw(name_with_indent))
            }
        }
        StageStatus::Pending => {
            // Show name only - no trailing indicators
            Line::from(Span::raw(name_with_indent))
        }
    }
}
```

### Architecture Changes

**Modified Components:**
- `src/tui/renderer.rs::render_subtask_line()` - Simplified rendering logic
- `src/tui/theme.rs` - Can remove `dotted_leader_style()` and `arrow_style()` if unused elsewhere

**Removed Complexity:**
- Progress bar width calculation
- Animated arrow frame logic
- Dot repetition for empty space
- Multiple span construction for bars

**Simplified Logic:**
- 3 clear cases: completed, active with progress, pending
- Simple spacing calculation for right alignment
- Fewer spans per line (2-3 instead of 4-5)

### Visual Examples

**Before (Current):**
```
    Analyzing functions ▸▸▸▸▸▸▸▸▸▸············ 125/450
    Extracting metrics ······························· done
    Computing scores ▸▸▸▸▸▸············ 45/120
    Pending sub-task ·································
```

**After (Proposed):**
```
    Analyzing functions                     125/450
    Extracting metrics                         done
    Computing scores                         45/120
    Pending sub-task
```

**Visual Impact:**
- Cleaner, calmer appearance
- Clear left (names) and right (metrics) edges
- Whitespace creates natural visual grouping
- Easy to scan either column independently
- Matches detail view design language

## Dependencies

**Prerequisites:** None

**Affected Components:**
- `src/tui/renderer.rs` - Main rendering logic for sub-tasks
- `src/tui/theme.rs` - May allow cleanup of unused style methods

**External Dependencies:** None

## Testing Strategy

### Manual Testing

1. **Visual Verification**
   - Run debtmap analysis in TUI mode
   - Verify sub-task display during active analysis
   - Check alignment across different terminal widths (80, 120, 160 columns)
   - Confirm "done" and numeric counts align properly
   - Validate pending tasks show name only

2. **Edge Cases**
   - Very long sub-task names (ensure no overflow)
   - Very short terminal widths (verify graceful degradation)
   - No sub-tasks (ensure main stages still work)
   - Single sub-task vs multiple sub-tasks
   - Sub-tasks with and without progress data

3. **Color and Style**
   - Verify metrics use correct theme styles
   - Check "done" appears in completed style (green)
   - Validate numeric counts use metric style
   - Ensure sub-task names use default text color

### Regression Testing

1. **Main Stage Display**
   - Verify stage icons still animate (✓, ▸, ·)
   - Confirm stage names and metrics still display
   - Check overall progress bar unaffected
   - Validate header and footer unchanged

2. **Layout Modes**
   - Test full view (with sub-tasks)
   - Test compact view (without sub-tasks)
   - Test minimal view (progress bar only)
   - Verify responsive breakpoints still work

3. **Performance**
   - Confirm no performance regression
   - Verify 60 FPS rendering maintained
   - Check memory usage unchanged

## Documentation Requirements

### Code Documentation

- Update `render_subtask_line()` function documentation
- Add comment explaining right-alignment calculation
- Document the three status display modes
- Note simplification from previous implementation

### User Documentation

- Update DESIGN.md visual examples to show new sub-task format
- Ensure progress view examples match implementation
- Add rationale for removing progress bars (numeric count is sufficient)

### Architecture Updates

- Document decision to prioritize simplicity over visual richness
- Explain alignment with detail view design principles
- Note performance benefit of simpler rendering

## Implementation Notes

### Design Philosophy Alignment

This change directly implements the core design principles:

1. **Clarity Through Restraint**
   - Every character serves a purpose (name or metric)
   - No decorative elements

2. **Space as a Design Element**
   - Whitespace creates the visual connection
   - Breathing room prevents overwhelm

3. **Information Hierarchy**
   - Names on left = what's happening
   - Metrics on right = current status
   - Clear visual scanning paths

4. **Never obstruct information**
   - Progress count is more precise than visual bar
   - Simpler display reduces cognitive load

### Alternative Considered and Rejected

**Option 5: Minimal Progress Bar (Filled Only)**
- Keep animated arrows but remove empty space dots
- More complex implementation
- Progress bar redundant with numeric count
- Adds visual weight without adding information

**Decision:** Option 1 chosen for maximum simplicity and clarity.

### Code Simplification Benefits

- **Fewer calculations** - No progress bar width or fill calculations
- **Fewer spans** - 2-3 spans instead of 4-5
- **No animation logic** - No frame-based arrow cycling for sub-tasks
- **Easier maintenance** - Simpler code is easier to understand and modify
- **Better performance** - Less string manipulation and span construction

## Migration and Compatibility

### Breaking Changes

**Visual Changes:**
- Sub-task progress bars removed
- Dotted leaders removed
- Animated arrows at sub-task level removed

**Note:** These are visual-only changes. No API or data format changes.

### User Impact

**Positive:**
- Calmer, less cluttered display
- Easier to scan progress during long analyses
- Consistent with detail view aesthetic
- More information density (less vertical space per sub-task)

**Potential Concerns:**
- Users accustomed to visual progress bars may initially miss them
- Numeric-only progress may feel less "visual" to some users

**Mitigation:**
- Numeric progress is actually more precise (125/450 vs approximate bar)
- Main stage progress bar still provides high-level visual feedback
- Change aligns with established design principles in DESIGN.md

### Rollback Plan

If user feedback is strongly negative:
1. Revert commit (changes isolated to `render_subtask_line()`)
2. Consider hybrid Option 5 (minimal progress bar without dots)
3. Gather specific feedback on what visual indicator users value

## Success Metrics

### Qualitative

- Visual consistency with detail view design
- Reduced visual clutter and cognitive load
- Improved scannability during analysis runs
- Alignment with zen minimal aesthetic

### Quantitative

- Code complexity reduction (fewer lines in `render_subtask_line()`)
- Render performance maintained at 60 FPS
- No increase in memory usage
- Fewer spans created per sub-task line

### User Feedback

- Gather feedback from early adopters
- Monitor for complaints about missing progress bars
- Assess whether numeric progress is sufficient
- Validate improved clarity and scannability

## Related Specifications

- Detail view label-value alignment changes (implemented, no spec)
- DESIGN.md futuristic zen minimalism principles (design doc)

## Future Enhancements

**If numeric progress proves insufficient:**
- Consider adding percentage in parentheses: `"(28%) 125/450"`
- Or adding ETA estimate: `"125/450 (2m remaining)"`
- Keep design minimal - add only essential information

**Potential follow-up:**
- Apply same minimal principles to main stage metrics
- Review other TUI components for decoration removal opportunities
- Create comprehensive TUI style guide based on implemented patterns
