---
number: 173
title: Simplify Responsibility Category Names
category: optimization
priority: high
status: draft
dependencies: [172]
created: 2025-11-10
---

# Specification 173: Simplify Responsibility Category Names

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 172 (Sanitize Module Names)

## Context

The current responsibility categories used for method classification and module split recommendations have verbose, inconsistent names that:

1. **Contain special characters** requiring sanitization ("Parsing & Input")
2. **Are overly verbose** ("Formatting & Output" vs. simpler "output")
3. **Are inconsistent** (some use "&", some use full words)
4. **Generate awkward module names** ("twopassextractor_parsing_&_input")

**Current Names** (`src/organization/god_object_analysis.rs:879`):
```rust
const RESPONSIBILITY_CATEGORIES: &[ResponsibilityCategory] = &[
    ResponsibilityCategory { name: "Formatting & Output", ... },  // ← Verbose, has "&"
    ResponsibilityCategory { name: "Parsing & Input", ... },      // ← Verbose, has "&"
    ResponsibilityCategory { name: "Filtering & Selection", ... }, // ← Redundant
    ResponsibilityCategory { name: "Data Access", ... },           // ← Space in name
    // ... 8 more categories
];
```

**Impact on Output**:
```
Current:  mod_parsing_&_input.rs - Parsing & Input (6 methods)
Better:   mod_parsing.rs - parsing (6 methods)
```

## Objective

Simplify responsibility category names to be:
1. **Short and concise** (single word preferred)
2. **Lowercase-ready** (no spaces or special characters)
3. **Semantically clear** (unambiguous meaning)
4. **Consistent** (follow same naming pattern)
5. **Module-friendly** (suitable for direct use in filenames)

## Requirements

### Functional Requirements

**FR1: Rename Categories**

| Current Name | New Name | Rationale |
|---|---|---|
| "Formatting & Output" | "output" | Shorter, covers both concepts |
| "Parsing & Input" | "parsing" | Shorter, "input" implied by "parsing" |
| "Filtering & Selection" | "filtering" | "selection" is a type of filtering |
| "Transformation" | "transformation" | ✓ Already good |
| "Data Access" | "data_access" | Underscore instead of space |
| "Validation" | "validation" | ✓ Already good |
| "Computation" | "computation" | ✓ Already good |
| "Construction" | "construction" | ✓ Already good |
| "Persistence" | "persistence" | ✓ Already good |
| "Processing" | "processing" | ✓ Already good |
| "Communication" | "communication" | ✓ Already good |
| "Utilities" | "utilities" | ✓ Already good |

**FR2: Update Prefix Matching**
- Extend prefixes to compensate for shorter names
- Ensure no loss of classification accuracy
- Add missing common patterns

**FR3: Backward Compatibility**
- Map old category names to new names internally
- Support both formats in configuration files
- Provide migration path for custom categories

### Non-Functional Requirements

**NFR1: Classification Accuracy**
- Maintain or improve classification accuracy (target: >85%)
- No increase in "Utilities" fallback rate
- Validate against existing test corpus

**NFR2: Naming Consistency**
- All category names follow same pattern (lowercase, underscores for multi-word)
- No special characters in any category name
- Names are directly usable as module names

**NFR3: Documentation**
- Update all documentation references
- Provide migration guide for users with custom categories
- Document rationale for each name change

## Acceptance Criteria

- [ ] All 12 responsibility categories renamed per requirements
- [ ] Category names contain no spaces or special characters
- [ ] Prefix lists updated to maintain classification accuracy
- [ ] All unit tests updated with new category names
- [ ] Integration tests pass with new names
- [ ] Classification accuracy maintained (>85% on test corpus)
- [ ] "Utilities" fallback rate unchanged or decreased
- [ ] Documentation updated with new terminology
- [ ] Migration guide created for custom categories
- [ ] Backward compatibility mapping implemented
- [ ] All compiler warnings resolved

## Technical Details

### Implementation Approach

**Location**: `src/organization/god_object_analysis.rs:879`

