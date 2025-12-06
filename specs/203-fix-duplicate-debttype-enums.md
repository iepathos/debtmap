---
number: 203
title: Fix Duplicate DebtType Enum Definitions
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-12-06
---

# Specification 203: Fix Duplicate DebtType Enum Definitions

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The codebase currently has **two different definitions** of the `DebtType` enum, causing error swallowing detection to fail silently:

1. **`src/core/mod.rs:219-234`** - Simple unit variants:
   ```rust
   pub enum DebtType {
       Todo,
       Fixme,
       CodeSmell,
       Duplication,
       Complexity,
       Dependency,
       ErrorSwallowing,  // ❌ Unit variant
       // ...
   }
   ```

2. **`src/priority/mod.rs:164-270`** - Rich struct variants with data:
   ```rust
   pub enum DebtType {
       TestingGap { coverage: f64, cyclomatic: u32, cognitive: u32 },
       ComplexityHotspot { cyclomatic: u32, cognitive: u32, ... },
       ErrorSwallowing { pattern: String, context: Option<String> },  // ✓ Struct variant
       // ...
   }
   ```

### The Broken State

The error swallowing detector (`src/debt/error_swallowing.rs:48`) creates debt items using the **core enum**:

```rust
self.items.push(DebtItem {
    debt_type: DebtType::ErrorSwallowing,  // ❌ Uses core::DebtType (unit variant)
    // ...
});
```

But the rest of the system (priority scoring, TUI, output formatting) expects the **priority enum**:

```rust
DebtType::ErrorSwallowing { pattern, context }  // ✓ What's actually needed
```

**Result**: Error swallowing detection creates incompatible `DebtItem`s that:
1. Fail type checking somewhere downstream
2. Get silently filtered out (likely via `.ok()` or pattern matching)
3. Never appear in user output

**Meta-irony**: The error swallowing detector is broken due to a type error that gets... error swallowed.

### Why This Happened

Looking at the code history:
- `src/core/mod.rs` contains a **legacy/deprecated** simple enum
- `src/priority/mod.rs` contains the **current** rich enum with actual data
- Error swallowing detector was written against the wrong enum
- No compilation error because both enums have `ErrorSwallowing` variant
- Runtime type mismatch gets silently swallowed

### Impact

1. **Error swallowing detection completely broken** - 0 results despite ~95 instances in codebase
2. **Silent failure** - No warning/error that detection failed
3. **User confusion** - Feature appears to exist but doesn't work
4. **False confidence** - Users trust output is complete when it's missing entire category

## Objective

Eliminate the duplicate `DebtType` enum definitions and fix error swallowing detection:

1. **Remove legacy enum** from `src/core/mod.rs`
2. **Use rich enum** from `src/priority/mod.rs` everywhere
3. **Fix error swallowing detector** to use correct variant with data
4. **Update all references** throughout codebase
5. **Verify detection works** by analyzing debtmap itself

After this fix, running debtmap on itself should report ~95 error swallowing instances.

## Requirements

### Functional Requirements

1. **Single Source of Truth**
   - Only one `DebtType` enum definition in entire codebase
   - Located in `src/priority/mod.rs` (already the rich version)
   - All modules import from this location

2. **Error Swallowing Detector Fix**
   - Update `src/debt/error_swallowing.rs:48` to use struct variant:
     ```rust
     debt_type: DebtType::ErrorSwallowing {
         pattern: pattern.to_string(),
         context: Some(context.to_string()),
     },
     ```
   - Pass pattern name to variant (e.g., "if let Ok without else")
   - Pass contextual information where available

3. **Import Updates**
   - Replace all `use crate::core::DebtType` with `use crate::priority::DebtType`
   - Or re-export from core if needed for API compatibility
   - Ensure all references resolve to priority enum

4. **Backward Compatibility**
   - If `core::DebtType` is part of public API, create type alias:
     ```rust
     // src/core/mod.rs
     pub use crate::priority::DebtType;
     ```
   - This maintains API compatibility while using single definition

