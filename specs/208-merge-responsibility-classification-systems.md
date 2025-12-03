---
number: 208
title: Merge Dual Responsibility Classification Systems
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-02
---

# Specification 208: Merge Dual Responsibility Classification Systems

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently has **two separate systems** for classifying method responsibilities in god object analysis:

1. **RESPONSIBILITY_CATEGORIES** (in `src/organization/god_object_analysis.rs:1202-1263`)
   - String-based constant array with 12 categories
   - Returns lowercase names: `"validation"`, `"parsing"`, etc.
   - Used as primary classification path with 0.85 confidence
   - Simple prefix matching logic

2. **BehavioralCategorizer** (in `src/organization/behavioral_decomposition.rs:9-249`)
   - Type-safe enum-based system with 7 categories + Domain fallback
   - Returns Title Case names via `display_name()`: `"Validation"`, `"Parsing"`, etc.
   - Used as fallback classification with 0.65 confidence
   - More sophisticated matching (prefix + contains checks)

### The Problem

When analyzing code, different methods in the same file can be classified by different systems, leading to **duplicate responsibilities with different capitalizations**:

```
Responsibilities: unclassified, transformation, Lifecycle, filtering,
                 Persistence, validation, parsing, computation,
                 Validation, Rendering
```

Notice:
- `validation` (lowercase) from RESPONSIBILITY_CATEGORIES
- `Validation` (Title Case) from BehavioralCategorizer

This happens because:
1. `infer_responsibility_with_confidence` tries RESPONSIBILITY_CATEGORIES first
2. If no match or low confidence, it falls back to BehavioralCategorizer
3. Both systems can classify different methods in the same analysis
4. No normalization occurs before storing in `Vec<String>`
5. HTML output displays duplicate categories

### Root Cause Analysis

**Classification Flow:**
```rust
infer_responsibility_with_confidence(method_name)
  ├─ Try RESPONSIBILITY_CATEGORIES first
  │   ├─ Match found → return "validation" (lowercase)
  │   └─ No match → continue to fallback
  └─ Fallback to BehavioralCategorizer
      └─ return category.display_name() → "Validation" (Title Case)
```

**Why This Violates Best Practices:**

From Stillwater philosophy (`../stillwater/PHILOSOPHY.md`):

1. **Composition Over Complexity**: "Build complex behavior from simple, composable pieces" - Having two systems for the same pure function (`method_name → category`) is unnecessary complexity.

2. **Types Guide, Don't Restrict**: "Use types to make wrong code hard to write" - String-based categories allow typos; enum-based is objectively safer.

3. **Pure Core Principle**: Classification is pure logic with no I/O. Having dual paths for the same concern is an architectural smell.

### Current System Comparison

| Aspect | RESPONSIBILITY_CATEGORIES | BehavioralCategorizer |
|--------|--------------------------|----------------------|
| **Type Safety** | ❌ Strings (`"validation"`) | ✅ Enum (`BehaviorCategory::Validation`) |
| **Categories** | 12 specific categories | 7 behavioral + Domain fallback |
| **Matching** | Simple prefixes only | Sophisticated (prefix + contains) |
| **Confidence** | 0.85 (high) | 0.65 (medium) |
| **Output** | lowercase | Title Case |
| **Structure** | Flat const array | Structured with predicate functions |
| **Extensibility** | Hard (modify const) | Easy (add enum variant) |
| **Compiler Support** | None (runtime strings) | Full (exhaustive matching) |

## Objective

**Merge both classification systems into a single, unified, type-safe responsibility classification system based on the BehavioralCategorizer pattern**, eliminating:
1. Capitalization inconsistencies
2. Code duplication
3. Classification ambiguity
4. Maintenance burden

The unified system should:
- Use enum-based categories for type safety
- Support all categories from both systems
- Provide confidence scoring
- Maintain backwards compatibility with existing output
- Follow Stillwater philosophy (pure core, composition, type-driven)

## Requirements

### Functional Requirements