**Updated Categories**:
```rust
const RESPONSIBILITY_CATEGORIES: &[ResponsibilityCategory] = &[
    ResponsibilityCategory {
        name: "output",  // Was: "Formatting & Output"
        prefixes: &[
            "format", "render", "write", "print", "display",
            "show", "draw", "output", "emit"  // Added: more patterns
        ],
    },
    ResponsibilityCategory {
        name: "parsing",  // Was: "Parsing & Input"
        prefixes: &[
            "parse", "read", "extract", "decode",
            "deserialize", "unmarshal", "scan"  // Added: more patterns
        ],
    },
    ResponsibilityCategory {
        name: "filtering",  // Was: "Filtering & Selection"
        prefixes: &[
            "filter", "select", "find", "search",
            "query", "lookup", "match"  // Added: more patterns
        ],
    },
    ResponsibilityCategory {
        name: "transformation",  // Unchanged
        prefixes: &["transform", "convert", "map", "apply", "adapt"],
    },
    ResponsibilityCategory {
        name: "data_access",  // Was: "Data Access"
        prefixes: &["get", "set", "fetch", "retrieve", "access"],
    },
    ResponsibilityCategory {
        name: "validation",  // Unchanged
        prefixes: &["validate", "check", "verify", "is", "ensure", "assert"],
    },
    ResponsibilityCategory {
        name: "computation",  // Unchanged
        prefixes: &["calculate", "compute", "evaluate", "measure"],
    },
    ResponsibilityCategory {
        name: "construction",  // Unchanged
        prefixes: &["create", "build", "new", "make", "construct"],
    },
    ResponsibilityCategory {
        name: "persistence",  // Unchanged
        prefixes: &["save", "load", "store", "persist", "cache"],
    },
    ResponsibilityCategory {
        name: "processing",  // Unchanged
        prefixes: &["process", "handle", "execute", "run"],
    },
    ResponsibilityCategory {
        name: "communication",  // Unchanged
        prefixes: &["send", "receive", "transmit", "broadcast", "notify"],
    },
    ResponsibilityCategory {
        name: "utilities",  // Unchanged
        prefixes: &[],  // Empty = catch-all
    },
];
```

### Backward Compatibility Mapping

```rust
/// Map old category names to new names for backward compatibility
pub fn normalize_category_name(old_name: &str) -> String {
    match old_name {
        "Formatting & Output" => "output".to_string(),
        "Parsing & Input" => "parsing".to_string(),
        "Filtering & Selection" => "filtering".to_string(),
        "Data Access" => "data_access".to_string(),
        // Already normalized names pass through
        name => name.to_lowercase().replace(' ', "_"),
    }
}
```

### Affected Functions

**Update Call Sites**:
1. `infer_responsibility_from_method()` - Returns new names
2. `recommend_module_splits_with_evidence()` - Uses new names
3. `classify_struct_domain()` - May reference old names
4. Formatter functions - Display new names

**Test Updates**:
```rust
// Before:
assert_eq!(infer_responsibility_from_method("format_output"), "Formatting & Output");

// After:
assert_eq!(infer_responsibility_from_method("format_output"), "output");
```

## Dependencies

**Prerequisites**:
- Spec 172 (Sanitize Module Names) - Ensures names are valid after simplification

**Affected Components**:
- `src/organization/god_object_analysis.rs` - Category definitions
- `src/organization/module_function_classifier.rs` - Uses categories
- `src/analysis/multi_signal_aggregation.rs` - ResponsibilityCategory enum
- `src/priority/formatter.rs` - Displays category names
- All test files using category names

## Testing Strategy

### Unit Tests

**Test Suite**: `tests/responsibility_classification_test.rs`

```rust
#[test]
fn test_new_category_names() {
    assert_eq!(infer_responsibility_from_method("format_output"), "output");
    assert_eq!(infer_responsibility_from_method("parse_json"), "parsing");
    assert_eq!(infer_responsibility_from_method("filter_results"), "filtering");
    assert_eq!(infer_responsibility_from_method("get_value"), "data_access");
}

#[test]
fn test_category_names_have_no_spaces() {
    for category in RESPONSIBILITY_CATEGORIES {
        assert!(!category.name.contains(' '));
        assert!(!category.name.contains('&'));
    }
}

#[test]
fn test_category_names_are_lowercase() {
    for category in RESPONSIBILITY_CATEGORIES {
        assert_eq!(category.name, category.name.to_lowercase());
    }
}

#[test]
fn test_backward_compatibility_mapping() {
    assert_eq!(normalize_category_name("Formatting & Output"), "output");
    assert_eq!(normalize_category_name("Parsing & Input"), "parsing");
    assert_eq!(normalize_category_name("Data Access"), "data_access");
}
```

