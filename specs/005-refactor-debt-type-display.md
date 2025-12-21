---
number: 5
title: Refactor DebtType Display Implementation
category: optimization
priority: critical
status: draft
dependencies: []
created: 2025-12-21
---

# Specification 5: Refactor DebtType Display Implementation

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: none

## Context

The `fmt` method in `src/priority/mod.rs:575` has critical debt metrics:
- Cyclomatic complexity: 35 (35 match arms)
- Bug density: 100% (every change introduces bugs)
- Coverage: 26%
- Score: 186.16 (highest in codebase)

The function is a simple `Display` impl with a large match statement mapping enum variants to display strings. While the cognitive complexity is low (7), the high cyclomatic complexity and 100% bug density indicate maintenance issues.

Following Stillwater's "Composition Over Complexity" principle, this can be refactored to use a data-driven approach rather than procedural matching.

## Objective

Refactor the `Display` implementation for `DebtType` to:
1. Eliminate the large match statement
2. Reduce bug density by using a single source of truth
3. Improve maintainability for adding new variants
4. Enable better test coverage

## Requirements

### Functional Requirements

1. **Data-driven display mapping**: Create a pure function that maps `DebtType` discriminants to display strings
2. **Single source of truth**: Display names should be defined alongside the enum variants or in a centralized location
3. **Preserve behavior**: All existing display strings must remain unchanged
4. **Handle special cases**: `ErrorSwallowing { pattern, .. }` variant includes dynamic content

### Non-Functional Requirements

1. **No runtime overhead**: Solution should have zero additional cost vs current impl
2. **Compile-time safety**: Adding new variants should cause compile errors if display not defined
3. **Testability**: Each variant's display should be independently testable

## Acceptance Criteria

- [ ] Cyclomatic complexity reduced from 35 to ≤5
- [ ] All existing display strings preserved exactly
- [ ] Test coverage for Display impl reaches ≥80%
- [ ] Adding new DebtType variant without display causes compile error
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean

## Technical Details

### Implementation Approach: Derive Macro with strum

Use the `strum` crate's `Display` derive macro for simple variants:

```rust
use strum::Display;

#[derive(Debug, Clone, Display)]
pub enum DebtType {
    #[strum(serialize = "TODO")]
    Todo { ... },

    #[strum(serialize = "Code Smell")]
    CodeSmell { ... },

    // For dynamic content, use manual Display impl only for that variant
    #[strum(to_string = "Error Swallowing: {pattern}")]
    ErrorSwallowing { pattern: String, ... },
}
```

### Alternative: Display Trait with Helper

If strum is rejected, use a pure helper function:

```rust
impl DebtType {
    /// Pure function: returns display name for this debt type
    const fn display_name(&self) -> &'static str {
        match self {
            Self::Todo { .. } => "TODO",
            Self::CodeSmell { .. } => "Code Smell",
            // ...
        }
    }
}

impl Display for DebtType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ErrorSwallowing { pattern, .. } => {
                write!(f, "Error Swallowing: {}", pattern)
            }
            other => write!(f, "{}", other.display_name()),
        }
    }
}
```

This separates:
- **Pure core**: `display_name()` is a pure function, easy to test
- **Imperative shell**: `fmt()` handles the I/O (writing)

### Data Structures

No new data structures required.

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_todo() {
        let debt = DebtType::Todo { line: 1, message: "fix".into() };
        assert_eq!(debt.to_string(), "TODO");
    }

    #[test]
    fn test_display_error_swallowing_includes_pattern() {
        let debt = DebtType::ErrorSwallowing {
            pattern: "unwrap()".into(),
            line: 1,
        };
        assert_eq!(debt.to_string(), "Error Swallowing: unwrap()");
    }

    // Generate test for each variant to ensure coverage
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: `src/priority/mod.rs`
- **External Dependencies**: Optionally `strum` crate if derive approach chosen

## Testing Strategy

- **Unit Tests**: Test each variant's display output
- **Property Tests**: Ensure all variants have non-empty display
- **Compile-time Tests**: Verify new variants require display definition

## Documentation Requirements

- **Code Documentation**: Add doc comment explaining the display mapping approach
- **User Documentation**: None (internal implementation detail)

## Implementation Notes

The `ErrorSwallowing` variant is the only one with dynamic content (`{pattern}`). All other variants are static strings. The refactoring should handle this special case while simplifying the rest.

Consider using a macro to generate both the enum variants and their display strings from a single definition, ensuring they stay in sync.

## Migration and Compatibility

No breaking changes. Display output is identical to current implementation.
