---
number: 252
title: Output Format Unification
category: optimization
priority: high
status: draft
dependencies: [250, 251]
created: 2025-12-10
---

# Specification 252: Output Format Unification

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 250 (Unified View Data Model), Spec 251 (View Preparation Pipeline)

## Context

With the unified data model (Spec 250) and pure pipeline (Spec 251) in place, we need to update all output formats to consume `PreparedDebtView` instead of directly accessing `UnifiedAnalysis`.

**Current State** (inconsistent):

| Format | Entry Point | Data Source | Filtering | Issues |
|--------|-------------|-------------|-----------|--------|
| TUI | `ResultsApp::new()` | `analysis.items` | None | Missing file_items |
| Terminal | `format_priorities()` | `get_top_mixed_priorities()` | T4, score | Different from TUI |
| JSON | `convert_to_unified_format()` | `get_top_mixed_priorities()` | T4, score | Different from TUI |
| Markdown | `output_markdown()` | `apply_filters()` + `get_top_mixed_priorities()` | Double filtering | Different from all |

**Target State** (unified):

| Format | Entry Point | Data Source | Filtering |
|--------|-------------|-------------|-----------|
| TUI | `ResultsApp::new()` | `PreparedDebtView` | Via ViewConfig |
| Terminal | `format_terminal()` | `PreparedDebtView` | Via ViewConfig |
| JSON | `format_json()` | `PreparedDebtView` | Via ViewConfig |
| Markdown | `format_markdown()` | `PreparedDebtView` | Via ViewConfig |

All formats consume the same `PreparedDebtView`, ensuring consistent results.

## Objective

Update all output formatters to consume `PreparedDebtView`:

1. **TUI** - Use `PreparedDebtView` for both grouped and ungrouped display
2. **Terminal** - Render `PreparedDebtView` items
3. **JSON** - Serialize `PreparedDebtView` directly
4. **Markdown** - Format `PreparedDebtView` items

Each formatter becomes a **thin rendering layer** with no filtering or sorting logic.

## Requirements

### Functional Requirements

1. **TUI Update**
   - `ResultsApp::new(view: PreparedDebtView)` constructor
   - Use `view.groups` for grouped display
   - Use `view.items` for ungrouped display
   - Remove internal grouping logic (now in pipeline)
   - Remove direct `analysis.items` access

2. **Terminal Update**
   - `format_terminal(view: &PreparedDebtView, verbosity: u8) -> String`
   - Render `view.items` in order
   - Use `view.summary` for statistics
   - Remove calls to `get_top_mixed_priorities()`

3. **JSON Update**
   - `format_json(view: &PreparedDebtView) -> Value`
   - Serialize items, groups, summary
   - Remove `convert_to_unified_format()`
   - Direct serialization of `PreparedDebtView`

4. **Markdown Update**
   - `format_markdown(view: &PreparedDebtView, verbosity: u8) -> String`
   - Format `view.items` as markdown
   - Use `view.summary` for footer
   - Remove `apply_filters()` function

5. **Entry Point Update**
   - Single place where `prepare_view()` is called
   - Configuration read from CLI args/env at boundary
   - `ViewConfig` constructed at entry point
   - Pass `PreparedDebtView` to all formatters

6. **Backward Compatibility**
   - Same visual output for each format
   - Same CLI interface
   - Same JSON structure (or documented changes)

### Non-Functional Requirements

1. **Simplicity**
   - Each formatter under 200 lines
   - No business logic in formatters
   - Just rendering/serialization

2. **Consistency**
   - All formats show same items (given same config)
   - Same ordering across formats
   - Same statistics

3. **Testability**
   - Formatters testable with mock `PreparedDebtView`
   - No I/O in formatter core logic
   - Clear input/output contracts

## Acceptance Criteria

- [ ] TUI uses `PreparedDebtView` for all display
- [ ] Terminal formatter takes `PreparedDebtView`
- [ ] JSON formatter takes `PreparedDebtView`
- [ ] Markdown formatter takes `PreparedDebtView`
- [ ] Old `get_top_mixed_priorities()` calls removed from formatters
- [ ] `apply_filters()` in markdown.rs removed
- [ ] Single call site for `prepare_view()` in main flow
- [ ] All formatters under 200 lines
- [ ] Visual output unchanged (verified by snapshot tests)
- [ ] All existing tests pass
- [ ] New tests for each formatter with mock view

