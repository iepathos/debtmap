---
number: 254
title: Universal Responsibility Analysis for All Debt Items
category: foundation
priority: medium
status: draft
dependencies: [253]
created: 2025-12-07
---

# Specification 254: Universal Responsibility Analysis for All Debt Items

**Category**: foundation
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 253 (unified god object types)

## Context

Currently, responsibility analysis is only available for god objects (structs/classes with multiple methods). The TUI dependencies page shows a detailed "responsibilities" section displaying behavioral categories like "Data Access", "Validation", "Persistence" - but **only for god objects**.

This creates an inconsistent user experience:
- **God Objects**: Show detailed responsibilities (e.g., "Data Access: 12 methods", "Validation: 8 methods")
- **God Modules**: Have responsibility data in `god_object_indicators` but TUI doesn't display it
- **Regular Functions**: No responsibility analysis at all - just classified as generic "Complexity" debt

### Current Implementation

Responsibility inference exists and works well:
- **Location**: `src/organization/behavioral_decomposition/categorization.rs`
- **Core Function**: `infer_responsibility_with_confidence(method: &str) -> ResponsibilityClassification`
- **Used By**: `group_methods_by_responsibility()` for god object analysis
- **Categories**: 15+ behavioral categories (Parsing, Rendering, Validation, Data Access, etc.)

The infrastructure is **proven and tested** - we just need to make it available uniformly.

### Problem Statement

Users analyzing complex codebases want to understand what each function **does** (its responsibility), not just that it's complex. Current limitations:

1. **Missing Context**: A function with 20 cyclomatic complexity could be:
   - Complex validation logic (expected, acceptable)
   - Complex parsing logic (technical debt, should extract)
   - Complex rendering code (UI concern, different priority)

2. **Inconsistent Display**: TUI shows responsibilities for god objects but not individual functions

3. **Lost Opportunity**: Existing behavioral categorization (`BehavioralCategorizer`) could classify ALL functions but only runs for god objects

### User Impact

When reviewing debt items, users want to know:
- "Is this complex parsing that could be simplified?"
- "Is this validation logic that should be extracted to its own module?"
- "Is this data access code that violates separation of concerns?"

Currently, they have to manually inspect function names to infer responsibility.

## Objective

Extend responsibility analysis to **all debt items** (regular functions, god objects, god modules) using existing behavioral categorization infrastructure. Display the primary responsibility category uniformly in the TUI, providing users with immediate context about what each debt item does.

**Core Principle**: Follow Stillwater philosophy of **Pure Core, Imperative Shell**
- **Pure Core**: Responsibility inference during analysis phase (deterministic, testable)
- **Imperative Shell**: TUI/output displays pre-computed results (no business logic)

## Requirements

### Functional Requirements

1. **Universal Analysis**
   - All `UnifiedDebtItem` instances should have an optional `responsibility_category: Option<String>`
   - Category determined during analysis phase using existing `infer_responsibility_with_confidence()`
   - Returns `None` if confidence < 0.7 (avoiding low-confidence guesses)

2. **Reuse Existing Infrastructure**
   - **Must reuse**: `infer_responsibility_with_confidence()` (src/organization/behavioral_decomposition/categorization.rs)
   - **Must reuse**: `BehavioralCategorizer` predicates (is_parsing, is_validation, etc.)
   - **Must reuse**: Existing 15+ behavioral categories
   - **No duplication**: Don't recreate pattern matching or inference logic

3. **Consistent Data Model**
   - God objects: Keep existing detailed `responsibilities: Vec<String>` in `GodObjectAnalysis`
   - All items: Add new `responsibility_category: Option<String>` for primary/single responsibility
   - God objects populate both (detailed list + primary category)

4. **TUI Display**
   - Dependencies page: Show `responsibility_category` for ALL items
   - God objects: Keep existing detailed responsibilities breakdown
   - Regular functions: Show single primary responsibility
   - Format: `"primary responsibility: Data Access"` or `"responsibility: Validation"`

### Non-Functional Requirements

1. **Performance**
   - Responsibility inference adds <1ms per function (already proven in god object analysis)
   - No runtime computation in TUI (pre-computed during analysis)
   - Memory overhead: ~24 bytes per item (single String in Option)

2. **Functional Purity**
   - `analyze_function_responsibility(name: &str) -> Option<String>` must be pure
   - Same input always produces same output (deterministic)
   - No side effects, no I/O, no hidden state

