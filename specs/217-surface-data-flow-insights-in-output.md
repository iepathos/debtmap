---
number: 217
title: Surface Data Flow Insights in Output
category: optimization
priority: high
status: draft
dependencies: [216]
created: 2025-01-09
---

# Specification 217: Surface Data Flow Insights in Output

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 216 (Complete Data Flow Graph Population)

## Context

After Spec 216 completes, the `DataFlowGraph` will contain rich information about:
- Live vs dead mutations (from CFG analysis)
- Escaping variables (which affect function output)
- I/O operations with locations and types
- Variable dependencies and data transformations
- Taint propagation through mutations

However, **none of this information is currently displayed** to users:

- ❌ Markdown output: No data flow section in enhanced_markdown writers
- ❌ JSON output: `DataFlowGraph` has serde but actual data isn't shown
- ⚠️ TUI: Shows basic `is_pure`/`purity_confidence` from FunctionMetrics, not from DataFlowGraph
- ❌ Recommendations: Data flow insights not incorporated into actionable advice

**Current state**: Users see "is_pure: true, confidence: 0.85" but don't understand WHY or see the detailed analysis that produced this result.

## Objective

Make data flow analysis insights visible and actionable by:

1. **TUI Enhancement**: Add detailed data flow page showing mutations, I/O operations, escape analysis
2. **Markdown Output**: Include data flow section in enhanced markdown with actionable insights
3. **Recommendations**: Incorporate data flow patterns into refactoring suggestions
4. **Analysis Progress**: Show data flow analysis stages and counts during execution

## Requirements

### Functional Requirements

1. **TUI Data Flow Page**
   - Add new detail page (Page 5) showing data flow analysis
   - Display live mutations vs dead stores
   - Show escaping variables and their impact
   - List I/O operations with line numbers
   - Visualize taint propagation if present

2. **Enhanced Markdown Output**
   - Add "Data Flow Analysis" section after purity analysis
   - Show mutation details (live vs dead)
   - Display I/O operation summary
   - Include escape analysis results
   - Provide actionable insights based on patterns

3. **Recommendation Integration**
   - Suggest extracting pure subsets from impure functions
   - Recommend isolating I/O operations
   - Flag unnecessary mutations (dead stores)
   - Identify refactoring opportunities based on escape analysis

4. **Progress Visibility**
   - Display data flow population progress during analysis
   - Show counts of detected I/O operations
   - Report mutations analyzed and dead stores found

### Non-Functional Requirements

- **Clarity**: Data flow information must be understandable to non-experts
- **Actionability**: Every displayed insight should suggest a specific improvement
- **Performance**: Rendering data flow info should add < 100ms to output generation
- **Consistency**: Follow existing output formatting and style conventions

## Acceptance Criteria

- [ ] TUI detail view includes new "Data Flow" page (accessible via Page Down)
- [ ] Data flow page shows mutations categorized as live/dead/escaping
- [ ] I/O operations are listed with operation type, variables, and line numbers
- [ ] Markdown output includes "Data Flow Analysis" section for impure functions
- [ ] Recommendations mention specific refactoring based on data flow (e.g., "Extract lines 15-20 to pure function")
- [ ] Analysis progress shows data flow population counts
- [ ] JSON output includes `data_flow_summary` field with key metrics
- [ ] User can understand why a function is impure by reading the output
- [ ] Documentation explains how to interpret data flow insights

## Technical Details

### Implementation Approach

**Phase 1: TUI Data Flow Page**

Add new detail page in `src/tui/results/detail_pages/data_flow.rs`:

