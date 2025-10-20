# Suppression Patterns

Debtmap provides flexible suppression mechanisms to ignore specific debt items when they're intentional or false positives. This chapter covers inline comment suppression and config file exclusion patterns.

## Overview

Suppression is useful when:
- Technical debt is intentional and documented
- False positives are identified
- Generated code should be excluded
- Test fixtures have acceptable "debt"
- Legacy code is being gradually migrated

## Inline Comment Suppression

Suppress debt items using special comments in your source code.

### Basic Suppression

**Suppress all debt types on current line:**
```rust
fn complex_function() { // debtmap:ignore
    // This function won't be flagged
}
```

**Suppress all debt types on next line:**
```rust
// debtmap:ignore-next-line
fn complex_function() {
    // This function won't be flagged
}
```

### Block Suppression

**Suppress a block of code:**
```rust
// debtmap:ignore-start
fn function_1() { ... }
fn function_2() { ... }
fn function_3() { ... }
// debtmap:ignore-end
```

All functions within the block are suppressed.

### Type-Specific Suppression

**Suppress only specific debt types:**
```rust
// debtmap:ignore[todo]
// TODO: Refactor this function
fn process_data() { ... }
```

**Available types:**
- `todo` - Suppress TODO markers
- `fixme` - Suppress FIXME markers
- `hack` - Suppress HACK markers
- `complexity` - Suppress complexity warnings
- `duplication` - Suppress code duplication
- `god_object` - Suppress god object warnings
- `testing` - Suppress testing gap warnings
- `*` - Suppress all types (wildcard)

### Multiple Types

**Suppress multiple types:**
```rust
// debtmap:ignore[todo,fixme]
fn legacy_function() {
    // TODO: Clean this up
    // FIXME: Handle edge case
}
```

### Language-Specific Syntax

**Rust:**
```rust
// debtmap:ignore
fn function() { }
```

**Python:**
```python
# debtmap:ignore
def function():
    pass
```

**JavaScript/TypeScript:**
```javascript
// debtmap:ignore
function doSomething() { }
```

**All languages support the same suppression formats.**

## Config File Exclusion

Exclude entire files or directories via `.debtmap.toml`.

### Basic Exclusion

```toml
[ignore]
patterns = [
    "tests/**/*",
    "**/fixtures/**",
    "target/**",
    "node_modules/**",
]
```

### Glob Pattern Syntax

**Wildcards:**
- `*` - Match any characters except `/`
- `**` - Match any characters including `/`
- `?` - Match single character
- `[abc]` - Match any character in set
- `[!abc]` - Match any character NOT in set

**Examples:**
```toml
[ignore]
patterns = [
    # Ignore all test files
    "**/*.test.rs",
    "**/*_test.py",
    "**/*.spec.js",

    # Ignore specific directories
    "tests/**",
    "benchmarks/**",
    "examples/**",

    # Ignore generated files
    "**/generated/**",
    "**/*.generated.rs",

    # Ignore vendor code
    "vendor/**",
    "third_party/**",

    # Ignore build artifacts
    "target/**",
    "dist/**",
    "build/**",

    # Ignore documentation
    "docs/**/*.md",

    # Complex patterns
    "src/*/legacy_*.rs",  # Match src/module/legacy_foo.rs
    "lib/[a-z]*.js",      # Match lib/foo.js but not lib/Foo.js
]
```

### Combining Inline and Config Suppression

Use both for maximum flexibility:

```toml
# .debtmap.toml - Exclude entire directories
[ignore]
patterns = ["tests/**", "vendor/**"]
```

```rust
// src/main.rs - Suppress specific functions
fn main() {
    // debtmap:ignore[complexity]
    complex_initialization();
}
```

## Suppression Best Practices

### Document Why

**Bad:**
```rust
// debtmap:ignore
fn complex_function() { ... }
```

**Good:**
```rust
// debtmap:ignore[complexity] - State machine logic requires complexity
fn reconcile_state() { ... }
```

### Use Type-Specific Suppression

**Bad:**
```rust
// debtmap:ignore
// TODO: Refactor
fn function() { ... }
```

**Good:**
```rust
// debtmap:ignore[todo]
// TODO: Refactor after v2.0 release
fn function() { ... }
```

### Prefer Config for Large-Scale Exclusions

**Bad:**
```rust
// debtmap:ignore
mod tests { ... }

// debtmap:ignore
mod more_tests { ... }

// ... repeat 50 times
```