### Non-Functional Requirements

1. **Type Safety**
   - All DebtType uses must type-check correctly
   - No runtime type mismatches
   - Pattern matching must be exhaustive

2. **Verification**
   - Run debtmap on itself after fix
   - Confirm error swallowing results appear in output
   - Verify ~95 instances are detected

3. **No Regressions**
   - All existing tests must pass
   - Other debt detection still works
   - No performance degradation

## Acceptance Criteria

- [ ] `src/core/mod.rs` no longer contains `DebtType` enum definition
- [ ] `DebtType` is only defined in `src/priority/mod.rs`
- [ ] `src/core/mod.rs` re-exports `DebtType` from priority module (if needed for API)
- [ ] All imports of `DebtType` resolve to priority module's definition
- [ ] `src/debt/error_swallowing.rs` uses struct variant with pattern and context
- [ ] Error swallowing detector tests updated to match new variant
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Running debtmap on itself shows error swallowing results
- [ ] Output contains 50+ error swallowing instances (validated against manual count)
- [ ] No clippy warnings
- [ ] No compilation errors or warnings

## Technical Details

### Implementation Approach

**Phase 1: Identify All DebtType References**

```bash
# Find all files using DebtType
rg "DebtType::" --type rust -l > debttype_files.txt

# Find all imports
rg "use.*DebtType" --type rust

# Check for pattern matches
rg "match.*debt_type|DebtType::" --type rust -C 3
```

**Phase 2: Remove Legacy Enum from Core**

Current state (`src/core/mod.rs:219-234`):
```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Ord, PartialOrd)]
pub enum DebtType {
    Todo,
    Fixme,
    CodeSmell,
    Duplication,
    Complexity,
    Dependency,
    ErrorSwallowing,
    ResourceManagement,
    CodeOrganization,
    TestComplexity,
    TestTodo,
    TestDuplication,
    TestQuality,
}

impl std::fmt::Display for DebtType { /* ... */ }
```

Replace with re-export:
```rust
// Re-export from priority module for API compatibility
pub use crate::priority::DebtType;
```

**Phase 3: Fix Error Swallowing Detector**

Current broken code (`src/debt/error_swallowing.rs:34-56`):
```rust
fn add_debt_item(&mut self, line: usize, pattern: ErrorSwallowingPattern, context: &str) {
    // Check suppression...

    let priority = self.determine_priority(&pattern);
    let message = format!("{}: {}", pattern.description(), pattern.remediation());

    self.items.push(DebtItem {
        id: format!("error-swallow-{}-{}", self.current_file.display(), line),
        debt_type: DebtType::ErrorSwallowing,  // ❌ Wrong variant
        priority,
        file: self.current_file.to_path_buf(),
        line,
        column: None,
        message,
        context: Some(context.to_string()),
    });
}
```

Fixed code:
```rust
fn add_debt_item(&mut self, line: usize, pattern: ErrorSwallowingPattern, context: &str) {
    // Check suppression...

    let priority = self.determine_priority(&pattern);
    let message = format!("{}: {}", pattern.description(), pattern.remediation());

    self.items.push(DebtItem {
        id: format!("error-swallow-{}-{}", self.current_file.display(), line),
        debt_type: DebtType::ErrorSwallowing {
            pattern: pattern.to_string(),  // ✓ Pattern name
            context: Some(context.to_string()),  // ✓ Contextual info
        },
        priority,
        file: self.current_file.to_path_buf(),
        line,
        column: None,
        message,
        context: Some(context.to_string()),
    });
}
```

**Phase 4: Add Display Implementation for ErrorSwallowingPattern**

```rust
impl std::fmt::Display for ErrorSwallowingPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}
```

**Phase 5: Update Imports Throughout Codebase**

Replace:
```rust
use crate::core::DebtType;
```

With:
```rust
use crate::priority::DebtType;
```

Or if core is preferred import location:
```rust
use crate::core::DebtType;  // Now re-exports from priority
```