```rust
pub fn render(
    frame: &mut Frame,
    area: Rect,
    item: &UnifiedDebtItem,
    data_flow: &DataFlowGraph,
    theme: &Theme,
) {
    let func_id = FunctionId::new(
        item.location.file.clone(),
        item.function_name.clone(),
        item.location.line,
    );

    let mut lines = Vec::new();

    // Mutation Analysis Section
    if let Some(mutation_info) = data_flow.mutation_analysis.get(&func_id) {
        add_section_header(&mut lines, "MUTATION ANALYSIS", theme);

        add_label_value(
            &mut lines,
            "Total Mutations",
            mutation_info.total_mutations.to_string(),
            theme,
        );

        add_label_value(
            &mut lines,
            "Live Mutations",
            mutation_info.live_mutations.len().to_string(),
            theme,
        );

        add_label_value(
            &mut lines,
            "Dead Stores",
            mutation_info.dead_stores.len().to_string(),
            theme,
        );

        if !mutation_info.live_mutations.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from("Live Mutations:").style(theme.label));
            for mutation in &mutation_info.live_mutations {
                lines.push(Line::from(format!("  • {}", mutation))
                    .style(theme.warning));
            }
        }

        if !mutation_info.dead_stores.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from("Dead Stores:").style(theme.label));
            for dead in &mutation_info.dead_stores {
                lines.push(Line::from(format!("  • {} (never read)", dead))
                    .style(theme.dim));
            }
        }
    }

    // I/O Operations Section
    if let Some(io_ops) = data_flow.get_io_operations(&func_id) {
        lines.push(Line::from(""));
        add_section_header(&mut lines, "I/O OPERATIONS", theme);

        for op in io_ops {
            lines.push(Line::from(format!(
                "  {} at line {} (variables: {})",
                op.operation_type,
                op.line,
                op.variables.join(", ")
            )).style(theme.warning));
        }
    }

    // Escape Analysis Section
    if let Some(cfg_analysis) = data_flow.cfg_analysis.get(&func_id) {
        lines.push(Line::from(""));
        add_section_header(&mut lines, "ESCAPE ANALYSIS", theme);

        let escaping_count = cfg_analysis.escape_info.escaping_vars.len();
        add_label_value(
            &mut lines,
            "Escaping Variables",
            escaping_count.to_string(),
            theme,
        );

        if escaping_count > 0 {
            lines.push(Line::from("Variables affecting return value:")
                .style(theme.label));
            for var in &cfg_analysis.escape_info.return_dependencies {
                lines.push(Line::from(format!("  • {:?}", var))
                    .style(theme.value));
            }
        }
    }

    render_scrollable_text(frame, area, lines, theme);
}
```

Update `src/tui/results/detail_pages/mod.rs`:

```rust
pub enum DetailPage {
    Overview,
    Dependencies,
    GitContext,
    Patterns,
    DataFlow,  // NEW
}

impl DetailPage {
    pub fn render(&self, /* ... */) {
        match self {
            // ...
            Self::DataFlow => data_flow::render(frame, area, item, &analysis.data_flow_graph, theme),
        }
    }

    pub fn title(&self) -> &str {
        match self {
            // ...
            Self::DataFlow => "Data Flow",
        }
    }
}
```

**Phase 2: Enhanced Markdown Output**

Add to `src/io/writers/enhanced_markdown/debt_writer.rs`:

```rust
fn write_data_flow_section<W: Write>(
    writer: &mut W,
    item: &UnifiedDebtItem,
    data_flow: &DataFlowGraph,
) -> Result<()> {
    let func_id = FunctionId::new(
        item.location.file.clone(),
        item.function_name.clone(),
        item.location.line,
    );

    // Only include section if there's data flow info
    if !has_data_flow_info(&func_id, data_flow) {
        return Ok(());
    }

    writeln!(writer, "\n**Data Flow Analysis**\n")?;

    // Mutation summary
    if let Some(mutation_info) = data_flow.mutation_analysis.get(&func_id) {
        writeln!(writer, "- Mutations: {} total, {} live, {} dead stores",
            mutation_info.total_mutations,
            mutation_info.live_mutations.len(),
            mutation_info.dead_stores.len()
        )?;

        if !mutation_info.dead_stores.is_empty() {
            writeln!(writer, "  - **Opportunity**: Remove {} dead store(s) to simplify code",
                mutation_info.dead_stores.len()
            )?;
        }

        if mutation_info.live_mutations.len() <= 2 {
            writeln!(writer, "  - **Almost Pure**: Only {} live mutation(s), consider extracting pure subset",
                mutation_info.live_mutations.len()
            )?;
        }
    }

    // I/O operations
    if let Some(io_ops) = data_flow.get_io_operations(&func_id) {
        writeln!(writer, "- I/O Operations: {} detected", io_ops.len())?;
        for op in io_ops.iter().take(3) {
            writeln!(writer, "  - {} at line {}", op.operation_type, op.line)?;
        }
        if io_ops.len() > 3 {
            writeln!(writer, "  - ... and {} more", io_ops.len() - 3)?;
        }
        writeln!(writer, "  - **Recommendation**: Consider isolating I/O in separate functions")?;
    }

    // Escape analysis
    if let Some(cfg_analysis) = data_flow.cfg_analysis.get(&func_id) {
        let escaping_count = cfg_analysis.escape_info.escaping_vars.len();
        if escaping_count > 0 {
            writeln!(writer, "- Escaping Variables: {} affecting return value", escaping_count)?;
        }
    }

    Ok(())
}
```

