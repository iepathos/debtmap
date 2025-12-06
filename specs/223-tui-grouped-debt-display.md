---
number: 223
title: TUI Grouped Debt Display with Multi-Type Detail View
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-01-09
---

# Specification 223: TUI Grouped Debt Display with Multi-Type Detail View

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Currently, when a function exhibits multiple debt patterns (e.g., high complexity, deep nesting, and excessive length), debtmap generates a separate `UnifiedDebtItem` for each issue. This results in visual redundancy in the TUI list view where the same function appears multiple times:

```
#1    CRITICAL   75.0   calculate_total::process.rs  (Cov:0% Cog:22)
#2    HIGH       60.0   calculate_total::process.rs  (Cov:0% Cog:18)
#3    MEDIUM     45.0   calculate_total::process.rs  (Cov:0% Cog:12)
```

This creates several problems:

1. **Visual Clutter**: Same location repeated, making list harder to scan
2. **Lost Context**: Not immediately obvious these are compound issues in one function
3. **Prioritization Issues**: User sees 3 medium-severity items instead of 1 critical compound issue
4. **Cognitive Load**: Users must mentally group items by location
5. **Incomplete Detail View**: When drilling down into detail view, only one debt type is shown, missing the full picture

The current architecture already detects all debt types correctly (`src/pipeline/stages/debt.rs:90-100`) - the issue is purely in presentation.

## Objective

Implement location-based grouping in the TUI to display functions with multiple debt types as single entries with combined scores and badges, while enhancing the detail view to show all debt types present at a location. This maintains the clean, minimal aesthetic defined in DESIGN.md while reducing cognitive load and improving information hierarchy.

## Requirements

### Functional Requirements

#### List View Grouping

- **Group debt items by location** (file, function, line) during rendering
- **Display grouped items** with:
  - Combined score (sum of all debt type scores)
  - Badge showing count when multiple issues exist (e.g., `[3 issues]`)
  - All relevant metrics visible (coverage, complexity, nesting, length)
  - Extra vertical spacing between groups (1 blank line)
- **Badge formatting**:
  - Show `[N issues]` in DarkGray (muted color per DESIGN.md)
  - Only show badge when N > 1
  - Position badge at end of location line
- **Metric display**:
  - First line: severity, combined score, location, badge
  - Second line (indented 2 spaces): all relevant metrics
  - Show Cog, Nest, Len when present across any debt type

#### Detail View Enhancement

- **Show all debt types** for the selected location in detail view
- **Display format**:
  - "DEBT TYPES" section showing all issues at this location
  - Each debt type listed with:
    - Type name (Complexity, Deep Nesting, Long Function, etc.)
    - Individual score contribution
    - Relevant metrics for that type
  - Use clean label-value pairs per DESIGN.md
- **Combined information**:
  - Total combined score shown in overview
  - Individual scores for each debt type
  - All unique recommendations aggregated

#### Grouping Toggle

- **Keyboard shortcut** 'g' to toggle grouping on/off
- **Default state**: Grouping enabled
- **Toggle behavior**:
  - Preserves current filters and sort order
  - Maintains scroll position where possible
  - Updates footer to show "Grouping: ON/OFF"
- **Header display**:
  - Show both counts: "8 locations (12 issues)" when grouped
  - Show only issue count when ungrouped: "12 items"

### Non-Functional Requirements

- **Performance**: Grouping adds <10ms overhead to rendering
- **Memory**: Grouping uses temporary HashMap, cleaned up per frame
- **Compatibility**: No changes to underlying data model (`UnifiedDebtItem`)
- **Design Consistency**: Follows DESIGN.md principles:
  - Progressive disclosure (summary first, detail on demand)
  - Space as design element (extra spacing between groups)
  - Single point of focus (selected group stands out)
  - No decoration (badge serves information purpose)
  - Minimal color palette (DarkGray for badge)

## Acceptance Criteria

### List View

