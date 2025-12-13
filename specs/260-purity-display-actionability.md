---
number: 260
title: Improve Purity Display Actionability in TUI
category: optimization
priority: medium
status: draft
dependencies: [259]
created: 2025-12-12
---

# Specification 260: Improve Purity Display Actionability in TUI

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 259 (Fix Constants False Positive)

## Context

**Current Problem**: The TUI Data Flow page shows purity as a binary "Yes/No" with a confidence percentage, but doesn't explain:
- **Why** a function is impure (which specific reason)
- **How close** to pure (e.g., "1 I/O operation away from pure")
- **What to fix** to make it pure (actionable suggestions)

**Current Display** (`src/tui/results/detail_pages/data_flow.rs`):
```
purity analysis
pure        No
confidence  85.0%
```

**Desired Display**:
```
purity analysis
pure        No (1 issue)
confidence  85.0%
reasons     I/O: println! at line 42
suggestion  Extract println! to caller for pure function
```

## Objective

Make purity analysis actionable by showing specific impurity reasons and suggesting fixes, helping developers understand what prevents purity and how to achieve it.

## Requirements

### Functional Requirements

1. **Show Impurity Reasons**
   - Display each specific impurity reason with context
   - Show line numbers when available
   - Use clear, descriptive language

2. **Quantify Proximity to Pure**
   - Show count of violations: "No (2 issues)" vs just "No"
   - Distinguish single-issue functions as "Almost Pure"

3. **Provide Suggestions**
   - For common patterns (logging, time, random), show fix suggestions
   - Link to existing "Almost Pure" detection (Spec 162)

4. **Enhance Clipboard Export**
   - Include reasons and suggestions in clipboard copy

### Non-Functional Requirements

