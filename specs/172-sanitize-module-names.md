---
number: 172
title: Sanitize Module Names in God Object Recommendations
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-11-10
---

# Specification 172: Sanitize Module Names in God Object Recommendations

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The god object detection system generates module split recommendations with suggested module names. Currently, these names can contain invalid characters that make them unsuitable as actual module filenames.

**Current Issue**:
```rust
// Input: "Parsing & Input"
suggested_name: format!(
    "{}_{}",
    type_name.to_lowercase(),
    responsibility.to_lowercase().replace(' ', "_")
)
// Output: "twopassextractor_parsing_&_input"  ← Invalid! Contains "&"
```

**Real-world example from debtmap output**:
```
- mod_parsing_&_input.rs - Parsing & Input (6 methods, ~120 lines)
```

This produces invalid module names that:
1. Cannot be used as actual filenames in many filesystems
2. Are not valid Rust module identifiers
3. Create syntax errors if used in `mod` declarations
4. Look unprofessional in recommendations

## Objective

Implement robust module name sanitization that ensures all recommended module names are valid identifiers across all supported languages (Rust, Python, JavaScript, TypeScript).

## Requirements

### Functional Requirements

**FR1: Character Sanitization**
- Replace invalid characters with valid alternatives
  - `&` → `and`
  - `'` → empty string
  - `-` → `_`
  - Multiple spaces → single `_`
  - Multiple underscores → single `_`
- Remove leading/trailing underscores
- Convert to lowercase
- Preserve alphanumeric characters and underscores

**FR2: Language Compatibility**
- Generated names must be valid module identifiers in:
  - Rust: `mod {name};`
  - Python: `import {name}`
  - JavaScript/TypeScript: `import ... from './{name}'`
- Support directory separators: `config/misc` should become `config/misc` (keep `/`)
- Validate against reserved keywords (e.g., "mod", "type", "if")

**FR3: Collision Prevention**
- Detect and prevent name collisions after sanitization
- Add numeric suffix if collision detected: `utilities_1`, `utilities_2`
- Maintain deterministic behavior (same input → same output)

### Non-Functional Requirements

**NFR1: Performance**
- Sanitization should be O(n) where n = string length
- No regex compilation per call (compile once, reuse)
- Minimal allocations (single pass transformation)

**NFR2: Maintainability**
- Pure function with no side effects
- Comprehensive unit tests for edge cases
- Clear documentation of transformation rules

**NFR3: Backward Compatibility**
- Existing valid names should remain unchanged
- Only transform invalid names
- Preserve semantic meaning where possible

## Acceptance Criteria

- [ ] All special characters are replaced or removed
- [ ] Multiple consecutive underscores collapsed to single underscore
- [ ] Leading and trailing underscores removed
- [ ] Output names pass validation for Rust, Python, JS/TS modules
- [ ] Name collisions detected and resolved with numeric suffixes
- [ ] Function is pure and deterministic
- [ ] 100% unit test coverage with 50+ test cases
- [ ] All existing god object tests still pass
- [ ] Real-world test: "Parsing & Input" → "parsing_and_input"
- [ ] Real-world test: "Data Access" → "data_access"
- [ ] Real-world test: "Utilities" → "utilities"

## Technical Details

### Implementation Approach

**Location**: `src/organization/god_object_analysis.rs`