**Phase 3: Recommendation Enhancement**

Update recommendation generation in `src/priority/scoring/debt_item.rs`:

```rust
pub fn generate_recommendation_with_data_flow(
    // ... existing params ...
    data_flow: Option<&DataFlowGraph>,
) -> ActionableRecommendation {
    let mut recommendation = generate_base_recommendation(/* ... */);

    if let Some(flow) = data_flow {
        let func_id = FunctionId::new(/* ... */);

        // Add data flow insights to rationale
        if let Some(mutation_info) = flow.mutation_analysis.get(&func_id) {
            if mutation_info.dead_stores.len() > 0 {
                recommendation.rationale.push_str(&format!(
                    " Contains {} dead store(s) that can be removed.",
                    mutation_info.dead_stores.len()
                ));
            }

            if mutation_info.live_mutations.len() <= 2 && mutation_info.total_mutations > 2 {
                recommendation.implementation_steps.push(
                    format!("Remove {} dead mutations to create pure function subset",
                        mutation_info.total_mutations - mutation_info.live_mutations.len())
                );
            }
        }

        // Add I/O isolation recommendations
        if let Some(io_ops) = flow.get_io_operations(&func_id) {
            if io_ops.len() > 0 {
                recommendation.implementation_steps.push(
                    format!("Extract {} I/O operation(s) to separate function(s)", io_ops.len())
                );
            }
        }
    }

    recommendation
}
```

**Phase 4: Progress Display**

Update `src/builders/parallel_unified_analysis.rs`:

```rust
fn spawn_data_flow_task(&self, /* ... */) {
    scope.spawn(move |_| {
        progress.set_message("Building data flow graph...");
        let mut data_flow = DataFlowGraph::from_call_graph((*call_graph).clone());

        progress.set_message("Populating purity analysis...");
        // ... populate ...

        progress.set_message("Detecting I/O operations...");
        let io_count = populate_io_operations(&mut data_flow, &metrics);

        progress.set_message("Analyzing mutations...");
        let mutation_count = populate_mutation_info(&mut data_flow, &purity_results);

        progress.finish_with_message(&format!(
            "Data flow complete: {} I/O ops, {} mutations analyzed",
            io_count, mutation_count
        ));
    });
}
```

### Architecture Changes

**New Files**:
- `src/tui/results/detail_pages/data_flow.rs` - TUI data flow page

**Modified Files**:
- `src/tui/results/detail_pages/mod.rs` - Add DataFlow page enum
- `src/io/writers/enhanced_markdown/debt_writer.rs` - Add data flow section
- `src/priority/scoring/debt_item.rs` - Enhance recommendations
- `src/builders/parallel_unified_analysis.rs` - Update progress messages

### User Interface Design

**TUI Layout**:
```
┌─ Data Flow Analysis ─────────────────────────────────┐
│                                                       │
│ MUTATION ANALYSIS                                     │
│   Total Mutations:    5                               │
│   Live Mutations:     2                               │
│   Dead Stores:        3                               │
│                                                       │
│   Live Mutations:                                     │
│     • result (line 15)                                │
│     • counter (line 18)                               │
│                                                       │
│   Dead Stores:                                        │
│     • temp (never read)                               │
│     • intermediate (never read)                       │
│     • cache (never read)                              │
│                                                       │
│ I/O OPERATIONS                                        │
│   • File::open at line 10 (variables: path)          │
│   • read_to_string at line 12 (variables: content)   │
│                                                       │
│ ESCAPE ANALYSIS                                       │
│   Escaping Variables: 2                               │
│   Variables affecting return value:                   │
│     • result                                          │
│     • counter                                         │
│                                                       │
└───────────────────────────────────────────────────────┘
    [1/5] ← → Page [Tab] List  [q] Quit
```