## Technical Details

### Implementation Approach

#### Entry Point Changes

**File**: `src/main.rs` or `src/commands/analyze.rs`

```rust
// Before: Each output calls different methods
fn run_analysis(args: &Args) -> Result<()> {
    let analysis = analyze_project(&config)?;

    match args.output_format {
        OutputFormat::Tui => {
            let app = ResultsApp::new(analysis);  // Uses analysis.items directly
            run_tui(app)?;
        }
        OutputFormat::Terminal => {
            // Calls get_top_mixed_priorities internally
            let output = format_priorities(&analysis, args.limit, args.verbosity);
            println!("{}", output);
        }
        OutputFormat::Json => {
            // Calls get_top_mixed_priorities(usize::MAX) internally
            let json = convert_to_unified_format(&analysis);
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Markdown => {
            // Calls apply_filters then get_top_mixed_priorities
            output_markdown(&analysis, args.top, args.tail, args.verbosity)?;
        }
    }
}

// After: Single prepare_view call, all formats use same view
fn run_analysis(args: &Args) -> Result<()> {
    let analysis = analyze_project(&config)?;

    // Read configuration at boundary (I/O)
    let view_config = ViewConfig {
        min_score_threshold: get_min_score_threshold(),  // reads env/args
        exclude_t4_maintenance: !args.show_t4,
        limit: args.limit,
        sort_by: args.sort_by.into(),
        compute_groups: matches!(args.output_format, OutputFormat::Tui),
    };

    // Single pure transformation
    let view = prepare_view(&analysis, &view_config, &TierConfig::default());

    // All formats consume same view
    match args.output_format {
        OutputFormat::Tui => {
            let app = ResultsApp::new(view);
            run_tui(app)?;
        }
        OutputFormat::Terminal => {
            let output = format_terminal(&view, args.verbosity);
            println!("{}", output);
        }
        OutputFormat::Json => {
            let json = format_json(&view);
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Markdown => {
            let md = format_markdown(&view, args.verbosity);
            println!("{}", md);
        }
    }
}
```

#### TUI Changes

**File**: `src/tui/results/app.rs`

```rust
// Before
impl ResultsApp {
    pub fn new(analysis: UnifiedAnalysis) -> Self {
        let item_count = analysis.items.len();  // Only function items!
        let filtered_indices: Vec<usize> = (0..item_count).collect();
        // ...
    }
}

// After
impl ResultsApp {
    pub fn new(view: PreparedDebtView) -> Self {
        let item_count = view.items.len();
        let filtered_indices: Vec<usize> = (0..item_count).collect();

        Self {
            view,  // Store the prepared view
            filtered_indices,
            // ...
        }
    }

    /// Returns items for ungrouped display.
    pub fn items(&self) -> &[ViewItem] {
        &self.view.items
    }

    /// Returns groups for grouped display.
    pub fn groups(&self) -> &[LocationGroup] {
        &self.view.groups
    }

    /// Returns summary statistics.
    pub fn summary(&self) -> &ViewSummary {
        &self.view.summary
    }
}
```

**File**: `src/tui/results/list_view.rs`

```rust
// Before: Calls grouping module
fn render_list(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    if app.show_grouped {
        let groups = grouping::group_by_location(
            app.analysis.items.iter().filter(/* ... */),
            app.sort_by,
        );
        render_grouped_list(frame, &groups, app, area, theme);
    } else {
        render_ungrouped_list(frame, app, area, theme);
    }
}

// After: Uses pre-computed groups from view
fn render_list(frame: &mut Frame, app: &ResultsApp, area: Rect, theme: &Theme) {
    if app.show_grouped {
        render_grouped_list(frame, app.groups(), app, area, theme);
    } else {
        render_ungrouped_list(frame, app.items(), app, area, theme);
    }
}
```

#### Terminal Changes

**File**: `src/output/terminal.rs`