**Good:**
```toml
[ignore]
patterns = ["tests/**"]
```

### Review Suppressions Periodically

Suppressions can become stale:

```rust
// debtmap:ignore[complexity] - Waiting for v2.0 API
fn function() {
    // ... code was refactored but suppression remains
}
```

**Solution:** Periodically search for suppressions:
```bash
rg "debtmap:ignore" --type rust
```

## Finding Suppressed Items

### Count Suppressions

```bash
# Inline suppressions
rg "debtmap:ignore" | wc -l

# Config patterns
grep "patterns" .debtmap.toml
```

### Identify Over-Suppression

**Warning signs:**
- Many suppressions in a single file
- Wildcard suppressions (`debtmap:ignore[*]`)
- Entire modules suppressed

**Example of over-suppression:**
```rust
// debtmap:ignore-start
mod user_service {
    // 500 lines of code...
}
// debtmap:ignore-end
```

**Better approach:**
- Refactor the code instead of suppressing
- Use type-specific suppression
- Suppress only specific functions

## Advanced Patterns

### Conditional Suppression

**Suppress based on context:**
```rust
#[cfg(test)]
// debtmap:ignore[complexity]
fn test_helper() {
    // Complex test setup is acceptable
}
```

### Temporary Suppression

**Add expiration date:**
```rust
// debtmap:ignore[todo] - Remove after 2025-02-01
// TODO: Migrate to new API
fn legacy_function() { ... }
```

### Suppression with Justification

```rust
// debtmap:ignore[complexity] - Required by specification XYZ-123
// This function implements the state machine defined in spec XYZ-123.
// Complexity is inherent to the specification and cannot be reduced
// without violating requirements.
fn state_machine() { ... }
```

## Suppression Reporting

### View Suppressed Items (Future Feature)

```bash
# Show all suppressed items
debtmap analyze . --show-suppressed

# Show only specific types
debtmap analyze . --show-suppressed=complexity
```

### Export Suppressions (Future Feature)

```bash
debtmap analyze . --export-suppressions suppressions.json
```

Output:
```json
{
  "suppressions": [
    {
      "file": "src/main.rs",
      "line": 42,
      "type": "inline",
      "suppressed_types": ["complexity"],
      "reason": "State machine logic requires complexity"
    }
  ]
}
```

## Troubleshooting

### Suppression Not Working

**Issue:** Suppression comment is ignored

**Possible causes:**
1. Syntax error in suppression comment
2. Suppression on wrong line
3. Type name is incorrect

**Solutions:**

**Check syntax:**
```rust
// Wrong: debtmap: ignore (space after colon)
// Right: debtmap:ignore

// Wrong: debtmap:ignore[Complexity] (capital C)
// Right: debtmap:ignore[complexity]
```

**Check placement:**
```rust
// Wrong: comment after code
fn function() { } // debtmap:ignore

// Right: comment before code
// debtmap:ignore
fn function() { }
```

**Check type name:**
```rust
// Wrong: debtmap:ignore[cyclomatic]
// Right: debtmap:ignore[complexity]
```

### Pattern Not Matching Files

**Issue:** Config pattern doesn't exclude expected files

**Possible causes:**
1. Glob pattern syntax error
2. Path mismatch (absolute vs relative)
3. Pattern too specific or too broad

**Solutions:**

**Test pattern with find:**
```bash
find . -path "tests/**/*" -type f
```

**Use double asterisk for subdirectories:**
```toml
# Wrong: "tests/*" (only direct children)
# Right: "tests/**/*" (all descendants)
```

**Check relative paths:**
```toml
# Patterns are relative to project root
patterns = [
    "src/legacy/**",  # ✓ Correct
    "/src/legacy/**", # ✗ Wrong (absolute path)
]
```

## Best Practices Summary

1. **Document why** - Always include a reason for suppression
2. **Use type-specific** - Suppress only what's needed
3. **Prefer config for large exclusions** - Use inline for specific items
4. **Review periodically** - Suppressions can become stale
5. **Avoid over-suppression** - Suppressing entire modules is a red flag
6. **Consider refactoring** - Suppression should be last resort

## See Also

- [Configuration](configuration.md) - Complete configuration reference
- [CLI Reference](cli-reference.md) - Command-line options
- [Troubleshooting](troubleshooting.md) - General troubleshooting guide
