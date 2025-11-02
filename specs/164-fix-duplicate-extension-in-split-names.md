---
number: 164
title: Fix Duplicate Extension in Split Names
category: optimization
priority: high
status: draft
dependencies: [140]
created: 2025-11-02
---

# Specification 164: Fix Duplicate Extension in Split Names

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 140 (Domain-Based Struct Split Recommendations)

## Context

The formatter currently appends file extensions to `suggested_name` values, but some split name generators already include `.rs` in the suggested name. This results in double extensions like `config/misc.rs.rs` in the output.

**Current Output Example**:
```
- RECOMMENDED SPLITS (5 modules):
-  [M] config/misc.rs.rs - misc (0 methods, ~25 lines)
-  [M] config/thresholds.rs.rs - thresholds (0 methods, ~116 lines)
```

**Expected Output**:
```
- RECOMMENDED SPLITS (5 modules):
-  [M] config/misc.rs - misc (0 methods, ~25 lines)
-  [M] config/thresholds.rs - thresholds (0 methods, ~116 lines)
```

**Root Cause**:

In `src/priority/formatter.rs:814-815`:
```rust
writeln!(
    output,
    "  {}  {}.{} - {} ({} methods, ~{} lines) [{}]",
    branch,
    split.suggested_name,  // ← May already include .rs
    extension,              // ← Adds .rs again
    ...
)
```

And in `src/organization/god_object_analysis.rs:1145`:
```rust
ModuleSplit {
    suggested_name: format!("config/{}.rs", domain),  // ← Already includes .rs
    ...
}
```

## Objective

Eliminate duplicate file extensions in split name recommendations by:
1. Standardizing split name generation to **never** include file extensions
2. Let the formatter add the appropriate extension based on file type
3. Update all split name generators to use extension-free names
4. Add validation to catch any future violations

## Requirements

### Functional Requirements

**1. Normalize Split Name Generation**
- All `ModuleSplit::suggested_name` values MUST NOT include file extensions
- Split name generators should use semantic names only (e.g., `config/misc` not `config/misc.rs`)
- Formatter is responsible for adding appropriate extension based on source file type

**2. Update Split Name Generators**

Affected locations:
- `src/organization/god_object_analysis.rs:1145` - Domain-based struct splits
- `src/organization/module_function_classifier.rs:152-154` - Function classification splits
- Any other split generators that produce `ModuleSplit` instances

Change pattern:
```rust
// Before:
suggested_name: format!("config/{}.rs", domain)

// After:
suggested_name: format!("config/{}", domain)
```

**3. Formatter Extension Handling**

The formatter already has correct logic in `src/priority/formatter.rs:815`:
```rust
writeln!(output, "  {}  {}.{} - ...", branch, split.suggested_name, extension, ...)
```

This should continue to work once split names are extension-free.

**4. Validation**

Add debug assertions or compile-time checks:
```rust
impl ModuleSplit {
    pub fn new(suggested_name: String, ...) -> Self {
        debug_assert!(
            !suggested_name.ends_with(".rs") &&
            !suggested_name.ends_with(".py") &&
            !suggested_name.ends_with(".js") &&
            !suggested_name.ends_with(".ts"),
            "ModuleSplit::suggested_name should not include file extension: {}",
            suggested_name
        );
        Self { suggested_name, ... }
    }
}
```

### Non-Functional Requirements

- **Backward Compatibility**: JSON output format should remain unchanged (minor breaking change is acceptable)
- **Performance**: No performance impact (pure refactoring)
- **Testing**: Update tests that assert on split names to expect extension-free names

## Acceptance Criteria

- [ ] All `ModuleSplit::suggested_name` values are generated WITHOUT file extensions
- [ ] Formatter correctly appends appropriate extension based on source file type
- [ ] Output shows single extension (e.g., `config/misc.rs`) not double (e.g., `config/misc.rs.rs`)
- [ ] All existing tests updated to reflect extension-free split names
- [ ] Debug assertion added to `ModuleSplit` to prevent future regressions
- [ ] Manual testing confirms output is correct for Rust, Python, JavaScript, TypeScript files
- [ ] No visual changes to output except removal of duplicate extensions

## Technical Details

### Implementation Approach

**Phase 1: Add Validation**
```rust
// src/organization/god_object_analysis.rs
impl ModuleSplit {
    fn validate_name(name: &str) {
        debug_assert!(
            !name.ends_with(".rs") && !name.ends_with(".py") &&
            !name.ends_with(".js") && !name.ends_with(".ts"),
            "ModuleSplit name should not include extension: {}", name
        );
    }
}
```

**Phase 2: Update Split Generators**