- Keep display compact (don't overflow screen)
- Use consistent styling with other detail pages

## Implementation

### Update PurityInfo Display

Modify `src/tui/results/detail_pages/data_flow.rs`:

```rust
/// Render purity analysis section with actionable details
fn render_purity_section(
    lines: &mut Vec<Line<'static>>,
    purity_info: &PurityInfo,
    theme: &Theme,
    width: u16,
) -> bool {
    add_section_header(lines, "purity analysis", theme);

    // Show pure status with issue count
    let pure_display = if purity_info.is_pure {
        "Yes".to_string()
    } else {
        let issue_count = purity_info.impurity_reasons.len();
        if issue_count == 0 {
            "No".to_string()
        } else if issue_count == 1 {
            "No (1 issue)".to_string()
        } else {
            format!("No ({} issues)", issue_count)
        }
    };

    add_label_value(lines, "pure", pure_display, theme, width);

    add_label_value(
        lines,
        "confidence",
        format!("{:.1}%", purity_info.confidence * 100.0),
        theme,
        width,
    );

    // Show impurity reasons with suggestions
    if !purity_info.impurity_reasons.is_empty() {
        // Format reasons with suggestions
        let formatted_reasons = purity_info
            .impurity_reasons
            .iter()
            .map(|r| format_reason_with_suggestion(r))
            .collect::<Vec<_>>()
            .join("; ");

        add_label_value(lines, "reasons", formatted_reasons, theme, width);

        // Show actionable suggestion for almost-pure functions
        if purity_info.impurity_reasons.len() <= 2 {
            if let Some(suggestion) = get_fix_suggestion(&purity_info.impurity_reasons) {
                add_label_value(lines, "fix", suggestion, theme, width);
            }
        }
    }

    add_blank_line(lines);
    true
}

/// Format a reason with context
fn format_reason_with_suggestion(reason: &str) -> String {
    // Parse common patterns and add context
    if reason.to_lowercase().contains("i/o") || reason.to_lowercase().contains("print") {
        format!("{} (extract to caller)", reason)
    } else if reason.to_lowercase().contains("mutable") {
        format!("{} (consider &self instead)", reason)
    } else if reason.to_lowercase().contains("unsafe") {
        format!("{}", reason) // Can't easily fix unsafe
    } else if reason.to_lowercase().contains("external") {
        format!("{} (pass as parameter)", reason)
    } else {
        reason.to_string()
    }
}

/// Get actionable fix suggestion for almost-pure functions
fn get_fix_suggestion(reasons: &[String]) -> Option<String> {
    if reasons.len() > 2 {
        return None; // Too many issues for simple fix
    }

    let first_reason = reasons.first()?.to_lowercase();

    if first_reason.contains("i/o") || first_reason.contains("print") || first_reason.contains("log") {
        Some("Move logging to caller - function becomes pure".to_string())
    } else if first_reason.contains("time") || first_reason.contains("now") {
        Some("Pass time as parameter instead of calling now()".to_string())
    } else if first_reason.contains("random") || first_reason.contains("rand") {
        Some("Inject RNG as parameter for deterministic behavior".to_string())
    } else if first_reason.contains("mutable param") {
        Some("Consider taking &self instead of &mut self".to_string())
    } else {
        None
    }
}
```

### Update ImpurityReason Display

In `src/analyzers/purity_detector.rs`, improve reason descriptions:

```rust
impl ImpurityReason {
    /// Get a user-friendly description of the impurity reason
    pub fn display_description(&self) -> String {
        match self {
            Self::SideEffects => "Has side effects".to_string(),
            Self::MutableParameters => "Takes &mut self or &mut T".to_string(),
            Self::IOOperations => "I/O operations (print, file, network)".to_string(),
            Self::UnsafeCode => "Contains unsafe block".to_string(),
            Self::ModifiesExternalState => "Modifies external state".to_string(),
            Self::AccessesExternalState => "Reads external state".to_string(),
        }
    }
}
```

### Update Clipboard Export

In `src/tui/results/actions.rs`:

```rust
/// Format purity section with reasons for clipboard
fn format_purity_section(func_id: &FunctionId, data_flow: &DataFlowGraph) -> Option<String> {
    let purity = data_flow.get_purity_info(func_id)?;
    let mut lines = vec![format!(
        "=== Purity Analysis ===\nPure: {}\nConfidence: {:.1}%",
        if purity.is_pure { "Yes" } else { "No" },
        purity.confidence * 100.0
    )];

    if !purity.impurity_reasons.is_empty() {
        lines.push(format!("Reasons:\n  - {}", purity.impurity_reasons.join("\n  - ")));

        // Add fix suggestion for almost-pure
        if purity.impurity_reasons.len() <= 2 {
            if let Some(suggestion) = get_fix_suggestion(&purity.impurity_reasons) {
                lines.push(format!("Suggested Fix: {}", suggestion));
            }
        }
    }

    Some(lines.join("\n"))
}
```

## Acceptance Criteria

- [ ] Purity display shows issue count: "No (2 issues)" instead of just "No"
- [ ] Impurity reasons are displayed with clear descriptions
- [ ] Fix suggestions appear for functions with 1-2 issues
- [ ] Suggestions are specific to violation type:
  - I/O: "Move logging to caller"
  - Time: "Pass time as parameter"
  - Random: "Inject RNG as parameter"
  - Mutable params: "Consider &self"
- [ ] Clipboard export includes reasons and suggestions
- [ ] Display remains compact and readable on narrow terminals

## Technical Details

### Files to Modify

| File | Changes |
|------|---------|
| `src/tui/results/detail_pages/data_flow.rs` | Enhanced `render_purity_section` |
| `src/tui/results/actions.rs` | Updated `format_purity_section` |
| `src/analyzers/purity_detector.rs` | Add `display_description()` to `ImpurityReason` |

### Display Examples

**Before**:
```
purity analysis
pure        No
confidence  85.0%
reasons     I/O operations, mutable parameters
```

**After**:
```
purity analysis
pure        No (2 issues)
confidence  85.0%
reasons     I/O: println! (extract to caller); Takes &mut self
fix         Move I/O to caller for pure function
```

**Almost Pure (1 issue)**:
```
purity analysis
pure        No (1 issue)
confidence  92.0%
reasons     I/O: println! macro
fix         Move logging to caller - function becomes pure
```

## Dependencies

- **Prerequisites**: Spec 259 (improved constant handling)
- **Affected Components**: TUI detail pages, clipboard export
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Test reason formatting functions
- **Integration Tests**: Verify TUI renders correctly with various purity states
- **Manual Testing**: Check display on different terminal widths

## Documentation Requirements

- Update TUI help text if applicable
- No external documentation changes needed

## Implementation Notes

- Keep suggestions concise to fit on one line
- Truncate long reason lists with "... and N more"
- Consider color coding: green for suggestions, yellow for warnings

## Migration and Compatibility

- No breaking changes
- Purely additive display enhancements