```rust
// Before
pub fn format_priorities(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
) -> String {
    let top_items = analysis.get_top_mixed_priorities(limit);  // Filtering here
    // Format items...
}

// After
pub fn format_terminal(view: &PreparedDebtView, verbosity: u8) -> String {
    let mut output = String::new();

    // Header
    writeln!(output, "=== Debtmap Report ===").unwrap();
    writeln!(output).unwrap();

    // Items (already filtered and sorted)
    for (idx, item) in view.items.iter().enumerate() {
        format_item(&mut output, idx + 1, item, verbosity);
        writeln!(output).unwrap();
    }

    // Summary
    writeln!(output, "---").unwrap();
    writeln!(
        output,
        "Total Debt Score: {:.0}",
        view.summary.total_debt_score
    ).unwrap();
    writeln!(
        output,
        "Items: {} (filtered {} by tier, {} by score)",
        view.summary.total_items_after_filter,
        view.summary.filtered_by_tier,
        view.summary.filtered_by_score,
    ).unwrap();

    output
}

fn format_item(output: &mut String, rank: usize, item: &ViewItem, verbosity: u8) {
    let loc = item.location();
    writeln!(
        output,
        "{}. [{}] {} - {}:{}",
        rank,
        item.severity().as_str().to_uppercase(),
        item.display_type(),
        loc.file.display(),
        loc.function.as_deref().unwrap_or("(file-level)"),
    ).unwrap();

    writeln!(output, "   Score: {:.1}", item.score()).unwrap();

    if verbosity >= 1 {
        // Additional details...
    }
}
```

#### JSON Changes

**File**: `src/output/json.rs`

```rust
// Before
pub fn convert_to_unified_format(analysis: &UnifiedAnalysis) -> UnifiedOutput {
    let all_items = analysis.get_top_mixed_priorities(usize::MAX);  // Filtering here
    // Convert to output format...
}

// After
pub fn format_json(view: &PreparedDebtView) -> serde_json::Value {
    // PreparedDebtView is already serializable
    serde_json::to_value(view).unwrap_or_else(|_| {
        serde_json::json!({
            "error": "Failed to serialize view"
        })
    })
}

// Or for more control over output structure:
pub fn format_json_structured(view: &PreparedDebtView) -> serde_json::Value {
    serde_json::json!({
        "items": view.items.iter().map(format_item_json).collect::<Vec<_>>(),
        "summary": {
            "total_items": view.summary.total_items_after_filter,
            "total_debt_score": view.summary.total_debt_score,
            "debt_density": view.summary.debt_density,
            "filtered": {
                "by_tier": view.summary.filtered_by_tier,
                "by_score": view.summary.filtered_by_score,
            },
            "distribution": view.summary.score_distribution,
            "categories": view.summary.category_counts,
        },
        "config": view.config,
    })
}

fn format_item_json(item: &ViewItem) -> serde_json::Value {
    let loc = item.location();
    serde_json::json!({
        "type": item.display_type(),
        "file": loc.file,
        "function": loc.function,
        "line": loc.line,
        "score": item.score(),
        "severity": item.severity().as_str(),
        "category": format!("{:?}", item.category()),
    })
}
```

#### Markdown Changes

**File**: `src/output/markdown.rs`

```rust
// Before
pub fn output_markdown(
    analysis: &priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    verbosity: u8,
    // ...
) -> Result<()> {
    let filtered_analysis = apply_filters(analysis, top, tail);  // First filter
    let top_items = filtered_analysis.get_top_mixed_priorities(limit);  // Second filter!
    // Format...
}

fn apply_filters(...) -> UnifiedAnalysis {
    // Separate filtering for items and file_items
    // This is the source of double-filtering bug
}

// After
pub fn format_markdown(view: &PreparedDebtView, verbosity: u8) -> String {
    let mut output = String::new();

    // Header
    writeln!(output, "# Debtmap Report\n").unwrap();

    // Summary
    writeln!(output, "## Summary\n").unwrap();
    writeln!(
        output,
        "- **Total Items**: {}",
        view.summary.total_items_after_filter
    ).unwrap();
    writeln!(
        output,
        "- **Total Debt Score**: {:.0}",
        view.summary.total_debt_score
    ).unwrap();
    writeln!(
        output,
        "- **Debt Density**: {:.1} per 1K LOC",
        view.summary.debt_density
    ).unwrap();
    writeln!(output).unwrap();

    // Items
    writeln!(output, "## Debt Items\n").unwrap();
    for (idx, item) in view.items.iter().enumerate() {
        format_item_markdown(&mut output, idx + 1, item, verbosity);
    }

    // Footer
    writeln!(output, "---\n").unwrap();
    writeln!(
        output,
        "*Report generated by Debtmap. {} items filtered ({} by tier, {} by score).*",
        view.summary.filtered_by_tier + view.summary.filtered_by_score,
        view.summary.filtered_by_tier,
        view.summary.filtered_by_score,
    ).unwrap();

    output
}

fn format_item_markdown(output: &mut String, rank: usize, item: &ViewItem, verbosity: u8) {
    let loc = item.location();
    let severity_badge = match item.severity() {
        Severity::Critical => "**[CRITICAL]**",
        Severity::High => "**[HIGH]**",
        Severity::Medium => "[MEDIUM]",
        Severity::Low => "[LOW]",
    };

    writeln!(
        output,
        "### {}. {} `{}`\n",
        rank,
        severity_badge,
        loc.file.display(),
    ).unwrap();

    if let Some(func) = &loc.function {
        writeln!(output, "**Function**: `{}`\n", func).unwrap();
    }

    writeln!(output, "**Score**: {:.1}\n", item.score()).unwrap();

    if verbosity >= 1 {
        // Additional details...
    }

    writeln!(output).unwrap();
}
```

