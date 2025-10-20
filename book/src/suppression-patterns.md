# Suppression Patterns

Debtmap provides flexible suppression mechanisms to help you focus on the technical debt that matters most. You can suppress specific debt items inline with comments, or exclude entire files and functions through configuration.

## Why Use Suppressions?

Not all detected technical debt requires immediate action. Suppressions allow you to:

- **Focus on priorities**: Hide known, accepted debt to see new issues clearly
- **Handle false positives**: Suppress patterns that don't apply to your context
- **Document decisions**: Explain why certain debt is acceptable using reason annotations
- **Exclude test code**: Ignore complexity in test fixtures and setup functions

## Inline Comment Suppression

Debtmap supports four inline comment formats that work with your language's comment syntax:

### Single-Line Suppression

Suppress debt on the same line as the comment:

```rust
// debtmap:ignore
// TODO: Implement caching later - performance is acceptable for now
```

```python
# debtmap:ignore
# FIXME: Refactor this after the Q2 release
```

The suppression applies to debt detected on the same line as the comment.

### Next-Line Suppression

Suppress debt on the line immediately following the comment:

```rust
// debtmap:ignore-next-line
fn complex_algorithm() {
    // ...20 lines of complex code...
}
```

```typescript
// debtmap:ignore-next-line
function calculateMetrics(data: DataPoint[]): Metrics {
    // ...complex implementation...
}
```

This format is useful when you want the suppression comment to appear before the code it affects.

### Block Suppression

Suppress multiple lines of code between start and end markers:

```rust
// debtmap:ignore-start
fn setup_test_environment() {
    // TODO: Add more test cases
    // FIXME: Handle edge cases
    // Complex test setup code...
}
// debtmap:ignore-end
```

```python
# debtmap:ignore-start
def mock_api_responses():
    # TODO: Add more mock scenarios
    # Multiple lines of mock setup
    pass
# debtmap:ignore-end
```

**Important**: Every `ignore-start` must have a matching `ignore-end`. Debtmap tracks unclosed blocks and can warn you about them.

## Type-Specific Suppression

You can suppress specific types of debt using bracket notation instead of suppressing everything:

### Suppress Specific Types

```rust
// debtmap:ignore[todo]
// TODO: This TODO is ignored, but FIXMEs and complexity are still reported
```

```rust
// debtmap:ignore[todo,fixme]
// TODO: Both TODOs and FIXMEs are ignored here
// FIXME: But complexity issues would still be detected
```

### Supported Debt Types

- `todo` - TODO comments
- `fixme` - FIXME comments
- `hack` - HACK markers
- `smell` or `codesmell` - Code smell patterns
- `complexity` - High cognitive complexity
- `duplication` - Code duplication
- `god_object` - God object warnings
- `testing` - Testing gap warnings
- `dependency` - Dependency issues
- `*` - All types (wildcard)

### Wildcard Suppression

Use `[*]` to explicitly suppress all types (equivalent to no bracket notation):

```rust
// debtmap:ignore[*]
// Suppresses all debt types
```

### Type-Specific Blocks

Block suppressions also support type filtering:

```rust
// debtmap:ignore-start[complexity]
fn intentionally_complex_for_performance() {
    // Complex nested logic is intentional here
    // Complexity warnings suppressed, but TODOs still detected
}
// debtmap:ignore-end
```

## Suppression Reasons

Document why you're suppressing debt using the `--` separator:

```rust
// debtmap:ignore -- Intentional for backward compatibility
// TODO: Remove this after all clients upgrade to v2.0
```

```python
# debtmap:ignore[complexity] -- Performance-critical hot path
def optimize_query(params):
    # Complex but necessary for performance
    pass
```

```typescript
// debtmap:ignore-next-line -- Waiting on upstream library fix
function workaroundBug() {
    // FIXME: Remove when library v3.0 is released
}
```

**Best Practice**: Always include reasons for suppressions. This helps future maintainers understand the context and know when suppressions can be removed.

## Config File Exclusions

For broader exclusions, use the `[ignore]` section in `.debtmap.toml`:

### File Pattern Exclusions

```toml
[ignore]
patterns = [
    "target/**",              # Build artifacts
    "node_modules/**",        # Dependencies
    "**/*_test.rs",           # Test files with _test suffix
    "tests/**",               # All test directories
    "**/fixtures/**",         # Test fixtures
    "**/mocks/**",            # Mock implementations
    "**/*.min.js",            # Minified files
    "**/demo/**",             # Demo code
    "**/*.generated.rs",      # Generated files
    "vendor/**",              # Vendor code
    "third_party/**",         # Third-party code
]
```

