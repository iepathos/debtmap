---
number: 203
title: Separate Pure Formatting Logic from I/O Operations
category: refactor
priority: high
status: draft
dependencies: [202]
created: 2025-12-02
---

# Specification 203: Separate Pure Formatting Logic from I/O Operations

**Category**: refactor
**Priority**: high
**Status**: draft
**Dependencies**: Spec 202 (Extract Severity and Coverage Classification)

## Context

Debtmap's formatters violate Stillwater's **"Pure Core, Imperative Shell"** principle by mixing business logic with I/O operations. The `formatter.rs` file contains **151 `writeln!` calls** scattered throughout 3,095 lines, making the code:
- Hard to test (requires string buffers, no mocks possible)
- Impossible to reuse across formats (tied to string output)
- Difficult to reason about (what does this function compute vs. what does it print?)

### Current Anti-Pattern

```rust
// formatter.rs:649 - Passes mutable string through call stack
fn format_mixed_priority_item(
    output: &mut String,  // ← I/O concern leaked into logic
    rank: usize,
    item: &priority::DebtItem,
    verbosity: u8,
    config: FormattingConfig,
    has_coverage_data: bool,
) {
    match item {
        priority::DebtItem::Function(func_item) => {
            verbosity::format_priority_item_with_config(
                output,  // ← Passes I/O through layers
                rank,
                func_item,
                verbosity,
                config,
                has_coverage_data,
            );
        }
        // ... more I/O-coupled logic
    }
}
```

### Stillwater Violation

**Pure Core**: Logic should transform data → data
**Imperative Shell**: I/O should wrap pure logic at boundaries

Current code mixes both, preventing testability and reuse.

## Objective

Separate formatting logic into **pure functions that return data structures**, with I/O operations isolated to thin **writer layers** at system boundaries, following Stillwater's architecture.

## Requirements

### Functional Requirements

1. **Pure Formatting Functions**
   - Return structured data (e.g., `FormattedPriorityItem`)
   - No `&mut String` parameters
   - No side effects (no println!, writeln!, etc.)
   - Signature: `fn format_X(...) -> FormattedX`

2. **Structured Output Types**
   - `FormattedPriorityItem` struct with all display fields
   - `FormattedSection` enum for different section types
   - `FormattedHeader`, `FormattedMetrics`, etc. for subsections
   - All structs are pure data (no behavior, just fields)

3. **Writer Layer**
   - Separate `write_X()` functions for I/O
   - Takes `&mut dyn Write` (not `&mut String`)
   - Signature: `fn write_X(writer: &mut dyn Write, formatted: &FormattedX) -> io::Result<()>`
   - Minimal logic (just rendering, no computation)

4. **Backward Compatibility**
   - Old API preserved with deprecated wrappers
   - Output format unchanged
   - Incremental migration path

### Non-Functional Requirements

1. **Testability**: Pure functions must be 100% testable without I/O
2. **Performance**: Zero-cost abstraction (compile-time optimization)
3. **Maintainability**: Clear separation of concerns
4. **Reusability**: Format logic works for any output medium

## Acceptance Criteria

- [ ] `FormattedPriorityItem` struct defined with all display fields
- [ ] Pure `format_priority_item()` returns `FormattedPriorityItem` (no I/O)
- [ ] `write_priority_item()` takes `&mut dyn Write` and formatted data
- [ ] All tests for formatting logic work without string buffers
- [ ] Property-based tests verify formatting invariants
- [ ] `formatter.rs` has zero `writeln!` calls in pure functions
- [ ] All output formats produce identical results
- [ ] Integration tests verify end-to-end output
- [ ] Old API preserved with `#[deprecated]` wrappers
- [ ] Documentation updated with new architecture

## Technical Details

### Implementation Approach

**Stage 1: Define Structured Output Types**

```rust
// src/priority/formatted_output.rs

/// Pure data structure representing a formatted priority item
#[derive(Debug, Clone, PartialEq)]
pub struct FormattedPriorityItem {
    pub rank: usize,
    pub score: f64,
    pub severity: Severity,
    pub header: FormattedHeader,
    pub sections: Vec<FormattedSection>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormattedHeader {
    pub rank: usize,
    pub score: f64,
    pub severity: Severity,
    pub coverage_tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormattedSection {
    Location {
        file: PathBuf,
        line: u32,
        function: String,
    },
    Impact {
        complexity_reduction: u32,
        risk_reduction: f64,
    },
    Complexity {
        cyclomatic: u32,
        cognitive: u32,
        nesting: u32,
        entropy: Option<f64>,
    },
    Pattern {
        pattern_type: String,
        icon: String,
        metrics: Vec<(String, String)>,
        confidence: f64,
    },
    Coverage {
        percentage: f64,
        level: CoverageLevel,
    },
    Rationale {
        text: String,
    },
    Recommendation {
        action: String,
    },
}
```