### Files to Modify

| File | Changes |
|------|---------|
| `src/main.rs` or `src/commands/analyze.rs` | Single `prepare_view()` call |
| `src/tui/results/app.rs` | Store `PreparedDebtView` instead of `UnifiedAnalysis` |
| `src/tui/results/list_view.rs` | Use `app.groups()` and `app.items()` |
| `src/tui/results/grouping.rs` | Delete (moved to pipeline) |
| `src/output/terminal.rs` | Take `PreparedDebtView` parameter |
| `src/output/json.rs` | Take `PreparedDebtView` parameter |
| `src/output/markdown.rs` | Take `PreparedDebtView`, remove `apply_filters()` |
| `src/priority/formatter/recommendations.rs` | Update to use `PreparedDebtView` |

### Files to Delete

| File | Reason |
|------|--------|
| `src/tui/results/grouping.rs` | Logic moved to `view_pipeline.rs` |

### Deprecation

These functions can be deprecated (and later removed):

```rust
// src/priority/unified_analysis_queries.rs
#[deprecated(since = "0.x.0", note = "Use prepare_view() instead")]
pub fn get_top_mixed_priorities(&self, n: usize) -> Vector<DebtItem> {
    // Keep for backward compatibility during transition
}

// src/output/markdown.rs
// Remove apply_filters() entirely (not public API)
```

## Dependencies

- **Prerequisites**:
  - Spec 250 (types exist)
  - Spec 251 (pipeline exists)
- **Affected Components**:
  - All output formatters
  - TUI app state
  - Main entry point
- **External Dependencies**: None

## Testing Strategy