- [ ] Functions with multiple debt types appear as single entry with combined score
- [ ] Badge `[N issues]` shown when N > 1, styled in DarkGray
- [ ] Combined score equals sum of individual debt type scores
- [ ] All relevant metrics (Cog, Nest, Len) shown across debt types
- [ ] Extra blank line between grouped items for visual separation
- [ ] Grouped items maintain proper alignment with ungrouped items
- [ ] Selection indicator (▸) works correctly with grouped items
- [ ] Sorting works with combined scores (highest score groups first)

### Detail View

- [ ] Detail view shows "DEBT TYPES" section for grouped locations
- [ ] All debt types at location listed with individual scores
- [ ] Each debt type shows relevant metrics
- [ ] Combined score shown in overview section
- [ ] All unique recommendations shown from all debt types
- [ ] Single debt type locations show existing detail format
- [ ] Navigation between list and detail preserves grouping state

### Grouping Toggle

- [ ] 'g' key toggles grouping on/off
- [ ] Grouping state persists during session
- [ ] Header shows correct counts based on grouping state
- [ ] Footer shows "Grouping: ON/OFF" indicator
- [ ] Toggle preserves filters, sort order, and search results
- [ ] Toggle transitions smoothly without visual artifacts

### Performance & Quality

- [ ] Grouping logic completes in <10ms for 1000 items
- [ ] No memory leaks from grouping HashMap
- [ ] TUI maintains 60 FPS rendering with grouping enabled
- [ ] All existing TUI tests pass with grouping logic
- [ ] Works correctly with filters (filters apply before grouping)
- [ ] Works correctly with search (search applies before grouping)

## Technical Details

### Implementation Approach

#### 1. Create Grouping Module (`src/tui/results/grouping.rs`)

Pure functional module for grouping logic:

```rust
use crate::priority::UnifiedDebtItem;
use std::collections::HashMap;
use std::path::PathBuf;

/// A group of debt items at the same location
#[derive(Debug, Clone)]
pub struct LocationGroup<'a> {
    /// Representative location (from first item)
    pub location: &'a Location,
    /// All debt items at this location
    pub items: Vec<&'a UnifiedDebtItem>,
    /// Combined score (sum of all item scores)
    pub combined_score: f64,
    /// Highest severity color among items
    pub max_severity: &'static str,
}

/// Group debt items by (file, function, line) location
pub fn group_by_location<'a>(
    items: impl Iterator<Item = &'a UnifiedDebtItem>
) -> Vec<LocationGroup<'a>> {
    let mut groups: HashMap<(&PathBuf, &str, usize), Vec<&UnifiedDebtItem>> =
        HashMap::new();

    for item in items {
        let key = (
            &item.location.file,
            item.location.function.as_str(),
            item.location.line,
        );
        groups.entry(key).or_default().push(item);
    }

    groups.into_iter()
        .map(|(_, items)| {
            let combined_score = items.iter()
                .map(|i| i.unified_score.final_score)
                .sum();

            let max_severity = items.iter()
                .map(|i| calculate_severity(i.unified_score.final_score))
                .max()
                .unwrap_or("LOW");

            LocationGroup {
                location: &items[0].location,
                items,
                combined_score,
                max_severity,
            }
        })
        .collect()
}

/// Extract all unique metrics across items in group
pub fn aggregate_metrics(group: &LocationGroup) -> AggregatedMetrics {
    let max_cog = group.items.iter()
        .map(|i| i.cognitive_complexity)
        .max()
        .unwrap_or(0);

    let max_nest = group.items.iter()
        .map(|i| i.nesting_depth)
        .max()
        .unwrap_or(0);

    let max_len = group.items.iter()
        .map(|i| i.function_length)
        .max()
        .unwrap_or(0);

    // Coverage same across all items at location
    let coverage = group.items[0].transitive_coverage.as_ref();

    AggregatedMetrics {
        cognitive_complexity: max_cog,
        nesting_depth: max_nest,
        function_length: max_len,
        coverage,
    }
}

#[derive(Debug)]
pub struct AggregatedMetrics<'a> {
    pub cognitive_complexity: u32,
    pub nesting_depth: u32,
    pub function_length: usize,
    pub coverage: Option<&'a TransitiveCoverage>,
}
```