### Function Name Exclusions

Exclude entire function families by name pattern:

```toml
[ignore.functions]
patterns = [
    # Test setup functions
    "setup_test_*",
    "teardown_test_*",
    "create_test_*",
    "mock_*",

    # Generated code
    "derive_*",
    "__*",                    # Python dunder methods

    # CLI parsing (naturally complex)
    "parse_args",
    "parse_cli",
    "build_cli",

    # Serialization (naturally complex pattern matching)
    "serialize_*",
    "deserialize_*",
    "to_json",
    "from_json",
]
```

Function patterns use wildcard matching where `*` matches any characters.

## Glob Pattern Syntax

File patterns use standard glob syntax:

| Pattern | Matches | Example |
|---------|---------|---------|
| `*` | Any characters within a path component | `*.rs` matches `main.rs` |
| `**` | Any directories (recursive) | `tests/**` matches `tests/unit/foo.rs` |
| `?` | Single character | `test?.rs` matches `test1.rs` |
| `[abc]` | Character class | `test[123].rs` matches `test1.rs` |
| `[!abc]` | Negated class | `test[!0].rs` matches `test1.rs` but not `test0.rs` |

### Glob Pattern Examples

```toml
[ignore]
patterns = [
    "src/**/*_generated.rs",  # Generated files in any subdirectory
    "**/test_*.py",           # Python test files anywhere
    "legacy/**/[!i]*.js",     # Legacy JS files not starting with 'i'
    "**/*.{min.js,min.css}",  # Minified assets
]
```

## Language-Specific Comment Syntax

Debtmap automatically uses the correct comment syntax for each language:

| Language | Comment Prefix | Example |
|----------|---------------|---------|
| Rust | `//` | `// debtmap:ignore` |
| JavaScript | `//` | `// debtmap:ignore` |
| TypeScript | `//` | `// debtmap:ignore` |
| Python | `#` | `# debtmap:ignore` |

You don't need to configure this—Debtmap detects the language and uses the appropriate syntax.

## Explicitly Specified Files

**Important behavior**: When you analyze a specific file directly, ignore patterns are bypassed:

```bash
# Respects [ignore] patterns in .debtmap.toml
debtmap analyze .
debtmap analyze src/

# Bypasses ignore patterns - analyzes the file even if patterns would exclude it
debtmap analyze src/test_helper.rs
```

This ensures you can always analyze specific files when needed, even if they match an ignore pattern.

## Suppression Statistics

Debtmap tracks suppression usage and can detect issues:

- **Total suppressions**: Count of active suppressions
- **Suppressions by type**: How many of each debt type are suppressed
- **Unclosed blocks**: Detection of `ignore-start` without matching `ignore-end`

Future versions may include a command to report suppression statistics for your codebase.

## Best Practices

### Use Suppressions Sparingly

Suppressions hide information, so use them intentionally:

✅ **Good use cases:**
- Test fixtures and mock data
- Known technical debt with an accepted timeline
- Intentional complexity for performance
- False positives specific to your domain

❌ **Poor use cases:**
- Hiding all debt to make reports look clean
- Suppressing instead of fixing simple issues
- Using wildcards when specific types would work

### Always Include Reasons

```rust
// ✅ Good: Clear reason and timeline
// debtmap:ignore[complexity] -- Hot path optimization, profiled and necessary
fn fast_algorithm() { }

// ❌ Bad: No context for future maintainers
// debtmap:ignore
fn fast_algorithm() { }
```

### Prefer Specific Over Broad

```rust
// ✅ Good: Only suppress the specific debt type
// debtmap:ignore[todo] -- Remove after v2.0 migration
// TODO: Migrate to new API

// ❌ Bad: Suppresses everything, including real issues
// debtmap:ignore
// TODO: Migrate to new API
```

### Use Config for Systematic Exclusions

For patterns that apply project-wide, use `.debtmap.toml` instead of inline comments:

```toml
# ✅ Good: One config applies to all test files
[ignore]
patterns = ["tests/**"]

# ❌ Bad: Repetitive inline suppressions in every test file
```

### Review Suppressions Periodically

Suppressions can become outdated:

- Remove suppressions when the reason no longer applies
- Check if suppressed debt can now be fixed
- Verify reasons are still accurate after refactoring