1. **Enum Expansion**
   - Extend `BehaviorCategory` enum with all categories from RESPONSIBILITY_CATEGORIES
   - Add missing categories: Parsing, Filtering, Transformation, DataAccess, Construction, Processing, Communication
   - Keep existing categories: Lifecycle, StateManagement, Rendering, EventHandling, Persistence, Validation, Computation
   - Retain Domain(String) for unclassified/domain-specific methods

2. **Predicate Functions**
   - Implement `is_<category>` predicate functions for all new categories
   - Use both prefix and contains matching (following existing pattern)
   - Include all keywords from RESPONSIBILITY_CATEGORIES prefix lists
   - Maintain existing predicate logic for current categories

3. **Confidence Scoring**
   - Port confidence scoring logic from `infer_responsibility_with_confidence`
   - Assign high confidence (0.85) for recognized pattern matches
   - Assign medium confidence (0.65) for behavioral pattern matches
   - Assign low confidence (0.45) for Domain category fallback
   - Apply MINIMUM_CONFIDENCE (0.50) threshold
   - Apply UTILITIES_THRESHOLD (0.60) if "utilities" category is kept

4. **Display Name Consistency**
   - All categories return Title Case via `display_name()`: "Validation", "Parsing", etc.
   - Special categories with spaces: "Data Access", "State Management", "Event Handling"
   - Domain categories capitalize first letter: `Domain("auth") → "Auth"`

5. **Single Classification Path**
   - Update `infer_responsibility_with_confidence` to use only BehavioralCategorizer
   - Remove RESPONSIBILITY_CATEGORIES lookup path entirely
   - Maintain same function signature and return type (`ClassificationResult`)
   - Keep existing confidence thresholds

6. **Deprecation and Removal**
   - Mark RESPONSIBILITY_CATEGORIES as deprecated with compiler warning
   - Remove RESPONSIBILITY_CATEGORIES constant after unified system is validated
   - Mark old `infer_responsibility_from_method` function as deprecated (already done)
   - Remove ResponsibilityCategory struct if no longer needed

### Non-Functional Requirements

1. **Type Safety**: All category names must be enum variants (compile-time checked)
2. **Performance**: Classification should have same O(n) complexity as before (n = number of categories)
3. **Backwards Compatibility**: Existing code using `display_name()` should continue working
4. **Maintainability**: Adding new categories should only require:
   - Adding enum variant
   - Implementing predicate function
   - Updating `display_name()` match
   - Updating `categorize_method()` match
5. **Documentation**: All new categories must be documented with examples

## Acceptance Criteria

- [ ] `BehaviorCategory` enum includes all 14 categories (7 existing + 7 new)
- [ ] Each category has a predicate function with comprehensive keyword coverage
- [ ] `categorize_method()` checks all categories with appropriate precedence
- [ ] `display_name()` returns consistent Title Case for all categories
- [ ] `infer_responsibility_with_confidence()` uses only BehavioralCategorizer
- [ ] RESPONSIBILITY_CATEGORIES constant is removed
- [ ] ResponsibilityCategory struct is removed (if unused)
- [ ] All existing tests pass
- [ ] New tests verify no duplicate responsibilities in output
- [ ] Running debtmap on itself shows unique, Title Case responsibilities
- [ ] HTML output displays clean responsibility lists (e.g., "Validation, Parsing, Filtering" not "validation, Validation")
- [ ] Performance regression tests show <5% overhead change
- [ ] Code coverage for new predicate functions is ≥85%

## Technical Details

### Implementation Approach

**Phase 1: Expand BehaviorCategory Enum**

```rust
// src/organization/behavioral_decomposition.rs

pub enum BehaviorCategory {
    // Existing categories
    Lifecycle,
    StateManagement,
    Rendering,
    EventHandling,
    Persistence,
    Validation,
    Computation,

    // New categories from RESPONSIBILITY_CATEGORIES
    Parsing,          // parse, read, extract, decode, deserialize, unmarshal, scan
    Filtering,        // filter, select, find, search, query, lookup, match
    Transformation,   // transform, convert, map, apply, adapt
    DataAccess,       // get, set, fetch, retrieve, access
    Construction,     // create, build, new, make, construct
    Processing,       // process, handle, execute, run
    Communication,    // send, receive, transmit, broadcast, notify

    // Fallback for unclassified
    Domain(String),
}
```