#### 2. Update ResultsApp State (`src/tui/results/app.rs`)

Add grouping toggle state:

```rust
pub struct ResultsApp {
    // ... existing fields

    /// Whether to group items by location
    show_grouped: bool,
}

impl ResultsApp {
    pub fn new(analysis: UnifiedAnalysis) -> Self {
        Self {
            // ... existing initialization
            show_grouped: true,  // Default: ON
        }
    }

    /// Toggle grouping on/off
    pub fn toggle_grouping(&mut self) {
        self.show_grouped = !self.show_grouped;
        // Note: No need to re-apply filters/sort since
        // grouping happens at render time
    }

    /// Get grouping state
    pub fn is_grouped(&self) -> bool {
        self.show_grouped
    }

    /// Get count info for header display
    pub fn count_display(&self) -> String {
        if self.show_grouped {
            let groups = grouping::group_by_location(self.filtered_items());
            let issue_count = self.filtered_indices.len();
            format!("{} locations ({} issues)", groups.len(), issue_count)
        } else {
            format!("{} items", self.filtered_indices.len())
        }
    }
}
```

#### 3. Modify List View Rendering (`src/tui/results/list_view.rs`)

Update `render_list` function to support grouped display:

```rust
fn render_list(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let items: Vec<ListItem> = if app.is_grouped() {
        render_grouped_list(app, area, theme)
    } else {
        render_ungrouped_list(app, area, theme)
    };

    // ... rest of rendering
}

fn render_grouped_list(
    app: &ResultsApp,
    area: Rect,
    theme: &Theme,
) -> Vec<ListItem> {
    use super::grouping;

    let groups = grouping::group_by_location(app.filtered_items());

    let mut list_items = Vec::new();
    let mut display_index = 0;

    for group in groups.iter().skip(app.scroll_offset()) {
        if list_items.len() >= area.height as usize {
            break;
        }

        let is_selected = display_index == app.selected_index();
        list_items.push(format_grouped_item(
            group,
            display_index,
            is_selected,
            theme
        ));

        // Add spacing between groups (blank line)
        if list_items.len() < area.height as usize {
            list_items.push(ListItem::new(""));
        }

        display_index += 1;
    }

    list_items
}

fn format_grouped_item(
    group: &grouping::LocationGroup,
    index: usize,
    is_selected: bool,
    theme: &Theme,
) -> ListItem<'static> {
    let indicator = if is_selected { "▸ " } else { "  " };
    let severity_color = severity_color(group.max_severity);

    let file_name = group.location.file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Badge for multiple issues
    let badge = if group.items.len() > 1 {
        format!("[{} issues]", group.items.len())
    } else {
        String::new()
    };

    // First line: indicator, rank, severity, score, location, badge
    let mut line1 = vec![
        Span::styled(indicator, Style::default().fg(theme.accent())),
        Span::styled(
            format!("#{:<4}", index + 1),
            Style::default().fg(theme.muted),
        ),
        Span::styled(
            format!("{:<10}", group.max_severity),
            Style::default().fg(severity_color),
        ),
        Span::styled(
            format!("{:<7.1}", group.combined_score),
            Style::default().fg(theme.primary),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{}::{}", file_name, group.location.function),
            Style::default().fg(theme.secondary()),
        ),
    ];

    if !badge.is_empty() {
        line1.push(Span::raw("  "));
        line1.push(Span::styled(
            badge,
            Style::default().fg(theme.muted),
        ));
    }

    // Second line: aggregated metrics
    let metrics = grouping::aggregate_metrics(group);
    let coverage_str = metrics.coverage
        .map(|c| format!("{:.0}%", c.direct))
        .unwrap_or_else(|| "N/A".to_string());

    let mut metric_parts = vec![format!("Cov:{}", coverage_str)];

    if metrics.cognitive_complexity > 0 {
        metric_parts.push(format!("Cog:{}", metrics.cognitive_complexity));
    }
    if metrics.nesting_depth > 0 {
        metric_parts.push(format!("Nest:{}", metrics.nesting_depth));
    }
    if metrics.function_length > 0 {
        metric_parts.push(format!("Len:{}", metrics.function_length));
    }

    let line2 = vec![
        Span::raw("  "),  // Indent
        Span::styled(
            format!("({})", metric_parts.join(" ")),
            Style::default().fg(theme.muted),
        ),
    ];

    let style = if is_selected {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default()
    };

    ListItem::new(vec![Line::from(line1), Line::from(line2)]).style(style)
}
```