**Solution:** Periodically search for suppressions:
```bash
rg "debtmap:ignore" --type rust
```

### Ensure Blocks Are Closed

```rust
// ✅ Good: Properly closed block
// debtmap:ignore-start
fn test_setup() { }
// debtmap:ignore-end

// ❌ Bad: Unclosed block affects all subsequent code
// debtmap:ignore-start
fn test_setup() { }
// (missing ignore-end)
```

Debtmap detects unclosed blocks and can warn you about them.

## Common Patterns

### Suppressing Test Code

```toml
# In .debtmap.toml
[ignore]
patterns = [
    "tests/**/*",
    "**/*_test.rs",
    "**/test_*.py",
    "**/fixtures/**",
]

[ignore.functions]
patterns = [
    "test_*",
    "setup_*",
    "teardown_*",
    "mock_*",
]
```

### Suppressing Generated Code

```toml
[ignore]
patterns = [
    "**/*_generated.*",
    "**/proto/**",
    "**/bindings/**",
]

[ignore.functions]
patterns = [
    "derive_*",
    "__*",
]
```

### Temporary Suppressions with Timeline

```rust
// debtmap:ignore[complexity] -- TODO: Refactor during Q2 2025 sprint
fn legacy_payment_processor() {
    // Complex legacy code scheduled for refactoring
}
```

### Suppressing False Positives

```python
# debtmap:ignore[duplication] -- Similar but semantically different
def calculate_tax_us():
    # US tax calculation
    pass

# debtmap:ignore[duplication] -- Similar but semantically different
def calculate_tax_eu():
    # EU tax calculation with different rules
    pass
```

### Conditional Suppression

```rust
#[cfg(test)]
// debtmap:ignore[complexity]
fn test_helper() {
    // Complex test setup is acceptable
}
```

### Suppression with Detailed Justification

```rust
// debtmap:ignore[complexity] -- Required by specification XYZ-123
// This function implements the state machine defined in spec XYZ-123.
// Complexity is inherent to the specification and cannot be reduced
// without violating requirements.
fn state_machine() { ... }
```

## Troubleshooting

### Suppression Not Working

1. **Check comment syntax**: Ensure you're using the correct comment prefix for your language (`//` for Rust/JS/TS, `#` for Python)
2. **Verify spelling**: It's `debtmap:ignore`, not `debtmap-ignore` or `debtmap_ignore`
3. **Check line matching**: For same-line suppressions, ensure the debt is on the same line as the comment
4. **Verify type names**: Use `todo`, `fixme`, `complexity`, etc. (lowercase)

**Common syntax errors:**
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

### Unclosed Block Warning

If you see warnings about unclosed blocks:

```rust
// Problem: Missing ignore-end
// debtmap:ignore-start
fn test_helper() { }
// (Should have debtmap:ignore-end here)

// Solution: Add the closing marker
// debtmap:ignore-start
fn test_helper() { }
// debtmap:ignore-end
```

### File Still Being Analyzed

If a file in your ignore patterns is still being analyzed:

1. Check if you're analyzing the specific file directly (bypasses ignore patterns)
2. Verify the glob pattern matches the file path
3. Check for typos in the pattern
4. Test the pattern in isolation

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

### Function Suppression Not Working

If function name patterns aren't working:

1. Verify the pattern is under `[ignore.functions]`, not `[ignore]`
2. Check the function name exactly matches (case-sensitive)
3. Remember `*` is a wildcard: `test_*` matches `test_foo` but not `my_test`

## Related Topics

- [Configuration](configuration.md) - Full `.debtmap.toml` reference
- [CLI Reference](cli-reference.md) - Command-line analysis options
- [Analysis Guide](analysis-guide.md) - Understanding debt detection
- [Output Formats](output-formats.md) - Viewing suppressed items in reports

## Summary

Suppressions help you focus on actionable technical debt:

- **Inline comments**: `debtmap:ignore`, `ignore-next-line`, `ignore-start/end`
- **Type-specific**: Use `[type1,type2]` to suppress selectively
- **Reasons**: Always use `-- reason` to document why
- **Config patterns**: Use `.debtmap.toml` for systematic exclusions
- **Function patterns**: Use `[ignore.functions]` for function name matching
- **Best practices**: Use sparingly, prefer specific over broad, review periodically

With proper use of suppressions, your Debtmap reports show only the debt that matters to your team.
