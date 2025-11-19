# Spec 180: Shared Utilities for Architecture Analysis

**Status**: Draft
**Priority**: Foundation
**Used By**: [181, 182, 183, 184, 185]
**Created**: 2025-01-19

## Context

Specs 181-184 duplicate several utility functions (case conversion, type analysis, noun extraction). This creates maintenance burden and risks inconsistency.

## Objective

Create a shared utilities module with common functions used across all architecture analysis specs.

## Implementation

```rust
// src/organization/architecture_utils.rs

/// Shared utilities for architecture analysis (Specs 181-185)
use std::collections::HashMap;

// ============================================================================
// Case Conversion
// ============================================================================

/// Convert snake_case to PascalCase
///
/// Examples:
/// - `priority_item` → `PriorityItem`
/// - `god_object_metrics` → `GodObjectMetrics`
pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Convert PascalCase/camelCase to snake_case
///
/// Examples:
/// - `PriorityItem` → `priority_item`
/// - `GodObjectMetrics` → `god_object_metrics`
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

// ============================================================================
// Type Analysis
// ============================================================================

/// Check if type is primitive or standard library type
///
/// Used by:
/// - Spec 181: Filtering non-domain types in affinity calculation
/// - Spec 183: Detecting mixed data types
/// - Spec 184: Extracting meaningful field names
pub fn is_primitive_type(type_name: &str) -> bool {
    matches!(
        type_name,
        // Primitives
        "String" | "str" | "usize" | "isize" | "u32" | "i32" | "u64" | "i64" |
        "u8" | "i8" | "u16" | "i16" | "u128" | "i128" |
        "f32" | "f64" | "bool" | "char" | "()" |
        // Standard library generics
        "Vec" | "Option" | "Result" | "Box" | "Rc" | "Arc" |
        "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet" |
        "VecDeque" | "LinkedList" | "BinaryHeap" |
        // Path types (context-dependent)
        "Path" | "PathBuf" | "OsString" | "OsStr" |
        // IO types
        "File" | "BufReader" | "BufWriter" |
        // Smart pointers
        "Cow" | "RefCell" | "Cell" | "Mutex" | "RwLock" |
        "Error"
    ) || type_name.starts_with('&')
}

/// Check if type is domain-specific (not primitive/stdlib)
pub fn is_domain_type(type_name: &str) -> bool {
    !is_primitive_type(type_name)
}

/// Extract base type from generic wrappers
///
/// Examples:
/// - `Option<Metrics>` → `Metrics`
/// - `Vec<Item>` → `Item`
/// - `Result<Data, Error>` → `Data` (first type param)
/// - `&mut String` → `String`
pub fn extract_base_type(type_name: &str) -> String {
    let mut working = type_name;

    // Strip references
    working = working.trim_start_matches('&').trim_start_matches("mut ").trim();

    // Extract from generics
    if let Some(start) = working.find('<') {
        if let Some(end) = working.rfind('>') {
            let inner = &working[start + 1..end];
            // For multi-generic types, take first
            if let Some(comma) = inner.find(',') {
                return inner[..comma].trim().to_string();
            }
            return inner.trim().to_string();
        }
    }

    working.to_string()
}

/// Check if two types match, handling generic wrappers
///
/// Examples:
/// - `Metrics` matches `Metrics` (exact)
/// - `Option<Metrics>` matches `Metrics` (unwrap)
/// - `&str` matches `String` (equivalent)
pub fn types_equivalent(type1: &str, type2: &str) -> bool {
    if type1 == type2 {
        return true;
    }

    // Normalize and compare
    let norm1 = normalize_type_for_comparison(type1);
    let norm2 = normalize_type_for_comparison(type2);

    norm1 == norm2
}

fn normalize_type_for_comparison(type_name: &str) -> String {
    let base = extract_base_type(type_name);

    // str → String
    if base == "str" {
        "String".to_string()
    } else {
        base
    }
}

// ============================================================================
// Noun Extraction
// ============================================================================

/// Extract core noun from compound type names
///
/// Examples:
/// - `SourceLocation` → `Location`
/// - `FileMetrics` → `Metrics`
/// - `HttpRequestHandler` → `Request`
/// - `UserData` → `User`
///
/// Used by Spec 184 for type name inference
pub fn extract_noun(type_name: &str) -> String {
    // Common suffixes to remove
    const SUFFIXES: &[&str] = &[
        "Location", "Metrics", "Data", "Info", "Details",
        "Handler", "Manager", "Service", "Provider", "Factory",
        "Builder", "Analyzer", "Processor", "Controller",
        "Context", "Config", "Settings", "Result"
    ];

    for suffix in SUFFIXES {
        if type_name.ends_with(suffix) && type_name.len() > suffix.len() {
            return type_name[..type_name.len() - suffix.len()].to_string();
        }
    }

    // No suffix found, return as-is
    type_name.to_string()
}

/// Check if word is likely a verb (action) vs noun (data)
///
/// Used by Spec 183 for detecting technical groupings
pub fn is_likely_verb(word: &str) -> bool {
    // Verbal noun suffixes
    word.ends_with("ing") ||     // rendering, parsing
    word.ends_with("tion") ||    // calculation, validation
    word.ends_with("ment") ||    // management, placement
    word.ends_with("sion") ||    // conversion, extension
    word.ends_with("ance") ||    // performance, maintenance
    word.ends_with("ence") ||    // reference, persistence
    // Known action words
    matches!(
        word,
        "calculate" | "compute" | "process" | "handle" | "manage" |
        "render" | "format" | "display" | "show" | "print" |
        "validate" | "check" | "verify" | "ensure" |
        "parse" | "transform" | "convert" | "serialize" | "deserialize" |
        "get" | "set" | "update" | "modify" | "create" | "delete" |
        "authenticate" | "authorize" | "encrypt" | "decrypt"
    )
}

/// Check if word is a domain term (noun)
pub fn is_domain_term(word: &str) -> bool {
    // Domain suffixes
    word.ends_with("metrics") ||
    word.ends_with("data") ||
    word.ends_with("config") ||
    word.ends_with("settings") ||
    word.ends_with("context") ||
    word.ends_with("item") ||
    word.ends_with("result") ||
    // Plural nouns
    (word.ends_with('s') && !word.ends_with("ss")) ||
    // Known domain nouns
    matches!(
        word,
        "priority" | "god_object" | "debt" | "complexity" |
        "coverage" | "analysis" | "report" | "summary"
    )
}

// ============================================================================
// Collection Utilities
// ============================================================================

/// Find most common element in collection
///
/// Used by Spec 183 for verb detection
pub fn most_common<T: std::hash::Hash + Eq + Clone>(items: &[T]) -> Option<T> {
    let mut counts: HashMap<T, usize> = HashMap::new();
    for item in items {
        *counts.entry(item.clone()).or_insert(0) += 1;
    }
    counts.into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(item, _)| item)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("priority_item"), "PriorityItem");
        assert_eq!(to_pascal_case("god_object_metrics"), "GodObjectMetrics");
        assert_eq!(to_pascal_case("single"), "Single");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("PriorityItem"), "priority_item");
        assert_eq!(to_snake_case("GodObjectMetrics"), "god_object_metrics");
        assert_eq!(to_snake_case("single"), "single");
    }

    #[test]
    fn test_extract_base_type() {
        assert_eq!(extract_base_type("Option<Metrics>"), "Metrics");
        assert_eq!(extract_base_type("Vec<Item>"), "Item");
        assert_eq!(extract_base_type("Result<Data, Error>"), "Data");
        assert_eq!(extract_base_type("&mut String"), "String");
        assert_eq!(extract_base_type("String"), "String");
    }

    #[test]
    fn test_types_equivalent() {
        assert!(types_equivalent("Metrics", "Metrics"));
        assert!(types_equivalent("Option<Metrics>", "Metrics"));
        assert!(types_equivalent("&str", "String"));
        assert!(!types_equivalent("Metrics", "Config"));
    }

    #[test]
    fn test_extract_noun() {
        assert_eq!(extract_noun("SourceLocation"), "Source");
        assert_eq!(extract_noun("FileMetrics"), "File");
        assert_eq!(extract_noun("UserData"), "User");
        assert_eq!(extract_noun("Simple"), "Simple");
    }

    #[test]
    fn test_is_likely_verb() {
        assert!(is_likely_verb("rendering"));
        assert!(is_likely_verb("calculation"));
        assert!(is_likely_verb("format"));
        assert!(!is_likely_verb("priority"));
        assert!(!is_likely_verb("metrics"));
    }

    #[test]
    fn test_is_domain_term() {
        assert!(is_domain_term("priority"));
        assert!(is_domain_term("metrics"));
        assert!(is_domain_term("god_objects")); // plural
        assert!(!is_domain_term("rendering"));
        assert!(!is_domain_term("calculation"));
    }

    #[test]
    fn test_most_common() {
        let items = vec!["a", "b", "a", "c", "a", "b"];
        assert_eq!(most_common(&items), Some("a"));

        let empty: Vec<String> = vec![];
        assert_eq!(most_common(&empty), None);
    }
}
```