**Phase 2: Implement Predicate Functions**

```rust
impl BehavioralCategorizer {
    fn is_parsing(name: &str) -> bool {
        const PARSING_KEYWORDS: &[&str] = &[
            "parse", "read", "extract", "decode",
            "deserialize", "unmarshal", "scan"
        ];
        PARSING_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_filtering(name: &str) -> bool {
        const FILTERING_KEYWORDS: &[&str] = &[
            "filter", "select", "find", "search",
            "query", "lookup", "match"
        ];
        FILTERING_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_transformation(name: &str) -> bool {
        const TRANSFORMATION_KEYWORDS: &[&str] = &[
            "transform", "convert", "map", "apply", "adapt"
        ];
        TRANSFORMATION_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_data_access(name: &str) -> bool {
        const DATA_ACCESS_KEYWORDS: &[&str] = &[
            "get", "set", "fetch", "retrieve", "access"
        ];
        DATA_ACCESS_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_construction(name: &str) -> bool {
        const CONSTRUCTION_KEYWORDS: &[&str] = &[
            "create", "build", "new", "make", "construct"
        ];
        CONSTRUCTION_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_processing(name: &str) -> bool {
        const PROCESSING_KEYWORDS: &[&str] = &[
            "process", "handle", "execute", "run"
        ];
        PROCESSING_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }

    fn is_communication(name: &str) -> bool {
        const COMMUNICATION_KEYWORDS: &[&str] = &[
            "send", "receive", "transmit", "broadcast", "notify"
        ];
        COMMUNICATION_KEYWORDS
            .iter()
            .any(|&kw| name.starts_with(kw) || name.contains(&format!("_{}", kw)))
    }
}
```

**Phase 3: Update categorize_method()**

```rust
impl BehavioralCategorizer {
    pub fn categorize_method(method_name: &str) -> BehaviorCategory {
        let lower_name = method_name.to_lowercase();

        // Order matters: check more specific categories first

        // Construction (before lifecycle to catch "create_*")
        if Self::is_construction(&lower_name) {
            return BehaviorCategory::Construction;
        }

        // Lifecycle methods
        if Self::is_lifecycle(&lower_name) {
            return BehaviorCategory::Lifecycle;
        }

        // Parsing (check early as it's common)
        if Self::is_parsing(&lower_name) {
            return BehaviorCategory::Parsing;
        }

        // Rendering/Display methods
        if Self::is_rendering(&lower_name) {
            return BehaviorCategory::Rendering;
        }

        // Event handling methods
        if Self::is_event_handling(&lower_name) {
            return BehaviorCategory::EventHandling;
        }

        // Persistence methods
        if Self::is_persistence(&lower_name) {
            return BehaviorCategory::Persistence;
        }

        // Validation methods
        if Self::is_validation(&lower_name) {
            return BehaviorCategory::Validation;
        }

        // Computation methods
        if Self::is_computation(&lower_name) {
            return BehaviorCategory::Computation;
        }

        // Filtering methods
        if Self::is_filtering(&lower_name) {
            return BehaviorCategory::Filtering;
        }

        // Transformation methods
        if Self::is_transformation(&lower_name) {
            return BehaviorCategory::Transformation;
        }

        // Data access methods
        if Self::is_data_access(&lower_name) {
            return BehaviorCategory::DataAccess;
        }

        // Processing methods
        if Self::is_processing(&lower_name) {
            return BehaviorCategory::Processing;
        }

        // Communication methods
        if Self::is_communication(&lower_name) {
            return BehaviorCategory::Communication;
        }

        // State management methods
        if Self::is_state_management(&lower_name) {
            return BehaviorCategory::StateManagement;
        }

        // Default: domain-specific based on first word (capitalized)
        let domain = method_name
            .split('_')
            .next()
            .filter(|s| !s.is_empty())
            .map(capitalize_first)
            .unwrap_or_else(|| "Operations".to_string());
        BehaviorCategory::Domain(domain)
    }
}
```