**Phase 6: Update Suppression Checker**

The suppression context checker likely expects simple variants. Update if needed:

```rust
// src/debt/suppression.rs (if applicable)
pub fn is_suppressed(&self, line: usize, debt_type: &DebtType) -> bool {
    // Match on the variant, ignoring data
    let type_matches = match (debt_type, &self.suppressed_type) {
        (DebtType::ErrorSwallowing { .. }, DebtType::ErrorSwallowing { .. }) => true,
        (DebtType::Complexity { .. }, DebtType::Complexity { .. }) => true,
        // ... other variants
        _ => debt_type == self.suppressed_type,
    };

    type_matches && self.line_range.contains(&line)
}
```

**Phase 7: Update Display Implementation in Priority Module**

The priority module's `DebtType` may need `Display` impl if missing:

```rust
// src/priority/mod.rs
impl std::fmt::Display for DebtType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebtType::ErrorSwallowing { pattern, .. } => write!(f, "Error Swallowing: {}", pattern),
            DebtType::TestingGap { .. } => write!(f, "Testing Gap"),
            DebtType::ComplexityHotspot { .. } => write!(f, "Complexity Hotspot"),
            // ... other variants
        }
    }
}
```

### Architecture Changes

**Before (Broken):**
```
src/core/mod.rs
  └─ DebtType enum (simple, unit variants)  ❌ Legacy

src/priority/mod.rs
  └─ DebtType enum (rich, struct variants)  ✓ Current

src/debt/error_swallowing.rs
  └─ Uses core::DebtType  ❌ Wrong type
  └─ Creates incompatible DebtItems
  └─ Silently filtered out downstream

Output: 0 error swallowing results
```

**After (Fixed):**
```
src/core/mod.rs
  └─ pub use priority::DebtType;  ✓ Re-export

src/priority/mod.rs
  └─ DebtType enum (single source of truth)  ✓ Only definition

src/debt/error_swallowing.rs
  └─ Uses priority::DebtType  ✓ Correct type
  └─ Creates compatible DebtItems with pattern data
  └─ Flows through pipeline successfully

Output: ~95 error swallowing results  ✓ Detection works!
```

### Data Structures

**ErrorSwallowingPattern Enhancement**

Add `Display` trait:
```rust
impl std::fmt::Display for ErrorSwallowingPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}
```

This enables:
```rust
debt_type: DebtType::ErrorSwallowing {
    pattern: pattern.to_string(),  // Converts ErrorSwallowingPattern to String
    context: Some(context.to_string()),
}
```

### APIs and Interfaces

**Public API Compatibility**

If `core::DebtType` is part of public API:

```rust
// src/core/mod.rs - Maintain backward compatibility
pub use crate::priority::DebtType;

// All existing code using core::DebtType continues to work
// But now uses the correct definition
```

**Internal Consistency**

All internal modules should import from priority:
```rust
use crate::priority::DebtType;
```

## Dependencies

- **Prerequisites**: None (bug fix)
- **Affected Components**:
  - `src/core/mod.rs` - Remove enum, add re-export
  - `src/priority/mod.rs` - Add Display impl if missing
  - `src/debt/error_swallowing.rs` - Fix variant usage
  - `src/debt/suppression.rs` - Update matching logic (if applicable)
  - All files importing `DebtType` - Verify correct import
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

**Update Error Swallowing Tests**

```rust
// src/debt/error_swallowing.rs tests need updating
#[test]
fn test_if_let_ok_no_else() {
    let code = r#"
        fn example() {
            if let Ok(value) = some_function() {
                println!("{}", value);
            }
        }
    "#;

    let file = parse_str::<File>(code).expect("Failed to parse test code");
    let items = detect_error_swallowing(&file, Path::new("test.rs"), None);

    assert_eq!(items.len(), 1);

    // Update assertion to match struct variant
    match &items[0].debt_type {
        DebtType::ErrorSwallowing { pattern, context } => {
            assert!(pattern.contains("if let Ok"));
            assert!(context.is_some());
        }
        _ => panic!("Expected ErrorSwallowing variant"),
    }

    assert!(items[0].message.contains("if let Ok"));
    assert_eq!(items[0].line, 3);
}
```

