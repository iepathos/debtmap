# Understanding LOC Counts in Debtmap

## Overview

Debtmap uses a consistent methodology for counting Lines of Code (LOC) across all analysis modes. This document explains how LOC is counted, what files are included/excluded, and how to configure the counting behavior.

## LOC Counting Methodology

### Line Categories

Debtmap categorizes lines into four types:

1. **Physical Lines**: Total raw line count from the file (including everything)
2. **Code Lines**: Lines containing executable code (excludes comments and blank lines)
3. **Comment Lines**: Lines that are primarily comments
4. **Blank Lines**: Lines containing only whitespace

### What Counts as Code?

A line is counted as **code** if:
- It contains any executable statement or declaration
- It's not a comment-only line
- It's not a blank line

Examples:
```rust
fn main() {              // Code line
    println!("Hello");   // Code line
    // This is a comment // Comment line
                         // Blank line
}                        // Code line
```

### What Counts as a Comment?

A line is counted as a **comment** if it starts with (after trimming whitespace):
- `//` (Rust, JavaScript, TypeScript single-line comments)
- `/*` (Multi-line comment start)
- `*` (Continuation of multi-line comment, but not `*/`)
- `#` (Python comments)

## File Filtering

### Default Exclusions

By default, Debtmap **excludes** the following from LOC counts:

1. **Test Files**:
   - Files in `tests/` or `test/` directories
   - Files with `_test.rs` or `_tests.rs` suffixes
   - Files with `test_*.rs` prefixes

2. **Generated Files**:
   - Files containing `@generated` marker
   - Files containing `DO NOT EDIT` marker
   - Files containing `automatically generated` marker
   - Files with `.generated.` in the name
   - Files ending with `.g.rs`

### Custom Exclusions

You can configure additional exclusion patterns in your `.debtmap.toml`:

```toml
[debtmap.loc]
exclude_patterns = ["vendor", "third_party", "generated"]
```

## Configuration

### Config File (`.debtmap.toml`)

Configure LOC counting behavior in your project's `.debtmap.toml`:

```toml
[debtmap.loc]
# Include test files in LOC count (default: false)
include_tests = false

# Include generated files in LOC count (default: false)
include_generated = false

# Count comment lines as code lines (default: false)
count_comments = false

# Count blank lines as code lines (default: false)
count_blanks = false

# Additional patterns to exclude from LOC count
exclude_patterns = [
    "vendor",
    "third_party",
    "node_modules"
]
```

### Why Default to Excluding Tests?

Debtmap excludes test files by default because:
1. **Focus on Production Code**: The primary goal is to analyze production code quality
2. **Different Quality Standards**: Tests follow different complexity patterns (often intentionally high setup)
3. **Consistent Metrics**: Excluding tests provides more consistent complexity and debt metrics

If you want to include tests in your LOC count (e.g., for total codebase size metrics), set `include_tests = true`.

## Coverage Integration

When you provide a coverage file (LCOV format), Debtmap ensures consistency between:
- LOC counts from file analysis
- Line counts reported in coverage data

### How It Works

1. Debtmap parses your code files and counts LOC using the rules above
2. If coverage data is provided, it recalculates using the same `LocCounter`
3. This ensures the coverage denominator matches the LOC numerator

### Validating Consistency

Use the `--validate-loc` flag to check for discrepancies:

```bash
debtmap analyze . --coverage-file coverage/lcov.info --validate-loc
```

This will report any files where the LOC count differs between modes, helping you identify potential counting issues.

## Command-Line Validation

### `--validate-loc` Flag

The `--validate-loc` flag provides file-by-file LOC accounting:

```bash
debtmap analyze . --validate-loc
```

Output example:
```
LOC Validation Report
=====================

File: src/main.rs
  Code lines: 245
  Comment lines: 42
  Blank lines: 38
  Included in count: Yes

File: tests/integration_test.rs
  Code lines: 156
  Excluded: Test file (use --include-tests to include)

Total Code Lines: 245 (1 file included, 1 file excluded)
```

This helps you:
- Verify which files are included/excluded
- Understand why files are excluded
- Debug LOC count discrepancies

## Common Scenarios

### Scenario 1: "My LOC count seems low"

**Cause**: Test files are excluded by default

**Solution**:
```toml
[debtmap.loc]
include_tests = true
```

Or use CLI override: `debtmap analyze . --include-tests` (if such flag exists)

### Scenario 2: "LOC differs with coverage reports"

**Cause**: Coverage tools may count lines differently (e.g., including test files)

**Solution**:
1. Use `--validate-loc` to see detailed breakdown
2. Configure Debtmap to match your coverage tool's counting methodology
3. Or configure your coverage tool to exclude test files

### Scenario 3: "Want to exclude vendor code"

**Solution**:
```toml
[debtmap.loc]
exclude_patterns = ["vendor", "third_party"]
```

### Scenario 4: "Need consistent counts for CI metrics"

**Solution**:
1. Check in `.debtmap.toml` with LOC configuration
2. Use `--validate-loc` in CI to ensure consistency
3. Fail CI if counts diverge unexpectedly

## Best Practices

1. **Check in your config**: Add `.debtmap.toml` to version control for team consistency
2. **Document exclusions**: Comment why specific patterns are excluded
3. **Validate in CI**: Use `--validate-loc` to catch configuration drift
4. **Match your coverage tool**: Configure Debtmap to count lines the same way as your coverage tool

## Debug Logging

Enable debug logging to see which files are included/excluded:

```bash
RUST_LOG=debug debtmap analyze .
```

Output will include:
```
DEBUG debtmap::metrics::loc_counter: Including file in LOC count: src/main.rs
DEBUG debtmap::metrics::loc_counter: Excluding test file: tests/integration_test.rs
DEBUG debtmap::metrics::loc_counter: Excluding generated file: src/generated/schema.rs
```

## Technical Details

### Consistency Guarantees

Debtmap guarantees:
1. **Same counter, same count**: The same `LocCounter` instance produces identical counts
2. **Deterministic**: Counting is deterministic - same input always produces same output
3. **Pure functions**: Counting logic is pure (no side effects, no global state)

### Performance

- LOC counting is parallelized across files
- File content is read once and cached
- Exclusion patterns use fast string matching (not regex by default)

## Related Documentation

- [Coverage Integration Guide](./coverage-integration.md) (if exists)
- [Configuration Reference](./.debtmap.toml.example) (if exists)
- [Score Interpretation Guide](./score-interpretation-guide.md)