**Phase 4: Update display_name()**

```rust
impl BehaviorCategory {
    pub fn display_name(&self) -> String {
        match self {
            BehaviorCategory::Lifecycle => "Lifecycle".to_string(),
            BehaviorCategory::StateManagement => "State Management".to_string(),
            BehaviorCategory::Rendering => "Rendering".to_string(),
            BehaviorCategory::EventHandling => "Event Handling".to_string(),
            BehaviorCategory::Persistence => "Persistence".to_string(),
            BehaviorCategory::Validation => "Validation".to_string(),
            BehaviorCategory::Computation => "Computation".to_string(),
            BehaviorCategory::Parsing => "Parsing".to_string(),
            BehaviorCategory::Filtering => "Filtering".to_string(),
            BehaviorCategory::Transformation => "Transformation".to_string(),
            BehaviorCategory::DataAccess => "Data Access".to_string(),
            BehaviorCategory::Construction => "Construction".to_string(),
            BehaviorCategory::Processing => "Processing".to_string(),
            BehaviorCategory::Communication => "Communication".to_string(),
            BehaviorCategory::Domain(name) => name.clone(),
        }
    }
}
```

**Phase 5: Simplify infer_responsibility_with_confidence()**

```rust
// src/organization/god_object_analysis.rs

pub fn infer_responsibility_with_confidence(
    method_name: &str,
    _method_body: Option<&str>,
) -> ClassificationResult {
    use crate::organization::BehavioralCategorizer;

    let category = BehavioralCategorizer::categorize_method(method_name);
    let category_name = category.display_name();

    // Assign confidence based on category type
    let confidence = match category {
        crate::organization::BehaviorCategory::Domain(_) => 0.45, // Below threshold
        _ => 0.85, // High confidence for recognized patterns
    };

    // Apply confidence thresholds
    if confidence < MINIMUM_CONFIDENCE {
        log::debug!(
            "Low confidence classification for method '{}': confidence {:.2} below minimum {:.2}",
            method_name,
            confidence,
            MINIMUM_CONFIDENCE
        );
        return ClassificationResult {
            category: None,
            confidence,
            signals_used: vec![SignalType::NameHeuristic],
        };
    }

    ClassificationResult {
        category: Some(category_name),
        confidence,
        signals_used: vec![SignalType::NameHeuristic],
    }
}
```

**Phase 6: Remove Old Code**

```rust
// Delete from god_object_analysis.rs:
// - RESPONSIBILITY_CATEGORIES constant (lines 1202-1263)
// - ResponsibilityCategory struct (if defined)
// - Old classification logic in infer_responsibility_with_confidence
```

### Architecture Changes

**Before (Dual System):**
```
method_name
    ↓
infer_responsibility_with_confidence()
    ├─→ RESPONSIBILITY_CATEGORIES (primary)
    │       → "validation" (lowercase string)
    └─→ BehavioralCategorizer (fallback)
            → "Validation" (Title Case via enum)
                ↓
        HashMap<String, Vec<String>>
                ↓
        Duplicate keys: "validation" AND "Validation"
```

**After (Unified System):**
```
method_name
    ↓
infer_responsibility_with_confidence()
    └─→ BehavioralCategorizer::categorize_method()
            → BehaviorCategory::Validation (enum)
                    ↓
            display_name() → "Validation" (Title Case)
                    ↓
        HashMap<String, Vec<String>>
                    ↓
        Unique keys: "Validation" only
```

### Data Structures