**Add Verification Tests**

```rust
#[test]
fn test_debttype_is_struct_variant() {
    // Ensure we're using the rich enum, not simple enum
    let detector = ErrorSwallowingDetector::new(Path::new("test.rs"), None);
    let pattern = ErrorSwallowingPattern::OkMethodDiscard;

    detector.add_debt_item(10, pattern, "test context");

    let items = detector.items;
    assert_eq!(items.len(), 1);

    // This should compile and match
    match &items[0].debt_type {
        DebtType::ErrorSwallowing { pattern, context } => {
            assert_eq!(pattern, ".ok() discarding error information");
            assert_eq!(context.as_deref(), Some("test context"));
        }
        _ => panic!("Wrong DebtType variant"),
    }
}
```

### Integration Tests

**Verify Error Swallowing Detection Works**

```rust
#[test]
fn test_error_swallowing_detection_integration() {
    // Create a temp file with error swallowing patterns
    let test_code = r#"
        fn has_error_swallowing() {
            let result = risky_operation();
            result.ok();  // Should be detected

            if let Ok(value) = another_operation() {
                // Missing error handling - should be detected
                println!("{}", value);
            }

            let _ = yet_another_operation();  // Should be detected
        }
    "#;

    let temp_file = create_temp_file("test.rs", test_code);
    let results = analyze_file(&temp_file).unwrap();

    // Should detect 3 error swallowing instances
    let error_swallowing_count = results.debt_items.iter()
        .filter(|item| matches!(item.debt_type, DebtType::ErrorSwallowing { .. }))
        .count();

    assert_eq!(error_swallowing_count, 3);
}
```

### Self-Analysis Test

```rust
#[test]
fn test_debtmap_self_analysis_finds_error_swallowing() {
    // Run debtmap on its own codebase
    let config = Config::default();
    let results = analyze_project(".", &config).unwrap();

    // Should find substantial error swallowing instances
    let error_swallowing_count = results.all_debt_items()
        .filter(|item| matches!(item.debt_type, DebtType::ErrorSwallowing { .. }))
        .count();

    // Based on manual analysis, we expect ~95 instances
    assert!(
        error_swallowing_count >= 50,
        "Expected at least 50 error swallowing instances, found {}",
        error_swallowing_count
    );
}
```

### Manual Verification

After implementation:

```bash
# Run debtmap on itself
cargo run -- analyze . --output-format json > self_analysis.json

# Check for error swallowing results
jq '.debt_items[] | select(.debt_type.ErrorSwallowing)' self_analysis.json | wc -l

# Should show 50+ instances
```

## Documentation Requirements

### Code Documentation

```rust
/// Detects error swallowing patterns in Rust code.
///
/// Creates DebtItems with the ErrorSwallowing variant, including:
/// - `pattern`: Name of the error swallowing pattern detected
/// - `context`: Additional context about where/why it occurred
///
/// # Returns
///
/// Vector of DebtItems with ErrorSwallowing debt_type.
///
/// # Examples
///
/// ```
/// let file = parse_rust_file("src/main.rs")?;
/// let items = detect_error_swallowing(&file, Path::new("src/main.rs"), None);
///
/// for item in items {
///     match item.debt_type {
///         DebtType::ErrorSwallowing { pattern, context } => {
///             println!("Found {}: {:?}", pattern, context);
///         }
///         _ => unreachable!(),
///     }
/// }
/// ```
pub fn detect_error_swallowing(
    file: &File,
    file_path: &Path,
    suppression: Option<&SuppressionContext>,
) -> Vec<DebtItem> {
    // ...
}
```

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
## DebtType Enum

The `DebtType` enum is the single source of truth for all technical debt categories.

### Location

- **Defined in**: `src/priority/mod.rs`
- **Re-exported from**: `src/core/mod.rs` (for API compatibility)

### Design

`DebtType` uses **struct variants with data** to capture rich information:

```rust
pub enum DebtType {
    ErrorSwallowing {
        pattern: String,        // Pattern name (e.g., ".ok() discarding error")
        context: Option<String>, // Additional context
    },
    ComplexityHotspot {
        cyclomatic: u32,
        cognitive: u32,
        adjusted_cyclomatic: Option<u32>,
    },
    // ... other variants
}
```

### Historical Context

Previously, debtmap had **two different** DebtType enums:
- `src/core/mod.rs` - Simple unit variants (legacy, **removed in spec 203**)
- `src/priority/mod.rs` - Rich struct variants (current)

This duplication caused error swallowing detection to fail silently due to type mismatches.

### Usage

Always import from core (which re-exports from priority):

```rust
use crate::core::DebtType;

