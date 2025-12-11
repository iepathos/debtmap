---
number: 253
title: Refactor format_split_recommendations_markdown - Extract Pure Helpers
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-11
---

# Specification 253: Refactor format_split_recommendations_markdown - Extract Pure Helpers

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The `format_split_recommendations_markdown` function at `src/priority/formatter_markdown/priority_item.rs:168` has been identified by debtmap as the #3 critical debt item with:

- **Score**: 100 (CRITICAL)
- **Cyclomatic complexity**: 29 (dampened: 55)
- **Cognitive complexity**: 99
- **Nesting depth**: 6 levels (target: max 2)
- **Function length**: 202 lines (target: max 20)
- **Test coverage**: 0%
- **Git history**: 12.9 changes/month, 33.3% bugs, 7 days old

The debtmap analysis specifically notes:
> "Deep nesting (depth 6) drives cognitive complexity to 99. Cognitive/Cyclomatic ratio of 3.4x confirms nesting is primary issue."

**Recommended action**: Reduce nesting from 6 to 2 levels (primary impact: -55 complexity)

This function formats split recommendations for god object analysis in markdown output. The current implementation has multiple nested conditionals that check:
1. Whether god object analysis exists
2. Whether splits are available
3. Whether `--show-splits` flag is enabled
4. Verbosity level for evidence display
5. Classification confidence thresholds
6. Method list sampling

The deep nesting makes the code hard to test, understand, and maintain.

## Objective

Refactor `format_split_recommendations_markdown` using the **extract-and-flatten pattern** to:
1. Reduce nesting depth from 6 to 2 levels
2. Extract 6 pure helper functions for testability
3. Use early returns to eliminate nested conditionals
4. Enable unit testing of each helper independently
5. Achieve measurable complexity reduction (target: -55 dampened complexity)

## Requirements

### Functional Requirements

1. **Extract Pure Helper Functions**

   Create 6 focused helper functions, each with single responsibility:

   | Function | Responsibility | Lines |
   |----------|---------------|-------|
   | `format_splits_hint()` | Show "--show-splits" hint message | ~5 |
   | `format_detailed_splits()` | Orchestrate full splits display | ~25 |
   | `format_single_split()` | Format one ModuleSplit entry | ~30 |
   | `format_split_evidence()` | Classification evidence with confidence | ~15 |
   | `format_split_methods()` | Methods list with sampling (max 5) | ~20 |
   | `format_no_splits_diagnostic()` | No splits available message | ~15 |

2. **Refactor Main Function**

   Transform main function to use early-return pattern:
   ```rust
   fn format_split_recommendations_markdown(
       output: &mut String,
       item: &FileDebtItem,
       verbosity: u8,
       show_splits: bool,
   ) {
       let god_analysis = match &item.metrics.god_object_analysis {
           Some(analysis) => analysis,
           None => return,
       };

       if god_analysis.recommended_splits.is_empty() {
           if show_splits {
               format_no_splits_diagnostic(output);
           }
           return;
       }

       if !show_splits {
           format_splits_hint(output);
           return;
       }

       format_detailed_splits(output, god_analysis, &extension, verbosity);
   }
   ```

3. **Eliminate Code Duplication**

   The current function has duplicated logic for `representative_methods` vs `methods_to_move` display. Extract into single `format_split_methods()` function that handles both cases.

4. **Preserve Exact Output**

   All markdown output must remain byte-for-byte identical. The refactoring must be purely structural.

### Non-Functional Requirements

1. **Complexity Targets**
   - Main function: max 10 lines, max nesting depth 2
   - Helper functions: max 30 lines each, max nesting depth 2
   - Each function cyclomatic complexity < 8
   - Each function cognitive complexity < 15

2. **Testability**
   - Each helper function must be pure (take inputs, return String or write to &mut String)
   - No global state access
   - All conditionals testable via function parameters

3. **Code Organization**
   - Keep all functions in same file (cohesive responsibility)
   - Order helpers logically (hint, detailed, single, evidence, methods, diagnostic)
   - Add doc comments for each helper