**Stage 2: Extract Pure Formatting Functions**

```rust
// src/priority/formatter/pure.rs

/// Pure function: transforms debt item → formatted output
///
/// No I/O operations, fully testable.
pub fn format_priority_item(
    rank: usize,
    item: &UnifiedDebtItem,
    verbosity: u8,
    config: FormattingConfig,
    has_coverage_data: bool,
) -> FormattedPriorityItem {
    let severity = Severity::from_score(item.unified_score.final_score);

    let header = FormattedHeader {
        rank,
        score: item.unified_score.final_score,
        severity,
        coverage_tag: extract_coverage_tag(item, has_coverage_data),
    };

    let sections = vec![
        FormattedSection::Location {
            file: item.location.file.clone(),
            line: item.location.line,
            function: item.location.function.clone(),
        },
        FormattedSection::Impact {
            complexity_reduction: item.expected_impact.complexity_reduction,
            risk_reduction: item.expected_impact.risk_reduction,
        },
        // ... build all sections purely
    ];

    FormattedPriorityItem {
        rank,
        score: item.unified_score.final_score,
        severity,
        header,
        sections,
    }
}

// Pure helper: extracts coverage tag (no I/O)
fn extract_coverage_tag(
    item: &UnifiedDebtItem,
    has_coverage_data: bool,
) -> Option<String> {
    if !has_coverage_data {
        return None;
    }

    item.transitive_coverage.as_ref().map(|cov| {
        let pct = cov.direct * 100.0;
        let level = CoverageLevel::from_percentage(pct);
        format!("{} ({:.1}%)", level.status_tag(), pct)
    })
}
```

**Stage 3: Create Writer Layer**

```rust
// src/priority/formatter/writer.rs

use std::io::{self, Write};

/// I/O layer: renders formatted item to writer
pub fn write_priority_item(
    writer: &mut dyn Write,
    item: &FormattedPriorityItem,
) -> io::Result<()> {
    write_header(writer, &item.header)?;

    for section in &item.sections {
        write_section(writer, section)?;
    }

    Ok(())
}

fn write_header(writer: &mut dyn Write, header: &FormattedHeader) -> io::Result<()> {
    let coverage_tag = header.coverage_tag.as_deref().unwrap_or("");

    writeln!(
        writer,
        "#{} SCORE: {:.1} {} [{}]",
        header.rank,
        header.score,
        coverage_tag,
        header.severity.as_str()
    )?;

    Ok(())
}

fn write_section(writer: &mut dyn Write, section: &FormattedSection) -> io::Result<()> {
    match section {
        FormattedSection::Location { file, line, function } => {
            writeln!(writer, "├─ LOCATION: {}:{} {}()", file.display(), line, function)?;
        }
        FormattedSection::Impact { complexity_reduction, risk_reduction } => {
            writeln!(
                writer,
                "├─ IMPACT: -{} complexity, -{:.1} risk",
                complexity_reduction, risk_reduction
            )?;
        }
        // ... write all sections
        _ => {}
    }
    Ok(())
}
```

**Stage 4: Backward Compatibility Wrapper**

```rust
// src/priority/formatter.rs

/// Deprecated: Use format_priority_item() + write_priority_item()
#[deprecated(
    since = "0.8.0",
    note = "Split into pure format_priority_item() and write_priority_item()"
)]
pub fn format_priority_item_legacy(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    verbosity: u8,
    config: FormattingConfig,
    has_coverage_data: bool,
) {
    use std::io::Write;

    let formatted = pure::format_priority_item(rank, item, verbosity, config, has_coverage_data);

    // Write to string buffer (compatibility layer)
    let mut cursor = std::io::Cursor::new(Vec::new());
    writer::write_priority_item(&mut cursor, &formatted).unwrap();
    output.push_str(&String::from_utf8(cursor.into_inner()).unwrap());
}
```

### Architecture Changes

**Before**:
```
src/priority/
└── formatter.rs
    ├── format_priority_item(output: &mut String, ...) - mixed logic + I/O
    └── 151 writeln! calls scattered throughout
```

**After**:
```
src/priority/
├── formatted_output.rs (data structures)
│   ├── FormattedPriorityItem
│   ├── FormattedSection
│   └── FormattedHeader
├── formatter/
│   ├── mod.rs (public API)
│   ├── pure.rs (pure formatting logic)
│   │   └── format_priority_item(...) -> FormattedPriorityItem
│   └── writer.rs (I/O layer)
│       └── write_priority_item(writer, formatted)
└── formatter.rs (legacy wrapper, deprecated)
```

### Data Flow