// Or directly from priority:
use crate::priority::DebtType;
```

When creating DebtItems, use struct variants with data:

```rust
DebtItem {
    debt_type: DebtType::ErrorSwallowing {
        pattern: "if let Ok without else".to_string(),
        context: Some("Missing error handling".to_string()),
    },
    // ...
}
```
```

## Implementation Notes

### Refactoring Steps

1. **Verify current state**
   ```bash
   rg "pub enum DebtType" --type rust
   # Should show two definitions - core and priority
   ```

2. **Update error swallowing detector first**
   - Easier to test in isolation
   - Validates the approach

3. **Remove core enum definition**
   - Replace with re-export
   - Maintain API compatibility

4. **Run tests incrementally**
   - After each change, run tests
   - Fix failures immediately

5. **Self-analyze**
   - Run debtmap on itself
   - Verify results appear

### Common Pitfalls

1. **Breaking Public API** - Use re-export to maintain compatibility
2. **Forgetting Display impl** - Add `Display` for `ErrorSwallowingPattern`
3. **Pattern matching** - Update all match expressions to handle struct variants
4. **Suppression logic** - Update to compare variant types, not full equality

### Verification Checklist

After implementation:

- [ ] Only one `pub enum DebtType` in codebase
- [ ] All files compile without errors
- [ ] All tests pass
- [ ] `cargo clippy` shows no warnings
- [ ] Self-analysis shows error swallowing results
- [ ] Count matches expected ~95 instances

## Migration and Compatibility

### Breaking Changes

**None** - Public API maintained through re-export.

### Migration Steps

**For External Users** (if debtmap is a library):

No migration needed. `core::DebtType` still available via re-export.

**For Internal Code**:

1. Update imports if needed (re-export handles most cases)
2. Update pattern matching to handle struct variants
3. Update equality comparisons to match on variant type

### Compatibility Considerations

**Pattern Matching**:

Old code (won't compile with struct variant):
```rust
if debt_item.debt_type == DebtType::ErrorSwallowing {
    // ...
}
```

New code:
```rust
if matches!(debt_item.debt_type, DebtType::ErrorSwallowing { .. }) {
    // ...
}
```

## Success Metrics

- ✅ Single `DebtType` enum definition
- ✅ All tests pass
- ✅ Error swallowing detection produces results
- ✅ Self-analysis finds 50+ error swallowing instances
- ✅ No compilation errors or warnings
- ✅ No clippy warnings
- ✅ API compatibility maintained

## Follow-up Work

After this fix:
- Spec 202: Improve error swallowing detection patterns (add `.filter_map(Result::ok)`)
- Spec 203: Add error collection and reporting for batch operations
- Review other debt detectors for similar type issues

## References

- **Meta-issue discovery**: 2025-12-06 debugging session
- **Root cause**: Duplicate enum definitions causing silent type mismatch
- **Manual analysis**: ~95 error swallowing instances found via grep
- **Stillwater PHILOSOPHY.md**: "Errors Should Tell Stories" - detection failures should never be silent