## Acceptance Criteria

- [ ] `format_split_recommendations_markdown` reduced to ~15 lines max
- [ ] 6 helper functions extracted with clear single responsibility
- [ ] Maximum nesting depth in any function is 2 levels
- [ ] Cyclomatic complexity per function < 8
- [ ] All existing tests pass (no regression)
- [ ] Markdown output is byte-for-byte identical before/after
- [ ] At least 5 new unit tests added for helper functions
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt` passes
- [ ] Debtmap re-analysis shows score reduction (target: from 100 to <40)

## Technical Details

### Implementation Approach

#### Phase 1: Extract `format_split_methods()` (Deduplicate)

Lines 262-291 contain duplicated logic for displaying methods. Extract into:

```rust
/// Format method list with sampling (shows max 5 methods)
fn format_split_methods(output: &mut String, methods: &[String], label: &str) {
    if methods.is_empty() {
        return;
    }
    let total = methods.len();
    let sample_size = 5.min(total);

    writeln!(output, "  - {} ({} total):", label, total).unwrap();
    for method in methods.iter().take(sample_size) {
        writeln!(output, "    - `{}()`", method).unwrap();
    }
    if total > sample_size {
        writeln!(output, "    - ... and {} more", total - sample_size).unwrap();
    }
}
```

#### Phase 2: Extract `format_split_evidence()` (Confidence)

Lines 240-258 handle classification evidence. Extract into:

```rust
/// Format classification evidence with confidence warnings
fn format_split_evidence(
    output: &mut String,
    evidence: &ClassificationEvidence,
) {
    writeln!(
        output,
        "  - Confidence: {:.1}% | Signals: {}",
        evidence.confidence * 100.0,
        evidence.evidence.len()
    ).unwrap();

    if evidence.confidence < 0.80 && !evidence.alternatives.is_empty() {
        writeln!(
            output,
            "  - **Warning: Low confidence classification - review recommended**"
        ).unwrap();
    }
}
```

#### Phase 3: Extract `format_single_split()` (Main Loop Body)

Lines 215-324 form the loop body. Extract into a function that formats one `ModuleSplit`:

```rust
/// Format a single module split recommendation
fn format_single_split(
    output: &mut String,
    split: &ModuleSplit,
    extension: &str,
    verbosity: u8,
) {
    // Module name and responsibility
    writeln!(output, "- **{}.{}**", split.suggested_name, extension).unwrap();

    let priority_indicator = match split.priority {
        Priority::High => "High",
        Priority::Medium => "Medium",
        Priority::Low => "Low",
    };

    writeln!(
        output,
        "  - Category: {} | Priority: {}",
        split.responsibility, priority_indicator
    ).unwrap();

    writeln!(
        output,
        "  - Size: {} methods, ~{} lines",
        split.methods_to_move.len(),
        split.estimated_lines,
    ).unwrap();

    // Evidence (conditional on verbosity)
    if verbosity > 0 {
        if let Some(ref evidence) = split.classification_evidence {
            format_split_evidence(output, evidence);
        }
    }

    // Methods list (prefer representative_methods, fallback to methods_to_move)
    let methods = if !split.representative_methods.is_empty() {
        &split.representative_methods
    } else {
        &split.methods_to_move
    };
    format_split_methods(output, methods, "Methods");

    // Fields needed
    if !split.fields_needed.is_empty() {
        writeln!(output, "  - Fields needed: {}", split.fields_needed.join(", ")).unwrap();
    }

    // Trait extraction (conditional on verbosity)
    if let Some(ref trait_suggestion) = split.trait_suggestion {
        if verbosity > 0 {
            writeln!(output, "  - Trait extraction:").unwrap();
            for line in trait_suggestion.lines() {
                writeln!(output, "    {}", line).unwrap();
            }
        }
    }

    // Structs
    if !split.structs_to_move.is_empty() {
        writeln!(output, "  - Structs: {}", split.structs_to_move.join(", ")).unwrap();
    }

    // Warning
    if let Some(warning) = &split.warning {
        writeln!(output, "  - **Warning: {}**", warning).unwrap();
    }

    writeln!(output).unwrap();
}
```

#### Phase 4: Extract Remaining Helpers

1. **`format_splits_hint()`** - Lines 186-192
2. **`format_single_group_note()`** - Lines 327-338
3. **`format_no_splits_diagnostic()`** - Lines 345-365
4. **`format_detailed_splits()`** - Orchestrates the full display

#### Phase 5: Refactor Main Function

Apply early-return pattern to `format_split_recommendations_markdown`.

### Data Flow

```
format_split_recommendations_markdown (orchestrator)
    │
    ├─ Early return: no god_analysis
    ├─ Early return: no splits + format_no_splits_diagnostic()
    ├─ Early return: !show_splits + format_splits_hint()
    │
    └─ format_detailed_splits()
           │
           ├─ Write header (single vs multiple)
           │
           ├─ for each split:
           │      └─ format_single_split()
           │             ├─ format_split_evidence() [if verbosity > 0]
           │             └─ format_split_methods()
           │
           └─ format_single_group_note() [if single split]