#### 4. Update Detail View (`src/tui/results/detail_pages/overview.rs`)

Add multi-type debt display to overview page:

```rust
pub fn render(/* ... */) -> Vec<Line<'static>> {
    // ... existing overview content

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "DEBT TYPES",
        Style::default().fg(theme.muted),
    )));
    lines.push(Line::from(""));

    // Get all items at this location
    let location_items = get_items_at_location(app, selected_item);

    if location_items.len() > 1 {
        // Multiple debt types - show breakdown
        lines.push(Line::from(vec![
            Span::raw("  Total Combined Score: "),
            Span::styled(
                format!("{:.1}", total_score),
                Style::default().fg(theme.primary),
            ),
        ]));
        lines.push(Line::from(""));

        for (idx, item) in location_items.iter().enumerate() {
            let debt_name = format_debt_type_name(&item.debt_type);
            let score = item.unified_score.final_score;

            lines.push(Line::from(vec![
                Span::raw(format!("  {}. ", idx + 1)),
                Span::styled(
                    format!("{:<20}", debt_name),
                    Style::default().fg(theme.secondary()),
                ),
                Span::styled(
                    format!("Score: {:.1}", score),
                    Style::default().fg(theme.primary),
                ),
            ]));

            // Show relevant metrics for this debt type
            let metric_line = format_debt_type_metrics(item);
            if !metric_line.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("     "),
                    Span::styled(metric_line, Style::default().fg(theme.muted)),
                ]));
            }
        }
    } else {
        // Single debt type - show as before
        let debt_name = format_debt_type_name(&selected_item.debt_type);
        lines.push(Line::from(vec![
            Span::raw("  Type: "),
            Span::styled(debt_name, Style::default().fg(theme.primary)),
        ]));
    }

    // ... rest of overview
}

/// Get all debt items at the same location as the selected item
fn get_items_at_location<'a>(
    app: &'a ResultsApp,
    selected: &UnifiedDebtItem,
) -> Vec<&'a UnifiedDebtItem> {
    app.analysis()
        .items
        .iter()
        .filter(|item|
            item.location.file == selected.location.file &&
            item.location.function == selected.location.function &&
            item.location.line == selected.location.line
        )
        .collect()
}

fn format_debt_type_name(debt_type: &DebtType) -> String {
    match debt_type {
        DebtType::ComplexityHotspot { .. } => "High Complexity",
        DebtType::DeepNesting { .. } => "Deep Nesting",
        DebtType::LongFunction { .. } => "Long Function",
        DebtType::HighParameterCount { .. } => "Too Many Parameters",
        // ... other types
    }
}

fn format_debt_type_metrics(item: &UnifiedDebtItem) -> String {
    match &item.debt_type {
        DebtType::ComplexityHotspot { cognitive, cyclomatic, .. } => {
            format!("Cognitive: {}, Cyclomatic: {}", cognitive, cyclomatic)
        }
        DebtType::DeepNesting { depth } => {
            format!("Nesting depth: {}", depth)
        }
        DebtType::LongFunction { lines } => {
            format!("Function length: {} lines", lines)
        }
        // ... other types
    }
}
```

#### 5. Update Navigation (`src/tui/results/navigation.rs`)

Add 'g' key handler:

```rust
pub fn handle_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    match app.view_mode() {
        ViewMode::List => match key.code {
            // ... existing handlers

            KeyCode::Char('g') => {
                app.toggle_grouping();
                Ok(false)
            }

            // ... rest of handlers
        }
        // ... other view modes
    }
}
```

#### 6. Update Header Display (`src/tui/results/list_view.rs`)

Modify `render_header` to show grouping state:

```rust
fn render_header(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    let analysis = app.analysis();
    let count_display = app.count_display();  // "8 locations (12 issues)" or "12 items"

    let header_text = vec![
        Line::from(vec![
            Span::styled("Debtmap Results", Style::default().fg(theme.accent())),
            Span::raw("  "),
            Span::styled(
                format!("Total: {}", count_display),
                Style::default().fg(theme.primary),
            ),
            // ... debt score, density
        ]),
        Line::from(vec![
            Span::styled(
                format!("Sort: {}", app.sort_by().display_name()),
                Style::default().fg(theme.muted),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Filters: {}", app.filters().len()),
                Style::default().fg(theme.muted),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Grouping: {}", if app.is_grouped() { "ON" } else { "OFF" }),
                Style::default().fg(theme.muted),
            ),
        ]),
    ];

    // ... rest of header
}
```

#### 7. Update Footer (`src/tui/results/list_view.rs`)

Add 'g' shortcut to footer:

```rust
fn render_footer(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    // ... existing position text

    let footer_text = Line::from(vec![
        Span::styled(position_text, Style::default().fg(theme.muted)),
        Span::raw("  |  "),
        Span::styled("↑↓/jk", Style::default().fg(theme.accent())),
        Span::raw(":Nav  "),
        Span::styled("g", Style::default().fg(theme.accent())),
        Span::raw(":Group  "),
        Span::styled("/", Style::default().fg(theme.accent())),
        Span::raw(":Search  "),
        // ... rest of shortcuts
    ]);

    // ... rest of footer
}
```

### Architecture Changes

**No changes to core data model** - `UnifiedDebtItem` remains unchanged. Grouping is purely a view-layer concern, implemented through:

1. Pure functional grouping module (`grouping.rs`)
2. Render-time grouping in list view
3. Location-aware detail view enhancement
4. Stateful toggle in `ResultsApp`

This maintains separation of concerns and allows easy future extension (e.g., grouping by file, module, or debt type).

### Data Structures

```rust
// New in src/tui/results/grouping.rs
pub struct LocationGroup<'a> {
    pub location: &'a Location,
    pub items: Vec<&'a UnifiedDebtItem>,
    pub combined_score: f64,
    pub max_severity: &'static str,
}

pub struct AggregatedMetrics<'a> {
    pub cognitive_complexity: u32,
    pub nesting_depth: u32,
    pub function_length: usize,
    pub coverage: Option<&'a TransitiveCoverage>,
}

// Modified in src/tui/results/app.rs
pub struct ResultsApp {
    // ... existing fields
    show_grouped: bool,  // NEW
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/tui/results/app.rs` - Add grouping state
  - `src/tui/results/list_view.rs` - Implement grouped rendering
  - `src/tui/results/detail_pages/overview.rs` - Multi-type display
  - `src/tui/results/navigation.rs` - 'g' key handler