## Integration

### Update Spec 181

Replace inline implementations with:

```rust
use crate::organization::architecture_utils::{
    to_pascal_case, to_snake_case, extract_base_type,
    is_domain_type, types_equivalent
};
```

### Update Spec 183

Replace inline implementations with:

```rust
use crate::organization::architecture_utils::{
    to_pascal_case, is_likely_verb, is_domain_term,
    most_common, is_primitive_type
};
```

### Update Spec 184

Replace inline implementations with:

```rust
use crate::organization::architecture_utils::{
    to_pascal_case, extract_noun, extract_base_type,
    is_domain_term
};
```

## Module Structure

```
src/organization/
├── architecture_utils.rs         # This spec
├── type_based_clustering.rs      # Spec 181
├── data_flow_analyzer.rs          # Spec 182
├── anti_pattern_detector.rs       # Spec 183
├── hidden_type_extractor.rs       # Spec 184
└── integrated_analyzer.rs         # Spec 185
```

## Benefits

1. **DRY Principle**: Single source of truth for common operations
2. **Consistency**: All specs use same case conversion logic
3. **Testability**: Centralized tests for utilities
4. **Maintainability**: Fix once, benefit everywhere
5. **Performance**: Potential for caching/memoization in one place

## Migration Path

1. Create `architecture_utils.rs` with all shared functions
2. Add comprehensive unit tests
3. Update specs 181-184 to use shared functions
4. Remove duplicated code
5. Verify all tests still pass