**Enum Equality and Hashing:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BehaviorCategory {
    // ... variants
}
```

The `Domain(String)` variant allows flexible fallback while maintaining type safety.

### Category Precedence Order

The order in `categorize_method()` matters for ambiguous cases:

1. **Construction** (before Lifecycle to catch `create_*`)
2. **Lifecycle** (fundamental operations)
3. **Parsing** (early, as it's common)
4. **Rendering** (output/display)
5. **Event Handling** (interaction)
6. **Persistence** (storage)
7. **Validation** (checking)
8. **Computation** (calculation)
9. **Filtering** (search/query)
10. **Transformation** (convert/map)
11. **Data Access** (get/set)
12. **Processing** (handle/execute)
13. **Communication** (send/receive)
14. **State Management** (state)
15. **Domain** (fallback)

### Confidence Scoring Strategy

| Category | Confidence | Rationale |
|----------|-----------|-----------|
| Specific behavioral pattern | 0.85 | High confidence: clear keyword match |
| Domain(String) | 0.45 | Low confidence: fallback classification |
| Minimum threshold | 0.50 | Balance precision vs recall |

## Dependencies

**Prerequisites**: None

**Affected Components**:
- `src/organization/behavioral_decomposition.rs` - Extend BehaviorCategory enum
- `src/organization/god_object_analysis.rs` - Simplify classification logic
- `src/organization/god_object_detector.rs` - Uses classification results (no changes needed)

**External Dependencies**: None (pure refactoring)

## Testing Strategy

### Unit Tests

1. **Category Predicate Tests** (`test_is_<category>_predicate`)
   ```rust
   #[test]
   fn test_is_parsing_predicate() {
       assert!(BehavioralCategorizer::is_parsing("parse_json"));
       assert!(BehavioralCategorizer::is_parsing("read_file"));
       assert!(BehavioralCategorizer::is_parsing("extract_data"));
       assert!(!BehavioralCategorizer::is_parsing("validate_input"));
   }
   ```

2. **Categorization Tests** (`test_categorize_<category>_methods`)
   ```rust
   #[test]
   fn test_categorize_parsing_methods() {
       assert_eq!(
           BehavioralCategorizer::categorize_method("parse_json"),
           BehaviorCategory::Parsing
       );
       assert_eq!(
           BehavioralCategorizer::categorize_method("deserialize_config"),
           BehaviorCategory::Parsing
       );
   }
   ```

3. **Display Name Tests**
   ```rust
   #[test]
   fn test_display_names_title_case() {
       assert_eq!(
           BehaviorCategory::Validation.display_name(),
           "Validation"
       );
       assert_eq!(
           BehaviorCategory::DataAccess.display_name(),
           "Data Access"
       );
   }
   ```

4. **No Duplication Tests**
   ```rust
   #[test]
   fn test_no_duplicate_responsibilities() {
       let methods = vec![
           "validate_input".to_string(),
           "check_permissions".to_string(),
           "verify_token".to_string(),
       ];
       let groups = group_methods_by_responsibility(&methods);

       // Should only have one "Validation" key, not "validation" and "Validation"
       let validation_keys: Vec<_> = groups
           .keys()
           .filter(|k| k.to_lowercase() == "validation")
           .collect();
       assert_eq!(validation_keys.len(), 1);
       assert_eq!(*validation_keys[0], "Validation"); // Title Case
   }
   ```

5. **Confidence Scoring Tests**
   ```rust
   #[test]
   fn test_confidence_scoring() {
       let result = infer_responsibility_with_confidence("validate_email", None);
       assert!(result.category.is_some());
       assert_eq!(result.confidence, 0.85);

       let domain_result = infer_responsibility_with_confidence("foo_bar", None);
       assert!(domain_result.category.is_none()); // Below 0.50 threshold
       assert_eq!(domain_result.confidence, 0.45);
   }
   ```

### Integration Tests

1. **God Object Analysis Test**
   ```rust
   #[test]
   fn test_god_object_analysis_unique_responsibilities() {
       // Analyze a file with mixed validation methods
       let analysis = analyze_god_object_in_file("test_data/mixed_validation.rs");

       // Check responsibilities are unique and Title Case
       let responsibilities: HashSet<_> = analysis.responsibilities.iter().collect();
       assert_eq!(
           responsibilities.len(),
           analysis.responsibilities.len(),
           "Responsibilities should be unique"
       );

       // All should be Title Case
       for resp in &analysis.responsibilities {
           assert_eq!(
               resp.chars().next().unwrap().to_uppercase().to_string(),
               resp.chars().next().unwrap().to_string(),
               "Responsibility '{}' should be Title Case",
               resp
           );
       }
   }
   ```

2. **Self-Analysis Test**
   ```rust
   #[test]
   fn test_debtmap_self_analysis_no_duplicates() {
       // Run debtmap on itself
       let output = Command::new("cargo")
           .args(&["run", "--", "analyze", "--format", "html"])
           .output()
           .expect("Failed to run debtmap");

       // Parse HTML output
       let html = String::from_utf8_lossy(&output.stdout);

       // Check for duplicate responsibilities (case-insensitive)
       // Should not see both "validation" and "Validation"
       assert!(
           !html.contains("validation, Validation") &&
           !html.contains("Validation, validation")
       );
   }
   ```

### Performance Tests

```rust
#[bench]
fn bench_unified_classification(b: &mut Bencher) {
    let methods = vec![
        "validate_input",
        "parse_json",
        "filter_results",
        "transform_data",
        "get_value",
        // ... 100 methods
    ];

    b.iter(|| {
        for method in &methods {
            black_box(infer_responsibility_with_confidence(method, None));
        }
    });
}
```

### User Acceptance Tests

1. Run debtmap on its own codebase: `cargo run -- analyze --format html`
2. Open generated HTML in browser
3. Navigate to god object analysis section
4. Verify responsibilities list:
   - ✅ All responsibilities are Title Case
   - ✅ No duplicate responsibilities (e.g., "validation" and "Validation")
   - ✅ Responsibilities make semantic sense
   - ✅ Count matches number of unique categories

## Documentation Requirements

### Code Documentation

1. **Module-level docs** for `behavioral_decomposition.rs`:
   ```rust
   //! Unified behavioral categorization for method responsibility inference.
   //!
   //! This module provides a type-safe, enum-based system for classifying
   //! methods and functions by their behavioral category. It replaces the
   //! previous dual-system approach (RESPONSIBILITY_CATEGORIES + BehavioralCategorizer)
   //! with a single, consistent classification path.
   ```

2. **Enum documentation**:
   ```rust
   /// Behavioral category for method classification.
   ///
   /// Categories are inferred from method names using keyword matching.
   /// Each category has associated predicate functions and keyword lists.
   ///
   /// # Categories
   ///
   /// - **Lifecycle**: Object creation and destruction (new, create, destroy, close)
   /// - **Parsing**: Data reading and extraction (parse, read, extract, decode)
   /// - **Validation**: Checking and verification (validate, check, verify, ensure)
   /// - **Computation**: Calculations (calculate, compute, evaluate, measure)
   /// - ... [document all categories]
   ```

3. **Function documentation** for each predicate:
   ```rust
   /// Check if a method name indicates parsing behavior.
   ///
   /// # Keywords
   /// - parse, read, extract, decode, deserialize, unmarshal, scan
   ///
   /// # Examples
   /// ```
   /// assert!(is_parsing("parse_json"));
   /// assert!(is_parsing("extract_metadata"));
   /// ```
   fn is_parsing(name: &str) -> bool
   ```

### User Documentation

Update relevant sections in:

1. **README.md** (if it mentions classification):
   - Note that responsibility classification uses behavioral pattern matching
   - Explain confidence scoring briefly

2. **ARCHITECTURE.md**:
   - Document unified classification system architecture
   - Explain enum-based type safety benefits
   - Describe confidence scoring strategy

3. **CHANGELOG.md**:
   ```markdown
   ## [Unreleased]

   ### Fixed
   - Fixed duplicate responsibilities in god object analysis (e.g., "validation" and "Validation")

   ### Changed
   - Merged dual responsibility classification systems into single enum-based system
   - All responsibility names now use consistent Title Case formatting
   - Improved classification confidence scoring

   ### Removed
   - Removed deprecated RESPONSIBILITY_CATEGORIES constant
   - Removed ResponsibilityCategory struct
   ```

## Implementation Notes

### Order of Implementation

1. **Start with behavioral_decomposition.rs**:
   - Add new enum variants
   - Implement predicate functions
   - Update `categorize_method()`
   - Update `display_name()`
   - Add unit tests

2. **Update god_object_analysis.rs**:
   - Simplify `infer_responsibility_with_confidence()`
   - Add confidence scoring logic
   - Test with existing code

3. **Verify with integration tests**:
   - Run full test suite
   - Check for regressions
   - Validate HTML output

4. **Remove old code**:
   - Delete RESPONSIBILITY_CATEGORIES
   - Delete ResponsibilityCategory struct
   - Clean up unused imports

5. **Documentation and cleanup**:
   - Update docs
   - Update CHANGELOG
   - Final verification

### Edge Cases to Handle

1. **Method names with multiple keywords**:
   - Example: `parse_and_validate_input`
   - Solution: First match wins (parsing beats validation if checked first)

2. **Ambiguous prefixes**:
   - Example: `read_*` could be parsing OR data access
   - Solution: Give parsing precedence (checked earlier)

3. **Domain fallback with common names**:
   - Example: `get_parser` → Should be data access, not Domain("Get")
   - Solution: Check data access before domain fallback

4. **Empty or invalid method names**:
   - Example: `""`, `_`, `__`
   - Solution: Domain("Operations") fallback

### Performance Considerations

- Predicate functions are called sequentially (worst case: 14 checks)
- Each check is O(k) where k = number of keywords (typically 3-7)
- Overall: O(14 * 7) = O(98) = O(1) constant time
- No performance regression expected

### Gotchas

1. **Order matters in `categorize_method()`**: More specific categories should be checked before general ones
2. **Predicate functions must use `lower_name`**: Already lowercase from `categorize_method()`
3. **`display_name()` must handle all variants**: Compiler enforces exhaustive matching
4. **Domain(String) should capitalize**: Use `capitalize_first()` helper

## Migration and Compatibility

### Breaking Changes

**None**. This is an internal refactoring. The public API remains unchanged:
- `infer_responsibility_with_confidence()` signature unchanged
- `ClassificationResult` structure unchanged
- `group_methods_by_responsibility()` signature unchanged
- HTML output format unchanged (just cleaner content)

### Migration Path

Not applicable - internal refactoring only.

### Backwards Compatibility

- Existing code calling these functions continues to work
- Existing tests continue to pass
- Output format remains the same (just with unique, Title Case names)
- Serialization format unchanged (still `Vec<String>`)

### Deprecation Timeline

1. **Immediate**: Mark RESPONSIBILITY_CATEGORIES as deprecated (this spec)
2. **Same PR**: Remove RESPONSIBILITY_CATEGORIES (low risk, internal only)
3. **Future**: Consider migrating to enum-based serialization (separate spec)

## Success Metrics

1. **Zero duplicate responsibilities** in god object analysis output
2. **100% Title Case** responsibility names
3. **All tests pass** including new unit and integration tests
4. **Code coverage ≥85%** for new predicate functions
5. **Performance regression <5%** in god object analysis benchmarks
6. **Clean HTML output** when running debtmap on itself

## Related Issues

- Original bug report: Duplicate "validation"/"Validation" in god object analysis
- Philosophy alignment: Follows Stillwater principles (pure core, type safety, composition)
- Code quality: Reduces technical debt, improves maintainability

---

**Implementation Estimate**: 4-6 hours
- Enum expansion: 1 hour
- Predicate functions: 2 hours
- Testing: 2 hours
- Documentation: 1 hour