- **External Dependencies**: None (uses existing ratatui)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_by_location_single_item() {
        let items = vec![create_test_item("file.rs", "func", 10)];
        let groups = group_by_location(items.iter());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].items.len(), 1);
    }

    #[test]
    fn test_group_by_location_multiple_types() {
        let items = vec![
            create_test_item("file.rs", "func", 10),
            create_test_item("file.rs", "func", 10),
            create_test_item("file.rs", "func", 10),
        ];
        let groups = group_by_location(items.iter());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].items.len(), 3);
    }

    #[test]
    fn test_combined_score_calculation() {
        let items = vec![
            create_test_item_with_score(75.0),
            create_test_item_with_score(60.0),
            create_test_item_with_score(45.0),
        ];
        let groups = group_by_location(items.iter());
        assert_eq!(groups[0].combined_score, 180.0);
    }

    #[test]
    fn test_aggregate_metrics_max_values() {
        let items = vec![
            create_item_with_metrics(22, 5, 87),
            create_item_with_metrics(18, 4, 65),
            create_item_with_metrics(14, 3, 50),
        ];
        let groups = group_by_location(items.iter());
        let metrics = aggregate_metrics(&groups[0]);

        assert_eq!(metrics.cognitive_complexity, 22);
        assert_eq!(metrics.nesting_depth, 5);
        assert_eq!(metrics.function_length, 87);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_grouped_display_integration() {
    // Create analysis with multiple debt types at same location
    let analysis = create_test_analysis_with_compound_debt();
    let app = ResultsApp::new(analysis);

    assert!(app.is_grouped());

    let count = app.count_display();
    assert!(count.contains("locations"));
    assert!(count.contains("issues"));
}

#[test]
fn test_grouping_toggle_preserves_state() {
    let analysis = create_test_analysis();
    let mut app = ResultsApp::new(analysis);

    // Apply some filters and sorting
    app.add_filter(Filter::Critical);
    app.set_sort_by(SortCriteria::Coverage);
    app.set_selected_index(5);

    // Toggle grouping
    app.toggle_grouping();

    // Verify state preserved
    assert_eq!(app.filters().len(), 1);
    assert_eq!(app.sort_by(), SortCriteria::Coverage);
    assert_eq!(app.selected_index(), 5);
}
```

### Performance Tests

```rust
#[test]
fn test_grouping_performance() {
    let items = create_large_item_set(1000);

    let start = Instant::now();
    let groups = group_by_location(items.iter());
    let duration = start.elapsed();

    assert!(duration.as_millis() < 10, "Grouping took {}ms", duration.as_millis());
}
```

### User Acceptance

Manual testing checklist:
- [ ] Multiple debt types appear grouped with correct combined score
- [ ] Badge shows correct issue count
- [ ] All metrics visible across debt types
- [ ] Spacing between groups looks clean
- [ ] Selection/navigation works smoothly
- [ ] Detail view shows all debt types
- [ ] 'g' toggle works immediately
- [ ] Grouping state shown in header/footer
- [ ] Maintains 60 FPS during navigation

## Documentation Requirements

### Code Documentation

- Document grouping module with examples
- Add inline comments for complex grouping logic
- Document `LocationGroup` and `AggregatedMetrics` structs
- Explain grouping toggle behavior in `ResultsApp`

### User Documentation

Update user guide to explain:
- Grouped display feature
- How to interpret badges and combined scores
- How to toggle grouping with 'g' key
- What detail view shows for compound debt

### DESIGN.md Updates

Add section on grouped display:
- Rationale for grouping approach
- Design principles applied (progressive disclosure)
- Visual examples of grouped vs ungrouped
- Badge styling and placement guidelines

## Implementation Notes

### Grouping Performance

- Grouping creates temporary HashMap each frame (~1000 items = <10ms)
- HashMap cleared automatically (no manual cleanup needed)
- Could cache grouped results if performance issues arise
- Current approach prioritizes simplicity and correctness

### Sorting with Groups

When grouped:
- Groups sorted by combined score
- Within-group order doesn't matter (all shown together)

When ungrouped:
- Original per-item sorting applies

### Filtering with Groups

Filters apply **before** grouping:
1. Apply all filters to get filtered_indices
2. Get filtered items
3. Group filtered items by location
4. Display groups

This ensures filtering correctness (filter on individual items, then group).

### Edge Cases

- **Single debt type**: Show without badge, normal display
- **All items filtered out**: Show empty state as usual
- **Zero items**: No crash, empty groups Vec
- **Very long function names**: Existing truncation applies
- **Very long metric lists**: Show most important metrics only

### Future Enhancements

Potential future additions (not in this spec):
- Group by file (show all functions in file)
- Group by module (show all files in module)
- Expand/collapse groups (tree view)
- Custom grouping criteria
- Group-level filtering

## Migration and Compatibility

### Breaking Changes

None - purely additive feature. Existing functionality unchanged.

### Backward Compatibility

- Grouping can be toggled off to see original view
- All existing filters, sorts, searches work unchanged
- No impact on non-TUI outputs (JSON, markdown, etc.)
- No changes to saved analysis results

### Migration Path

No migration needed - feature works immediately on existing analysis results.

### Deprecations

None - no existing functionality removed or deprecated.