### Unit Tests (Formatters)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_view() -> PreparedDebtView {
        PreparedDebtView {
            items: vec![
                create_test_view_item(80.0, "critical.rs", "func1"),
                create_test_view_item(50.0, "medium.rs", "func2"),
            ],
            groups: vec![],
            summary: ViewSummary {
                total_items_after_filter: 2,
                total_debt_score: 130.0,
                ..Default::default()
            },
            config: ViewConfig::default(),
        }
    }

    #[test]
    fn test_format_terminal_includes_all_items() {
        let view = create_test_view();

        let output = format_terminal(&view, 0);

        assert!(output.contains("critical.rs"));
        assert!(output.contains("medium.rs"));
        assert!(output.contains("130")); // Total score
    }

    #[test]
    fn test_format_json_serializable() {
        let view = create_test_view();

        let json = format_json(&view);

        assert!(json.is_object());
        assert!(json["items"].is_array());
        assert_eq!(json["items"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_format_markdown_structure() {
        let view = create_test_view();

        let md = format_markdown(&view, 0);

        assert!(md.starts_with("# Debtmap Report"));
        assert!(md.contains("## Summary"));
        assert!(md.contains("## Debt Items"));
        assert!(md.contains("CRITICAL"));
    }
}
```

### Snapshot Tests

```rust
#[test]
fn test_terminal_output_snapshot() {
    let view = load_fixture_view();
    let output = format_terminal(&view, 1);
    insta::assert_snapshot!(output);
}

#[test]
fn test_json_output_snapshot() {
    let view = load_fixture_view();
    let json = format_json(&view);
    insta::assert_json_snapshot!(json);
}

#[test]
fn test_markdown_output_snapshot() {
    let view = load_fixture_view();
    let output = format_markdown(&view, 1);
    insta::assert_snapshot!(output);
}
```

### Integration Tests

```rust
#[test]
fn test_all_formats_show_same_items() {
    let analysis = load_test_analysis();
    let config = ViewConfig::default();
    let view = prepare_view(&analysis, &config, &TierConfig::default());

    let terminal = format_terminal(&view, 0);
    let json = format_json(&view);
    let markdown = format_markdown(&view, 0);

    // All formats should reference same number of items
    let terminal_count = terminal.matches("Score:").count();
    let json_count = json["items"].as_array().unwrap().len();
    let markdown_count = markdown.matches("**Score**:").count();

    assert_eq!(terminal_count, view.items.len());
    assert_eq!(json_count, view.items.len());
    assert_eq!(markdown_count, view.items.len());
}
```

### TUI Integration Tests

```rust
#[test]
fn test_tui_app_with_prepared_view() {
    let view = create_test_view();
    let app = ResultsApp::new(view.clone());

    assert_eq!(app.items().len(), view.items.len());
    assert_eq!(app.groups().len(), view.groups.len());
}

#[test]
fn test_tui_grouped_mode_uses_precomputed_groups() {
    let view = create_test_view_with_groups();
    let app = ResultsApp::new(view);

    // Should use pre-computed groups, not compute on render
    let groups = app.groups();
    assert!(!groups.is_empty());
}
```

## Documentation Requirements

### Code Documentation

Each formatter function documented with:
- Input/output contract
- Verbosity level effects
- Example output

### User Documentation

Update README/docs to explain:
- Consistent output across formats
- How filtering works (at entry point, not per-format)
- How to customize via ViewConfig

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
## Output Formatting

All output formats consume `PreparedDebtView`:

```
┌─────────────────────┐
│  prepare_view()     │ ← Single transformation
└─────────────────────┘
          │
          ▼
┌─────────────────────┐
│  PreparedDebtView   │
└─────────────────────┘
    │   │   │   │
    ▼   ▼   ▼   ▼
   TUI Term JSON Markdown
```

### Format Responsibilities

| Format | Responsibility |
|--------|---------------|
| TUI | Interactive display, uses groups |
| Terminal | Colored text output |
| JSON | Machine-readable export |
| Markdown | Documentation/reports |

Each formatter is a thin rendering layer with no business logic.
```

## Implementation Notes

### Migration Steps

1. **Phase 1: Add new formatters alongside old**
   - Create `format_terminal()`, `format_json()`, `format_markdown()`
   - Keep old functions working
   - Add tests for new functions

2. **Phase 2: Update TUI**
   - Change `ResultsApp::new()` to take `PreparedDebtView`
   - Update all TUI rendering to use new accessors
   - Delete `src/tui/results/grouping.rs`

3. **Phase 3: Update entry point**
   - Single `prepare_view()` call
   - Route to new formatters
   - Verify output matches old

4. **Phase 4: Remove old code**
   - Deprecate `get_top_mixed_priorities()`
   - Remove `apply_filters()`
   - Clean up unused imports

### Verification

After each phase:
1. Run full test suite
2. Compare output visually (same items, same order)
3. Verify TUI works correctly
4. Check JSON structure unchanged

### Common Pitfalls

1. **TUI state management** - Ensure filter/sort in TUI still works with pre-computed view
2. **JSON compatibility** - Existing tools may depend on JSON structure
3. **Missing items** - Verify file_items appear in all formats now

## Migration and Compatibility

### Breaking Changes

**JSON Output**: Structure may change slightly. Document differences.

**API**: Internal functions change signatures. Not public API.

### Migration for Users

No user action required. Output should be consistent (or improved).

### Migration for Developers

Update any code calling old formatting functions to:
1. Create `ViewConfig`
2. Call `prepare_view()`
3. Pass `PreparedDebtView` to formatter

## Success Metrics

- All formats produce consistent results
- TUI shows file_items (god objects) now
- No double-filtering in markdown
- Formatters under 200 lines each
- All tests pass
- Visual output matches expectations

## Follow-up Work

- Add HTML output format (easy with unified view)
- Add CSV export format
- Consider streaming for large views

## References

- **Spec 250** - Unified View Data Model
- **Spec 251** - View Preparation Pipeline
- **Spec 186** - Split formatter.rs (similar pattern)
- **Stillwater PHILOSOPHY.md** - "Imperative Shell" pattern
- **src/tui/results/grouping.rs** - Code to consolidate
- **src/priority/unified_analysis_queries.rs** - Code to deprecate