```
Input Data (UnifiedDebtItem)
    ↓
Pure Formatting (format_priority_item)
    ↓
Structured Data (FormattedPriorityItem)
    ↓
Writer Layer (write_priority_item)
    ↓
Output (terminal/markdown/html)
```

### APIs and Interfaces

**New Pure API**:

```rust
use debtmap::priority::formatter::pure;
use debtmap::priority::formatted_output::FormattedPriorityItem;

// Pure function: fully testable
let formatted: FormattedPriorityItem = pure::format_priority_item(
    rank,
    item,
    verbosity,
    config,
    has_coverage_data,
);

// Test without I/O
assert_eq!(formatted.rank, 1);
assert_eq!(formatted.severity, Severity::High);
assert!(formatted.sections.iter().any(|s| matches!(s, FormattedSection::Pattern { .. })));
```

**Writer API**:

```rust
use debtmap::priority::formatter::writer;
use std::io::Write;

let formatted = /* ... */;

// Write to terminal
writer::write_priority_item(&mut std::io::stdout(), &formatted)?;

// Write to string
let mut output = Vec::new();
writer::write_priority_item(&mut output, &formatted)?;

// Write to file
let mut file = File::create("output.txt")?;
writer::write_priority_item(&mut file, &formatted)?;
```

## Dependencies

- **Prerequisites**: Spec 202 (Severity/Coverage classification)
- **Affected Components**:
  - `src/priority/formatter.rs` (migrate to pure functions)
  - `src/priority/formatter_verbosity.rs` (migrate to pure functions)
  - `src/priority/formatter_markdown.rs` (migrate to pure functions)
- **External Dependencies**: None

## Testing Strategy