3. **Backwards Compatibility**
   - Optional field (`Option<String>`) - existing code unaffected
   - Serialization: Use `#[serde(skip_serializing_if = "Option::is_none")]`
   - TUI: Gracefully handle None (just don't display)

4. **Code Quality**
   - Follow Debtmap functional programming principles
   - Pure functions separated from I/O
   - Single responsibility per function
   - Comprehensive unit tests for pure functions

## Acceptance Criteria

- [ ] **Data Structure**: `UnifiedDebtItem` has `responsibility_category: Option<String>` field
- [ ] **Pure Function**: `analyze_function_responsibility(name: &str) -> Option<String>` exists and is pure
- [ ] **Reuse**: Function calls existing `infer_responsibility_with_confidence()` with confidence threshold
- [ ] **Analysis Integration**: Builders populate `responsibility_category` for all debt items
- [ ] **God Object Compatibility**: God objects have both detailed responsibilities AND primary category
- [ ] **TUI Display**: Dependencies page shows responsibility for all items (not just god objects)
- [ ] **God Module Fix**: God modules display responsibilities (blocked by spec 253)
- [ ] **Unit Tests**: Pure function has comprehensive tests (15+ behavioral categories)
- [ ] **Integration Tests**: TUI displays responsibilities correctly for all debt types
- [ ] **Performance**: No measurable performance impact (<5% analysis time increase)
- [ ] **Documentation**: Update architecture docs to reflect universal responsibility analysis

## Technical Details

### Implementation Approach

#### 1. Data Structure Change

**File**: `src/priority/unified_scorer.rs`

```rust
pub struct UnifiedDebtItem {
    // ... existing fields

    /// Primary responsibility category for this function/module
    /// Derived from behavioral analysis of function name
    /// Examples: "Data Access", "Validation", "Parsing", "Rendering"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub responsibility_category: Option<String>,

    // ... rest of fields
}
```

#### 2. Pure Analysis Function

**File**: `src/organization/behavioral_decomposition/analysis.rs` (or new file)

```rust
/// Pure function: analyzes function name and returns primary responsibility category.
///
/// Reuses existing behavioral categorization infrastructure to provide uniform
/// responsibility analysis across all debt items (not just god objects).
///
/// # Arguments
/// * `function_name` - Name of the function to analyze
///
/// # Returns
/// * `Some(category)` - If behavioral category can be inferred with high confidence (>= 0.7)
/// * `None` - If function name doesn't clearly indicate a behavioral pattern
///
/// # Examples
/// ```
/// assert_eq!(analyze_function_responsibility("validate_email"), Some("Validation".to_string()));
/// assert_eq!(analyze_function_responsibility("parse_json"), Some("Parsing".to_string()));
/// assert_eq!(analyze_function_responsibility("get_user"), Some("Data Access".to_string()));
/// assert_eq!(analyze_function_responsibility("process_item"), None); // Low confidence
/// ```
///
/// # Stillwater Principle: Pure Core
/// This function is pure - same input always gives same output, no side effects.
/// Responsibility inference happens once during analysis, not during rendering.
pub fn analyze_function_responsibility(function_name: &str) -> Option<String> {
    use super::categorization::infer_responsibility_with_confidence;

    // Reuse existing inference with confidence threshold
    let result = infer_responsibility_with_confidence(function_name, None);

    // Only return category if confidence meets threshold
    // Threshold of 0.7 matches existing god object analysis patterns
    if result.confidence >= 0.7 {
        result.category
    } else {
        None
    }
}
```

#### 3. Builder Integration

**File**: `src/builders/unified_analysis.rs` and `src/builders/parallel_unified_analysis.rs`

```rust
use crate::organization::behavioral_decomposition::analyze_function_responsibility;

// When creating UnifiedDebtItem for ANY function:
fn create_debt_item(/* ... */) -> UnifiedDebtItem {
    let responsibility_category = analyze_function_responsibility(&function_name);

    UnifiedDebtItem {
        location,
        debt_type,
        // ... other fields
        responsibility_category,
        // ... rest of fields
    }
}

// For god objects specifically:
fn create_god_object_item(/* ... */) -> UnifiedDebtItem {
    // Extract primary responsibility from existing Vec<String>
    let responsibility_category = god_object_indicators
        .responsibilities
        .first()
        .cloned();

    UnifiedDebtItem {
        // ... fields
        responsibility_category,
        god_object_indicators: Some(god_object_indicators), // Keep detailed breakdown
        // ... rest
    }
}
```

#### 4. TUI Display Enhancement

**File**: `src/tui/results/detail_pages/dependencies.rs`

```rust
pub fn render(/* ... */) {
    let mut lines = Vec::new();

    // Dependency Metrics section (existing code)
    add_section_header(&mut lines, "dependency metrics", theme);
    // ... existing dependency display

    // NEW: Primary Responsibility (for ALL items)
    if let Some(category) = &item.responsibility_category {
        lines.push(ratatui::text::Line::from(""));
        add_label_value(
            &mut lines,
            "primary responsibility",
            category.clone(),
            theme,
            area.width
        );
    }

    // EXISTING: Detailed responsibilities (for god objects only)
    if let Some(indicators) = &item.god_object_indicators {
        if indicators.is_god_object && !indicators.responsibilities.is_empty() {
            lines.push(ratatui::text::Line::from(""));
            add_section_header(&mut lines, "responsibilities", theme);

            // Display detailed breakdown (existing code)
            for resp in indicators.responsibilities.iter().take(6) {
                // ... existing display logic
            }
        }
    }

    // ... rest of function
}
```

### Architecture Changes

#### Before (Current State)
```
Analysis Phase:
  God Objects → group_methods_by_responsibility() → Vec<String>
  Regular Functions → (no responsibility analysis)

Display Phase:
  God Objects → Show responsibilities from god_object_indicators
  Regular Functions → Show nothing
```

#### After (This Spec)
```
Analysis Phase (Pure Core):
  God Objects → group_methods_by_responsibility() → Vec<String> (detailed)
              → analyze_function_responsibility() → Option<String> (primary)

  Regular Functions → analyze_function_responsibility() → Option<String>

Display Phase (Imperative Shell):
  All Items → Show responsibility_category if present
  God Objects → ALSO show detailed breakdown from god_object_indicators
```

### Data Flow

```
Function Name
    ↓
analyze_function_responsibility() [Pure]
    ↓
infer_responsibility_with_confidence() [Existing, Pure]
    ↓
BehavioralCategorizer.categorize_method() [Existing, Pure]
    ↓
ResponsibilityClassification { category: Option<String>, confidence: f64 }
    ↓
Filter by confidence >= 0.7
    ↓
Option<String> stored in UnifiedDebtItem.responsibility_category
    ↓
TUI reads and displays (Imperative)
```

### Error Handling

1. **Low Confidence Names**: Return `None` - don't force a category
2. **Invalid Input**: Empty string or whitespace → `None`
3. **Serialization**: `Option` handles missing field gracefully
4. **Display**: TUI checks `if let Some(category)` before rendering

## Dependencies

### Prerequisites
- **Spec 253**: Unifying god object types will simplify god module responsibility display

### Affected Components
- `src/priority/unified_scorer.rs` - Add field to `UnifiedDebtItem`
- `src/organization/behavioral_decomposition/` - Add `analyze_function_responsibility()`
- `src/builders/unified_analysis.rs` - Populate field during analysis
- `src/builders/parallel_unified_analysis.rs` - Populate field during parallel analysis
- `src/tui/results/detail_pages/dependencies.rs` - Display responsibility for all items

### External Dependencies
None - uses existing infrastructure

## Testing Strategy

### Unit Tests

**File**: `src/organization/behavioral_decomposition/tests.rs` or inline

```rust
#[cfg(test)]
mod responsibility_analysis_tests {
    use super::*;

    #[test]
    fn test_analyze_function_responsibility_parsing() {
        assert_eq!(
            analyze_function_responsibility("parse_json"),
            Some("Parsing".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("read_config"),
            Some("Parsing".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_validation() {
        assert_eq!(
            analyze_function_responsibility("validate_email"),
            Some("Validation".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("check_bounds"),
            Some("Validation".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_data_access() {
        assert_eq!(
            analyze_function_responsibility("get_user"),
            Some("Data Access".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("set_property"),
            Some("Data Access".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_rendering() {
        assert_eq!(
            analyze_function_responsibility("render_view"),
            Some("Rendering".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("draw_chart"),
            Some("Rendering".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_low_confidence() {
        // Generic names should return None
        assert_eq!(analyze_function_responsibility("process"), None);
        assert_eq!(analyze_function_responsibility("handle"), None);
        assert_eq!(analyze_function_responsibility("do_something"), None);
    }

    #[test]
    fn test_analyze_function_responsibility_purity() {
        // Pure function: same input = same output
        let result1 = analyze_function_responsibility("validate_input");
        let result2 = analyze_function_responsibility("validate_input");
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_analyze_function_responsibility_all_categories() {
        // Test coverage for all 15+ behavioral categories
        let test_cases = vec![
            ("new_instance", Some("Construction")),
            ("parse_xml", Some("Parsing")),
            ("filter_results", Some("Filtering")),
            ("transform_data", Some("Transformation")),
            ("send_message", Some("Communication")),
            ("save_user", Some("Persistence")),
            ("handle_click", Some("Event Handling")),
            // ... all categories
        ];

        for (input, expected) in test_cases {
            assert_eq!(
                analyze_function_responsibility(input),
                expected.map(String::from),
                "Failed for input: {}",
                input
            );
        }
    }
}
```

### Integration Tests

**File**: `tests/tui_integration_test.rs` or new test file

```rust
#[test]
fn test_responsibility_display_for_all_items() {
    // Create debt items with different types
    let items = vec![
        create_complex_function("validate_email", 20, 15),
        create_complex_function("parse_json", 25, 20),
        create_god_object_item(/* ... */),
    ];

    // Verify all items have responsibility_category populated
    for item in &items {
        assert!(
            item.responsibility_category.is_some(),
            "Item {} should have responsibility category",
            item.location.function_name
        );
    }

    // Verify TUI renders responsibilities
    // (Test rendering logic)
}
```

### Performance Tests

**File**: `benches/responsibility_analysis_bench.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_responsibility_analysis(c: &mut Criterion) {
    let function_names = vec![
        "validate_email",
        "parse_json",
        "get_user",
        "render_view",
        // ... 100+ function names
    ];

    c.bench_function("analyze_function_responsibility", |b| {
        b.iter(|| {
            for name in &function_names {
                black_box(analyze_function_responsibility(name));
            }
        })
    });
}

criterion_group!(benches, benchmark_responsibility_analysis);
criterion_main!(benches);
```

### User Acceptance Testing

1. **Manual TUI Testing**
   - Run debtmap on a real codebase
   - Navigate to dependencies page for various debt items
   - Verify responsibilities display correctly
   - Check for god objects, god modules, regular functions

2. **Visual Regression Testing**
   - Compare TUI screenshots before/after
   - Verify consistent formatting
   - Check for layout issues

## Documentation Requirements

### Code Documentation

1. **Inline Documentation**
   - Document `analyze_function_responsibility()` with examples
   - Add doc comments to `responsibility_category` field
   - Explain confidence threshold rationale

2. **Module Documentation**
   - Update `behavioral_decomposition/mod.rs` documentation
   - Explain pure function guarantees
   - Document category list and confidence thresholds

### User Documentation

**File**: `book/src/responsibility-analysis.md` (update existing)

Add section:
```markdown
## Universal Responsibility Analysis

Debtmap analyzes the behavioral responsibility of **all functions**, not just god objects.

### What is Responsibility Analysis?

Function names often reveal their primary purpose:
- `validate_email` → **Validation**
- `parse_json` → **Parsing**
- `get_user` → **Data Access**
- `render_view` → **Rendering**

### Viewing Responsibilities

In the TUI dependencies page, you'll see:
- **Primary Responsibility**: The main behavioral category for this function
- **Responsibilities** (god objects only): Detailed breakdown of multiple responsibilities

### Behavioral Categories

Debtmap recognizes 15+ behavioral patterns:
- Construction (new, create, build)
- Parsing (parse, read, extract)
- Validation (validate, check, verify)
- Data Access (get, set, fetch)
- Rendering (render, draw, display)
- ... (full list)

### Confidence Threshold

Only functions with clear behavioral patterns (confidence >= 70%) show a responsibility.
Generic names like "process" or "handle" won't have a responsibility category.
```

### Architecture Updates

**File**: `ARCHITECTURE.md`

Add section under "Behavioral Decomposition":
```markdown
### Universal Responsibility Analysis

All `UnifiedDebtItem` instances include optional `responsibility_category` field
populated during analysis phase using pure function composition.

**Data Flow:**
1. Analysis: `function_name` → `analyze_function_responsibility()` → `Option<String>`
2. Storage: Result stored in `UnifiedDebtItem.responsibility_category`
3. Display: TUI reads pre-computed value (no runtime computation)

**Stillwater Principle:**
- Pure Core: Responsibility inference (deterministic, testable)
- Imperative Shell: TUI display (I/O only, no logic)
```

## Implementation Notes

### Best Practices

1. **Confidence Threshold**
   - Use 0.7 (70%) to match existing god object analysis patterns
   - Document rationale in code comments
   - Consider making configurable in future specs

2. **God Object Handling**
   - Populate both `responsibility_category` (primary) and `god_object_indicators.responsibilities` (detailed)
   - Use first item from detailed list as primary
   - Maintain backwards compatibility

3. **Empty/Whitespace Names**
   - Return `None` for empty strings or whitespace-only names
   - Add defensive check at function entry

4. **Performance Optimization**
   - Responsibility inference is already fast (<1ms per function)
   - Pre-compute during analysis, never during rendering
   - No caching needed (pure function already optimal)

### Common Pitfalls

1. **Don't compute in TUI**
   - ❌ `let resp = infer_responsibility(&item.location.function_name);`
   - ✅ `if let Some(resp) = &item.responsibility_category { ... }`

2. **Don't ignore confidence**
   - ❌ `result.category.unwrap_or("Unknown")`
   - ✅ `if result.confidence >= 0.7 { result.category } else { None }`

3. **Don't duplicate logic**
   - ❌ Create new pattern matching for function names
   - ✅ Reuse `infer_responsibility_with_confidence()`

### Future Enhancements (Out of Scope)

- Configurable confidence threshold (spec 255?)
- Context-aware inference using AST (spec 256?)
- Multiple responsibility support for complex functions (spec 257?)
- Responsibility-based filtering/grouping in TUI (spec 258?)

## Migration and Compatibility

### Breaking Changes
None - this is a purely additive change.

### Migration Path
1. Add optional field to `UnifiedDebtItem` (backwards compatible)
2. Populate field during analysis (new items get it, old items have None)
3. TUI checks for presence before displaying (graceful degradation)

### Compatibility Considerations

1. **Serialization**
   - `#[serde(skip_serializing_if = "Option::is_none")]` ensures old JSON files still parse
   - New field omitted if None, preserving old format

2. **Existing Code**
   - All existing `UnifiedDebtItem` creation continues to work
   - Field defaults to None if not specified

3. **TUI Display**
   - Existing layouts unchanged
   - New responsibility display adds context without breaking layout

### Rollback Strategy

If issues arise:
1. Remove `responsibility_category` field population in builders
2. Remove TUI display code
3. Keep field in struct (set to None) to maintain compatibility

Rollback is safe because field is optional and display is additive.

## Success Metrics

### Quantitative Metrics
- **Coverage**: >95% of functions have responsibility category (when confidence high)
- **Performance**: <5% increase in analysis time
- **Memory**: <2% increase in total memory usage
- **Accuracy**: >90% agreement with manual categorization (spot check 100 functions)

### Qualitative Metrics
- Users can immediately understand function purpose from TUI
- Reduced need to inspect code to understand complexity context
- Improved prioritization decisions based on responsibility

### Validation Criteria
- All behavioral categories covered in tests
- TUI displays consistently across debt types
- No performance regressions
- Documentation is clear and complete

## Appendix: Behavioral Categories Reference

Complete list of categories recognized by `BehavioralCategorizer`:

1. **Construction** - Creating instances (new, create, build, make)
2. **Parsing** - Reading/extracting data (parse, read, extract, decode)
3. **Validation** - Checking correctness (validate, check, verify, is_valid)
4. **Data Access** - Getting/setting data (get, set, fetch, retrieve)
5. **Rendering** - Display/output (render, draw, paint, display, format, to_string)
6. **Filtering** - Selecting/searching (filter, select, find, search, query)
7. **Transformation** - Converting data (transform, convert, map, adapt)
8. **Communication** - Sending/receiving (send, receive, transmit, broadcast)
9. **Persistence** - Saving/loading (save, load, persist, serialize)
10. **Event Handling** - Responding to events (handle, on_, dispatch)
11. **Lifecycle** - Initialization/cleanup (initialize, setup, cleanup, shutdown)
12. **State Management** - Managing state (_state, update_)
13. **Processing** - Executing logic (process, execute, run)
14. **Domain** - Business logic (context-specific patterns)
15. **Utilities** - Helper functions (generic operations)

See `src/organization/behavioral_decomposition/categorization.rs` for complete patterns.