Locations to update:
1. `src/organization/god_object_analysis.rs:1145`
   - Change: `format!("config/{}.rs", domain)` → `format!("config/{}", domain)`

2. `src/organization/module_function_classifier.rs:152-154`
   - Already correct (uses `{}_module` format without extension)
   - Verify no edge cases

3. Search for other `ModuleSplit` instantiations:
   ```bash
   rg "ModuleSplit\s*{" --type rust
   ```

**Phase 3: Update Tests**

Files likely needing updates:
- `tests/god_object_config_rs_test.rs`
- `tests/evidence_display_integration_test.rs`
- Any test asserting on `split.suggested_name`

Change assertions:
```rust
// Before:
assert_eq!(split.suggested_name, "config/misc.rs");

// After:
assert_eq!(split.suggested_name, "config/misc");
```

### Architecture Changes

No architectural changes. This is a pure data format refactoring.

### Data Format Change

**Before**:
```json
{
  "suggested_name": "config/misc.rs",
  ...
}
```

**After**:
```json
{
  "suggested_name": "config/misc",
  ...
}
```

**Impact**: Minimal. External consumers (if any) would need to append extension themselves, but this is more correct as extension should be determined by target language.

## Dependencies

- **Prerequisites**: None (pure refactoring)
- **Affected Components**:
  - `src/organization/god_object_analysis.rs` (split name generation)
  - `src/organization/module_function_classifier.rs` (function-based splits)
  - `src/priority/formatter.rs` (already correct, no changes needed)
  - Test files asserting on split names

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_split_names_have_no_extensions() {
    let split = ModuleSplit {
        suggested_name: "config/misc".to_string(),
        // ...
    };

    assert!(!split.suggested_name.ends_with(".rs"));
    assert!(!split.suggested_name.ends_with(".py"));
    assert!(!split.suggested_name.ends_with(".js"));
}

#[test]
#[should_panic]
fn test_split_name_validation_catches_extensions() {
    let split = ModuleSplit::new("config/misc.rs".to_string(), ...);
}
```

### Integration Tests

Update `tests/god_object_config_rs_test.rs`:
```rust
// Verify split names are extension-free
for split in &analysis.recommended_splits {
    assert!(!split.suggested_name.contains(".rs"),
        "Split name should not contain extension: {}", split.suggested_name);
}

// Verify formatter adds extension correctly
let output = format_priorities(&analysis, OutputFormat::Default);
assert!(output.contains("config/misc.rs - misc"));
assert!(!output.contains("config/misc.rs.rs"));
```

### Manual Testing

Test with actual debtmap runs:
```bash
cargo run -- analyze . --format default | grep "RECOMMENDED SPLITS" -A20
```

Expected: No `.rs.rs`, `.py.py`, `.js.js`, or `.ts.ts` in output.

## Documentation Requirements

### Code Documentation

- Add doc comment to `ModuleSplit::suggested_name` field:
  ```rust
  /// Suggested module name WITHOUT file extension (e.g., "config/misc", not "config/misc.rs")
  /// The formatter will add the appropriate extension based on source file type.
  pub suggested_name: String,
  ```

### User Documentation

- No user-facing documentation changes needed (internal implementation detail)

## Implementation Notes

**Gotchas**:
1. Search thoroughly for all `ModuleSplit` instantiations - there may be more than the obvious ones
2. Some split generators may use string concatenation instead of `format!` macro
3. Check both direct `ModuleSplit { ... }` instantiations and builder patterns

**Best Practices**:
- Make the change atomic - all split generators at once
- Run full test suite after changes
- Add regression test to prevent future violations

## Migration and Compatibility

### Breaking Changes

**Minor breaking change**: JSON output format changes `suggested_name` field to not include extension.

**Impact**:
- Low - Most consumers likely use the formatted text output, not raw JSON
- External JSON consumers would need to append extension themselves
- This is arguably more correct as extension should be determined by consumer context

### Migration Strategy

No migration needed. This is a forward-only change affecting new analysis runs only.

## Implementation Order

1. **Add validation** - Debug assertion in `ModuleSplit` to catch violations
2. **Update generators** - Fix all `ModuleSplit::suggested_name` assignments
3. **Update tests** - Adjust test assertions to expect extension-free names
4. **Manual verification** - Run debtmap on real codebases, verify output
5. **Commit with clear message** - "fix: remove duplicate file extensions from split names (spec 164)"

## Related Issues

- Spec 140: Domain-Based Struct Split Recommendations (introduced the bug)
- Spec 152: Domain Diversity Metrics (also mentions output formatting)

## Success Metrics

- Zero instances of double extensions (`.rs.rs`, `.py.py`, etc.) in output
- All tests passing with updated assertions
- Debug assertions catch any future violations in development