### Unit Tests (Pure Functions)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_priority_item_pure() {
        let item = create_test_item(score: 8.5);

        let formatted = format_priority_item(1, &item, 0, config, false);

        // Test without I/O!
        assert_eq!(formatted.rank, 1);
        assert_eq!(formatted.score, 8.5);
        assert_eq!(formatted.severity, Severity::Critical);
        assert_eq!(formatted.sections.len(), 5);

        // Verify location section
        let location = formatted.sections.iter()
            .find_map(|s| match s {
                FormattedSection::Location { file, line, function } => Some((file, line, function)),
                _ => None,
            })
            .unwrap();

        assert_eq!(location.0, &PathBuf::from("test.rs"));
        assert_eq!(*location.1, 10);
        assert_eq!(location.2, "test_function");
    }

    #[test]
    fn header_includes_coverage_when_available() {
        let item = create_test_item_with_coverage(80.0);

        let formatted = format_priority_item(1, &item, 0, config, true);

        assert!(formatted.header.coverage_tag.is_some());
        assert!(formatted.header.coverage_tag.unwrap().contains("[INFO MODERATE]"));
    }

    #[test]
    fn pattern_section_included_when_detected() {
        let item = create_test_item_with_pattern(PatternType::StateMachine);

        let formatted = format_priority_item(1, &item, 0, config, false);

        let has_pattern = formatted.sections.iter().any(|s| {
            matches!(s, FormattedSection::Pattern { .. })
        });

        assert!(has_pattern);
    }
}
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn formatted_item_always_has_location(
        score in 0.0f64..20.0,
        rank in 1usize..100,
    ) {
        let item = create_test_item_with_score(score);
        let formatted = format_priority_item(rank, &item, 0, config, false);

        // Invariant: every formatted item has a location section
        let has_location = formatted.sections.iter().any(|s| {
            matches!(s, FormattedSection::Location { .. })
        });

        prop_assert!(has_location);
    }

    #[test]
    fn rank_preserved(rank in 1usize..1000) {
        let item = create_test_item();
        let formatted = format_priority_item(rank, &item, 0, config, false);

        prop_assert_eq!(formatted.rank, rank);
    }
}
```

### Integration Tests (Writer Layer)

```rust
#[test]
fn write_produces_expected_output() {
    let formatted = FormattedPriorityItem {
        rank: 1,
        score: 8.5,
        severity: Severity::Critical,
        header: FormattedHeader {
            rank: 1,
            score: 8.5,
            severity: Severity::Critical,
            coverage_tag: None,
        },
        sections: vec![
            FormattedSection::Location {
                file: PathBuf::from("test.rs"),
                line: 10,
                function: "test_fn".to_string(),
            },
        ],
    };

    let mut output = Vec::new();
    write_priority_item(&mut output, &formatted).unwrap();

    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.contains("#1 SCORE: 8.5"));
    assert!(output_str.contains("[CRITICAL]"));
    assert!(output_str.contains("├─ LOCATION: test.rs:10 test_fn()"));
}
```

### Regression Tests

```rust
#[test]
fn output_unchanged_after_refactor() {
    let analysis = load_test_analysis();

    // Expected output (captured before refactor)
    let expected = include_str!("../test_data/expected_output.txt");

    // Generate output using new API
    let items = analysis.get_top_mixed_priorities(10);
    let formatted: Vec<_> = items.iter()
        .enumerate()
        .map(|(i, item)| format_priority_item(i + 1, item, 0, config, false))
        .collect();

    let mut actual = Vec::new();
    for item in &formatted {
        write_priority_item(&mut actual, item).unwrap();
    }

    let actual_str = String::from_utf8(actual).unwrap();

    assert_eq!(actual_str, expected);
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Formats a priority debt item into a pure data structure.
///
/// This is a **pure function** with no side effects:
/// - No I/O operations
/// - No mutation of input data
/// - Deterministic output for same inputs
/// - Fully testable without mocks
///
/// # Examples
///
/// ```
/// use debtmap::priority::formatter::pure;
///
/// let formatted = pure::format_priority_item(
///     1,              // rank
///     &item,          // debt item
///     0,              // verbosity
///     config,         // formatting config
///     false,          // has coverage data
/// );
///
/// assert_eq!(formatted.rank, 1);
/// assert_eq!(formatted.severity, Severity::Critical);
/// ```
///
/// # See Also
///
/// - [`write_priority_item`](crate::priority::formatter::writer::write_priority_item) - Renders to output
pub fn format_priority_item(...) -> FormattedPriorityItem { ... }
```

### User Documentation

Update README.md:

```markdown
## Architecture: Pure Core, Imperative Shell

Debtmap follows functional programming principles:

**Pure Core** (formatting logic):
- `format_priority_item()` - Pure transformation, fully testable
- Returns structured data (`FormattedPriorityItem`)
- No I/O, no side effects

**Imperative Shell** (I/O layer):
- `write_priority_item()` - Renders to terminal/file/string
- Thin layer around pure core
- Handles output formatting only

This separation enables:
- 100% testable formatting logic
- Reusable across output formats
- Easy to reason about
```

### Architecture Updates

Update ARCHITECTURE.md:

```markdown
## Formatting Architecture

### Pure Core (src/priority/formatter/pure.rs)

Pure functions that transform data → data:
- `format_priority_item()` → `FormattedPriorityItem`
- `format_section()` → `FormattedSection`
- No I/O, fully testable

### Imperative Shell (src/priority/formatter/writer.rs)

I/O operations at system boundary:
- `write_priority_item(writer, formatted)` → renders to output
- Minimal logic (just rendering)
- Supports any `Write` implementation

### Data Flow

```
UnifiedDebtItem
    ↓ (pure)
format_priority_item()
    ↓
FormattedPriorityItem
    ↓ (I/O)
write_priority_item()
    ↓
Terminal/File/String
```
```

## Implementation Notes

### Key Principles

1. **Pure functions return data**: Never take `&mut String` parameter
2. **Writer functions perform I/O**: Take `&mut dyn Write`
3. **Structured output types**: Define clear data structures
4. **Test pure functions first**: No I/O mocking needed
5. **Migrate incrementally**: One formatter at a time

### Common Pitfalls

- **Don't** put I/O in pure functions (breaks testability)
- **Don't** put complex logic in writer functions (belongs in pure layer)
- **Do** define clear data structures for formatted output
- **Do** test pure functions exhaustively

### Performance Considerations

- Structured output types are stack-allocated (no heap for small items)
- Writer layer can be inlined (zero-cost abstraction)
- No performance regression vs current implementation

## Migration and Compatibility

### Breaking Changes

None. Old API preserved with deprecated wrappers.

### Migration Path

1. **v0.8.0**: Introduce pure API with deprecated wrappers
2. **v0.8.1**: Migrate internal code to pure API
3. **v0.9.0**: Remove deprecated functions

### Rollback Strategy

If issues discovered:
1. Revert to deprecated wrapper (old behavior)
2. Fix issues in pure layer
3. Re-test and re-deploy

## Success Metrics

- **Test Coverage**: 100% coverage on pure formatting functions
- **I/O Reduction**: Zero `writeln!` in pure functions
- **Testability**: All formatting logic testable without I/O
- **Reusability**: Same logic works for terminal, markdown, HTML
- **Performance**: No measurable regression

## Timeline

- **Week 1**: Define structured output types (8h)
- **Week 2**: Extract pure formatting functions (16h)
- **Week 3**: Create writer layer (8h)
- **Week 4**: Migrate formatter.rs (8h)
- **Week 5**: Migrate formatter_verbosity.rs (8h)
- **Week 6**: Migrate formatter_markdown.rs, remove deprecated code (8h)

**Total Effort**: 56 hours (7 person-days)