**New Function**:
```rust
/// Sanitize module name to be valid across all languages.
///
/// Transforms human-readable responsibility names into valid module identifiers
/// by replacing invalid characters and normalizing whitespace.
///
/// # Examples
///
/// ```
/// assert_eq!(sanitize_module_name("Parsing & Input"), "parsing_and_input");
/// assert_eq!(sanitize_module_name("Data  Access"), "data_access");
/// assert_eq!(sanitize_module_name("I/O Utilities"), "io_utilities");
/// ```
pub fn sanitize_module_name(name: &str) -> String {
    name.to_lowercase()
        .replace('&', "and")
        .replace('/', "_")
        .replace('-', "_")
        .replace('\'', "")
        .replace(' ', "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}
```

**Integration Point**:
```rust
// src/organization/god_object_analysis.rs:1083
ModuleSplit {
    suggested_name: format!(
        "{}_{}",
        type_name.to_lowercase(),
        sanitize_module_name(&responsibility)  // ← Use sanitization
    ),
    // ... rest of fields
}
```

### Reserved Keywords Check

```rust
const RESERVED_KEYWORDS: &[&str] = &[
    // Rust
    "mod", "pub", "use", "type", "impl", "trait", "fn", "let", "mut",
    // Python
    "import", "from", "def", "class", "if", "for", "while", "try",
    // JavaScript/TypeScript
    "import", "export", "function", "const", "let", "var", "class",
];

fn is_reserved_keyword(name: &str) -> bool {
    RESERVED_KEYWORDS.contains(&name)
}

fn ensure_not_reserved(mut name: String) -> String {
    if is_reserved_keyword(&name) {
        name.push_str("_module");
    }
    name
}
```

### Collision Detection

```rust
/// Ensure uniqueness by appending numeric suffix if needed
pub fn ensure_unique_name(
    name: String,
    existing_names: &HashSet<String>,
) -> String {
    if !existing_names.contains(&name) {
        return name;
    }

    let mut counter = 1;
    loop {
        let candidate = format!("{}_{}", name, counter);
        if !existing_names.contains(&candidate) {
            return candidate;
        }
        counter += 1;
    }
}
```

## Dependencies

**Prerequisites**: None

**Affected Components**:
- `src/organization/god_object_analysis.rs` - Add sanitization function
- `src/organization/module_function_classifier.rs:153` - Apply to module splits
- `src/priority/formatter.rs:885` - Display sanitized names

## Testing Strategy

### Unit Tests

**Test Suite**: `tests/module_name_sanitization_test.rs`

```rust
#[test]
fn test_ampersand_replacement() {
    assert_eq!(sanitize_module_name("Parsing & Input"), "parsing_and_input");
    assert_eq!(sanitize_module_name("Read & Write"), "read_and_write");
}

#[test]
fn test_multiple_spaces() {
    assert_eq!(sanitize_module_name("Data  Access"), "data_access");
    assert_eq!(sanitize_module_name("I/O   Utilities"), "io_utilities");
}

#[test]
fn test_special_characters() {
    assert_eq!(sanitize_module_name("User's Profile"), "users_profile");
    assert_eq!(sanitize_module_name("Data-Access-Layer"), "data_access_layer");
}

#[test]
fn test_leading_trailing_underscores() {
    assert_eq!(sanitize_module_name("_utilities_"), "utilities");
    assert_eq!(sanitize_module_name("__internal__"), "internal");
}

#[test]
fn test_empty_and_whitespace() {
    assert_eq!(sanitize_module_name(""), "");
    assert_eq!(sanitize_module_name("   "), "");
}

#[test]
fn test_reserved_keywords() {
    assert_eq!(ensure_not_reserved("mod".to_string()), "mod_module");
    assert_eq!(ensure_not_reserved("type".to_string()), "type_module");
}

#[test]
fn test_collision_resolution() {
    let mut existing = HashSet::new();
    existing.insert("utilities".to_string());

    assert_eq!(ensure_unique_name("utilities".to_string(), &existing), "utilities_1");

    existing.insert("utilities_1".to_string());
    assert_eq!(ensure_unique_name("utilities".to_string(), &existing), "utilities_2");
}

#[test]
fn test_directory_paths() {
    assert_eq!(sanitize_module_name("config/misc"), "config/misc");
    assert_eq!(sanitize_module_name("src/utils"), "src/utils");
}
```

### Integration Tests

**Test**: God object detection still works correctly
```rust
#[test]
fn test_god_object_recommendations_have_valid_names() {
    let detector = GodObjectDetector::new();
    let ast = parse_rust_file(GOD_OBJECT_EXAMPLE);
    let analysis = detector.analyze_enhanced(&ast);

    for split in &analysis.file_metrics.recommended_splits {
        // All names should be valid identifiers
        assert!(is_valid_module_name(&split.suggested_name));
        assert!(!split.suggested_name.contains('&'));
        assert!(!split.suggested_name.contains("  "));
    }
}
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn sanitized_names_are_valid(s in "\\PC{0,100}") {
        let result = sanitize_module_name(&s);

        // No special characters except underscore
        prop_assert!(result.chars().all(|c| c.is_alphanumeric() || c == '_'));

        // No leading/trailing underscores
        if !result.is_empty() {
            prop_assert!(!result.starts_with('_'));
            prop_assert!(!result.ends_with('_'));
        }

        // No consecutive underscores
        prop_assert!(!result.contains("__"));
    }
}
```

## Documentation Requirements

**Code Documentation**:
- Inline documentation for `sanitize_module_name()` with examples
- Document transformation rules and edge cases
- Explain collision resolution strategy

**User Documentation**:
- Update god object detection documentation
- Add note about automatic name sanitization
- Provide examples of transformations

## Implementation Notes

**Performance Considerations**:
- Use single-pass string transformation where possible
- Avoid repeated allocations with `.collect()` and `.join()`
- Consider caching results if called repeatedly with same input

**Edge Cases**:
- Empty strings
- Strings with only special characters
- Very long names (>100 characters)
- Unicode characters (normalize to ASCII if needed)
- Names that become empty after sanitization

**Future Enhancements**:
- Configurable transformation rules
- Language-specific sanitization strategies
- Preserve semantic meaning better (e.g., "I/O" → "io" not "i_o")

## Migration and Compatibility

**Breaking Changes**: None - this is a bug fix

**Migration**: Automatic - no user action required

**Compatibility**:
- Existing tests will see module names change in output
- Update test expectations to use sanitized names
- Golden file tests may need regeneration

## Success Metrics

- Zero invalid module names in god object recommendations
- No reported issues with using recommended module names
- All existing tests pass after implementation
- Sanitization overhead < 1% of total god object detection time