**Markdown Format**:
```markdown
### function_name

**Location**: `src/module.rs:42`
**Priority**: CRITICAL (Score: 85.3)

**Data Flow Analysis**

- Mutations: 5 total, 2 live, 3 dead stores
  - **Opportunity**: Remove 3 dead store(s) to simplify code
  - **Almost Pure**: Only 2 live mutation(s), consider extracting pure subset
- I/O Operations: 2 detected
  - File::open at line 10
  - read_to_string at line 12
  - **Recommendation**: Consider isolating I/O in separate functions
- Escaping Variables: 2 affecting return value

**Recommendation**
Extract pure computation subset (remove dead stores and isolate I/O)
```

## Dependencies

**Prerequisites**:
- Spec 216: Complete Data Flow Graph Population (REQUIRED)

**Affected Components**:
- All output formatters (markdown, JSON, TUI)
- Recommendation generation
- Progress display

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_render_data_flow_page() {
    let item = create_test_item_with_mutations();
    let data_flow = create_populated_data_flow_graph();

    let output = render_to_string(item, data_flow);

    assert!(output.contains("MUTATION ANALYSIS"));
    assert!(output.contains("Live Mutations: 2"));
    assert!(output.contains("Dead Stores: 3"));
}

#[test]
fn test_markdown_data_flow_section() {
    let item = create_test_item();
    let data_flow = create_data_flow_with_io();

    let markdown = generate_markdown(item, data_flow);

    assert!(markdown.contains("Data Flow Analysis"));
    assert!(markdown.contains("I/O Operations: 2 detected"));
}
```

### Integration Tests

```rust
#[test]
fn test_full_output_includes_data_flow() {
    let analysis = run_full_analysis();

    // Check TUI has data flow page
    assert!(analysis.items[0].detail_pages.contains(&DetailPage::DataFlow));

    // Check markdown includes data flow
    let markdown = format_to_markdown(&analysis);
    assert!(markdown.contains("Data Flow Analysis"));

    // Check recommendations mention data flow insights
    assert!(analysis.items[0].recommendation.implementation_steps
        .iter()
        .any(|step| step.contains("dead store") || step.contains("I/O")));
}
```

### User Acceptance

- [ ] User can understand why function is impure from TUI/markdown
- [ ] Data flow page provides actionable refactoring guidance
- [ ] Recommendations include specific line numbers for changes
- [ ] Progress messages clarify what analysis is happening

## Documentation Requirements

### Code Documentation

- Document data flow page rendering logic
- Add examples of data flow section formatting
- Document recommendation enhancement with data flow

### User Documentation

Update `book/src/tui-guide.md`:
```markdown
## Detail View Pages

### Page 5: Data Flow

The data flow page shows detailed analysis of how data moves through the function:

- **Mutation Analysis**: Distinguishes between live mutations (affect output) and dead stores (can be removed)
- **I/O Operations**: Lists all detected file, network, or console operations with line numbers
- **Escape Analysis**: Shows which variables affect the function's return value

Use this page to identify refactoring opportunities like extracting pure subsets or isolating I/O.
```

Update `book/src/output-guide.md`:
```markdown
## Data Flow Analysis Section

For impure functions, the output includes data flow insights:

- **Mutations**: Shows total vs live mutations, identifies dead stores
- **I/O Operations**: Lists detected I/O with locations
- **Recommendations**: Suggests specific refactorings based on data flow patterns

Example:
[Include markdown example from UI design]
```

## Implementation Notes

### Performance Considerations

- Render data flow sections lazily (only when page is viewed in TUI)
- Cache formatted output for repeated views
- Limit displayed items (e.g., show first 5 I/O ops, collapse rest)

### Progressive Enhancement

Phase 1: Basic display of mutations and I/O
Phase 2: Add escape analysis visualization
Phase 3: Add taint propagation visualization
Phase 4: Interactive exploration (expand/collapse)

### Accessibility

- Use color + symbols (not just color) for live/dead indication
- Provide keyboard shortcuts for navigation
- Ensure screen reader compatibility with semantic structure

## Migration and Compatibility

### Breaking Changes

None - this is additive display functionality.

### Backward Compatibility

- Old output formats remain available
- New sections only appear if data is populated
- JSON output includes new optional fields

### Migration Path

After deploying Spec 216, users will immediately see new data flow sections in output without any configuration changes.