### Integration Tests

**Classification Accuracy Test**:
```rust
#[test]
fn test_classification_accuracy_maintained() {
    // Load ground truth corpus (from Spec 150)
    let corpus = load_test_corpus("tests/data/classification_corpus.json");

    let mut correct = 0;
    let mut total = 0;

    for sample in corpus {
        let result = infer_responsibility_from_method(&sample.method_name);
        let expected = normalize_category_name(&sample.expected_category);

        if result == expected {
            correct += 1;
        }
        total += 1;
    }

    let accuracy = correct as f64 / total as f64;
    assert!(accuracy >= 0.85, "Accuracy dropped below 85%: {}", accuracy);
}
```

**Golden File Tests**:
```rust
#[test]
fn test_god_object_output_format() {
    let detector = GodObjectDetector::new();
    let analysis = detector.analyze_enhanced(&PYTHON_TYPE_TRACKER_AST);

    // Verify new category names appear in output
    let split_names: Vec<_> = analysis.file_metrics.recommended_splits
        .iter()
        .map(|s| &s.responsibility)
        .collect();

    assert!(split_names.contains(&"parsing".to_string()));
    assert!(!split_names.iter().any(|n| n.contains('&')));
}
```

### Regression Tests

**Ensure No Loss of Functionality**:
```rust
#[test]
fn test_all_prefixes_still_match() {
    // Test that extended prefix lists don't break existing matches
    assert_eq!(infer_responsibility_from_method("format_text"), "output");
    assert_eq!(infer_responsibility_from_method("parse_xml"), "parsing");
    assert_eq!(infer_responsibility_from_method("filter_items"), "filtering");
}
```

## Documentation Requirements

**Code Documentation**:
- Update inline docs for `RESPONSIBILITY_CATEGORIES`
- Document rationale for name simplification
- Add migration notes for custom categories

**User Documentation**:
- Update `book/src/scoring-strategies.md` with new names
- Update god object detection guide
- Add migration section for v0.3.x → v0.4.0

**Migration Guide**:
```markdown
## Category Name Changes in v0.4.0

The following responsibility categories have been renamed for clarity:

| Old Name | New Name |
|---|---|
| "Formatting & Output" | "output" |
| "Parsing & Input" | "parsing" |
| "Filtering & Selection" | "filtering" |
| "Data Access" | "data_access" |

### Impact

- Module split recommendations will use shorter names
- No action required for most users
- If you have custom category configurations, update them to use new names

### Custom Categories

If you extended `RESPONSIBILITY_CATEGORIES` in your fork:
```rust
// Before
ResponsibilityCategory { name: "Custom & Logic", ... }

// After
ResponsibilityCategory { name: "custom_logic", ... }
```
```

## Implementation Notes

**Order of Changes**:
1. Update `RESPONSIBILITY_CATEGORIES` constant
2. Update all test expectations
3. Add backward compatibility mapping
4. Update documentation
5. Regenerate golden files

**Validation**:
- Run full test suite after each category rename
- Check that no tests hard-code old category names
- Validate formatter output with new names

**Communication**:
- This is a user-visible change
- Include in release notes
- Consider deprecation warnings in v0.3.x if time permits

## Migration and Compatibility

**Breaking Changes**: Yes - category names visible in output

**Deprecation Strategy**:
- Keep backward compatibility mapping for 2 releases
- Log warning when old names detected in config files
- Remove mapping in v0.5.0

**User Migration**:
```bash
# Update custom configurations
sed -i 's/Formatting & Output/output/g' debtmap.toml
sed -i 's/Parsing & Input/parsing/g' debtmap.toml
sed -i 's/Data Access/data_access/g' debtmap.toml
```

**Version Compatibility**:
- v0.3.x: Old names only
- v0.4.x: New names + backward compatibility
- v0.5.x: New names only

## Success Metrics

- All category names are single words or snake_case
- Zero special characters in category names
- Classification accuracy maintained (≥85%)
- "Utilities" fallback rate unchanged or decreased
- Zero test failures after migration
- User-facing output cleaner and more professional
