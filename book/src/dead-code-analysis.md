# Dead Code Analysis

Debtmap can detect potentially unused code through public API heuristics and dead code identification. This helps identify code that may be safe to remove, reducing maintenance burden.

## Overview

Dead code analysis identifies:
- Unused functions and methods
- Unreferenced types
- Dead imports
- Public API boundaries

## Public API Detection

### Heuristics

Debtmap uses multiple heuristics to identify public APIs:

**Rust:**
- `pub` visibility
- `#[no_mangle]` attribute
- Exported from `lib.rs` or `main.rs`
- Documented with `///` doc comments
- Used in public trait implementations

**Python:**
- Module `__all__` exports
- Lack of leading underscore
- Used in `__init__.py`

**JavaScript/TypeScript:**
- `export` statements
- Used in `index.ts`
- JSDoc with `@public`

### Confidence Levels

Each detection has a confidence score (0.0-1.0):
- **0.9-1.0**: Definitely public API
- **0.7-0.9**: Likely public API
- **0.5-0.7**: Possibly public API
- **0.0-0.5**: Likely internal

## Configuration

### Enable Dead Code Analysis

```bash
# Dead code analysis is enabled by default
debtmap analyze .
```

### Disable Public API Detection

```bash
debtmap analyze . --no-public-api-detection
```

### Adjust Confidence Threshold

```bash
# Stricter detection (higher confidence required)
debtmap analyze . --public-api-threshold 0.8

# More lenient (catch more potential public APIs)
debtmap analyze . --public-api-threshold 0.6
```

Default threshold: 0.7

### Configuration File

```toml
[dead_code]
enabled = true
public_api_threshold = 0.7
exclude_tests = true
exclude_examples = true

[dead_code.rust]
check_exports = true
check_doc_comments = true
check_no_mangle = true

[dead_code.python]
check___all__ = true
check_underscore = true
```

## Dead Code Identification

### Detection Criteria

Code is flagged as potentially dead when:
1. Not referenced in call graph
2. Below public API confidence threshold
3. Not in exclusion list
4. Not marked as intentionally unused

### Example

```rust
// Detected as potentially dead (no references, not public)
fn internal_helper() {
    // ...
}

// Not flagged (public API)
pub fn public_function() {
    // ...
}

// Not flagged (referenced)
fn helper() {
    // ...
}

fn caller() {
    helper();  // Reference prevents dead code flag
}
```

## Best Practices

**Review before deleting:**
- Verify no dynamic calls
- Check for reflection usage
- Validate testing implications
- Consider future plans

**Common false positives:**
- Plugin systems
- Trait implementations
- Callback functions
- FFI exports

**Suppression for intentional code:**
```rust
// debtmap:ignore dead-code
fn future_feature() {
    // Planned but not yet used
}
```

**Exclude appropriate paths:**
```toml
[dead_code]
exclude_paths = [
    "tests/",
    "examples/",
    "benches/",
    "prototypes/"
]
```

## Use Cases

### Code Cleanup

```bash
# Find potentially unused code
debtmap analyze . --filter-categories DeadCode --format markdown
```

### API Review

```bash
# Identify public API surface
debtmap analyze . --public-api-threshold 0.8
```

### Refactoring Safety

```bash
# Verify function is unused before removing
debtmap analyze . --trace-function old_function
```

## Troubleshooting

### False Positives

**Issue:** Active code flagged as dead

**Solution:**
- Lower confidence threshold
- Check for dynamic calls
- Verify call graph completeness
- Use suppression comments

### Missing Dead Code

**Issue:** Known unused code not detected

**Solution:**
- Raise confidence threshold
- Enable comprehensive analysis
- Disable public API detection for internal code

## See Also

- [Call Graph Analysis](analysis-guide.md#call-graph) - Understanding references
- [Suppression Patterns](suppression-patterns.md) - Suppress false positives
- [Configuration](configuration.md) - Dead code configuration