```

### Testing Strategy

1. **Unit Tests for Each Helper**
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn format_split_methods_shows_sample_when_many() {
           let mut output = String::new();
           let methods = vec!["a", "b", "c", "d", "e", "f", "g"]
               .into_iter().map(String::from).collect::<Vec<_>>();

           format_split_methods(&mut output, &methods, "Methods");

           assert!(output.contains("Methods (7 total)"));
           assert!(output.contains("`a()`"));
           assert!(output.contains("`e()`"));
           assert!(output.contains("... and 2 more"));
           assert!(!output.contains("`f()`"));
       }

       #[test]
       fn format_split_methods_empty_produces_no_output() {
           let mut output = String::new();
           format_split_methods(&mut output, &[], "Methods");
           assert!(output.is_empty());
       }

       #[test]
       fn format_split_evidence_warns_on_low_confidence() {
           let mut output = String::new();
           let evidence = ClassificationEvidence {
               confidence: 0.65,
               evidence: vec!["signal1".to_string()],
               alternatives: vec!["alt".to_string()],
           };

           format_split_evidence(&mut output, &evidence);

           assert!(output.contains("Confidence: 65.0%"));
           assert!(output.contains("Low confidence"));
       }
   }
   ```

2. **Integration Test**
   - Capture output before refactoring
   - Compare with output after refactoring
   - Assert byte-for-byte identical

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/priority/formatter_markdown/priority_item.rs`
- **External Dependencies**: None (uses only std::fmt::Write)

## Testing Strategy

- **Unit Tests**: Test each helper function in isolation
- **Integration Tests**: Verify markdown output unchanged
- **Regression Tests**: Run existing test suite
- **Complexity Validation**: Re-run debtmap to verify score reduction

## Documentation Requirements

- **Code Documentation**: Add doc comments to all new helper functions
- **User Documentation**: None required (internal refactoring)
- **Architecture Updates**: None required

## Implementation Notes

1. **Order of Extraction**: Start with `format_split_methods()` since it eliminates duplication, then work bottom-up through the call hierarchy.

2. **Visibility**: All helpers can be `fn` (private) since they're only called within the same module.

3. **String Building Pattern**: Continue using `writeln!(output, ...)` pattern consistent with existing codebase.

4. **Error Handling**: Keep existing `.unwrap()` pattern for `writeln!` since write to String cannot fail.

5. **Testing During Refactor**: After extracting each helper, run tests to ensure no regression before proceeding.

## Migration and Compatibility

No breaking changes. This is a pure internal refactoring that preserves:
- Public API unchanged
- Output format identical
- No behavioral changes

## Success Metrics

| Metric | Before | Target |
|--------|--------|--------|
| Debt Score | 100 | <40 |
| Nesting Depth | 6 | 2 |
| Cyclomatic Complexity | 29 | <8 per function |
| Cognitive Complexity | 99 | <15 per function |
| Lines per Function | 202 | <30 |
| Test Coverage | 0% | >80% |
