---
number: 140
title: Use Boilerplate Recommendations in Output
category: optimization
priority: critical
status: draft
dependencies: []
created: 2025-10-27
---

# Specification 140: Use Boilerplate Recommendations in Output

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

**Problem Identified**: The boilerplate detection system is working correctly (detects ripgrep's flags/defs.rs with 87.8% confidence), but the boilerplate-specific recommendations are NOT appearing in the output.

**Root Cause**: The `FileDebtMetrics::generate_recommendation()` function (src/priority/file_metrics.rs:146) only checks the boolean `is_god_object` flag but does not check the actual `GodObjectType` enum variant. When a file is detected as `GodObjectType::BoilerplatePattern`, it still generates generic "split into modules" recommendations instead of using the macro-specific recommendations from the boilerplate detector.

**Current Behavior**:
```
#1 SCORE: 474 [CRITICAL - FILE - HIGH COMPLEXITY]
└─ ./crates/core/flags/defs.rs (7775 lines, 888 functions)
└─ WHY: File exceeds recommended size...
└─ ACTION: Extract complex functions, reduce file to <500 lines
```

**Expected Behavior**:
```
#1 SCORE: 474 [CRITICAL - FILE - BOILERPLATE PATTERN]
└─ ./crates/core/flags/defs.rs (7775 lines, 888 functions)
└─ WHY: BOILERPLATE DETECTED: 104 implementations of Flag trait...
└─ ACTION: Create a declarative macro to generate Flag implementations
   This is NOT a god object requiring module splitting. Focus on reducing
   repetition through macros rather than splitting into multiple files.
```

## Objective

Fix the `generate_recommendation()` function to check for `GodObjectType::BoilerplatePattern` and use the boilerplate-specific recommendations when appropriate.

## Requirements

### Functional Requirements

1. **Check GodObjectType Variant**
   - Modify `FileDebtMetrics::generate_recommendation()` to check the actual `GodObjectType` enum
   - When `GodObjectType::BoilerplatePattern` is detected, use the boilerplate recommendation
   - Preserve existing behavior for `GodObjectType::GodClass` and `GodObjectType::GodFile`

2. **Access Boilerplate Recommendation**
   - The boilerplate recommendation is already generated and stored in the `GodObjectType::BoilerplatePattern` variant
   - Extract this recommendation and use it instead of generating a generic one

3. **Update WHY Message**
   - The WHY message should reflect that this is boilerplate, not a complexity issue
   - Use the boilerplate analysis confidence score in messaging

4. **Maintain Backwards Compatibility**
   - Non-boilerplate god objects should continue to get existing recommendations
   - Files without god object issues should behave as before

### Non-Functional Requirements

1. **Performance**: No significant performance impact
2. **Testability**: Should be easily testable with unit tests
3. **Clarity**: Output should clearly distinguish boilerplate from other issues

## Acceptance Criteria

- [ ] `FileDebtMetrics::generate_recommendation()` checks `GodObjectType` enum variant
- [ ] When `GodObjectType::BoilerplatePattern` is detected, boilerplate recommendation is used
- [ ] Ripgrep flags/defs.rs shows boilerplate recommendation in output
- [ ] Output shows "BOILERPLATE DETECTED" message
- [ ] Output includes confidence percentage
- [ ] Output says "This is NOT a god object requiring module splitting"
- [ ] Generic god objects still get splitting recommendations
- [ ] Unit test verifies boilerplate recommendation is used
- [ ] Integration test confirms ripgrep output shows macro recommendation
- [ ] No regressions in existing god object detection

## Technical Details

### Implementation Approach

**Problem Location**: `src/priority/file_metrics.rs:146-195`

**Current Code**:
```rust
pub fn generate_recommendation(&self) -> String {
    if self.god_object_indicators.is_god_object {
        // Only checks boolean flag, doesn't check GodObjectType variant!
        format!("URGENT: {} lines, {} functions! Split by data flow...", ...)
    } else if self.total_lines > 500 {
        ...
    }
}
```

**Fix Required**:

1. **Add GodObjectType to FileDebtMetrics**

   The `FileDebtMetrics` struct needs access to the actual `GodObjectType` enum value, not just the boolean `is_god_object` flag.

   ```rust
   pub struct FileDebtMetrics {
       // ... existing fields
       pub god_object_indicators: GodObjectIndicators,
       pub god_object_type: Option<GodObjectType>, // Add this field
   }
   ```

2. **Update generate_recommendation()**

   ```rust
   pub fn generate_recommendation(&self) -> String {
       // First check for boilerplate pattern
       if let Some(GodObjectType::BoilerplatePattern { recommendation, .. }) = &self.god_object_type {
           return recommendation.clone();
       }

       // Then check for regular god objects
       if self.god_object_indicators.is_god_object {
           // Existing god object recommendation logic
           if let Some(GodObjectType::GodClass { .. }) = &self.god_object_type {
               // Class-specific splitting advice
           } else if let Some(GodObjectType::GodFile { .. }) = &self.god_object_type {
               // Module-specific splitting advice
           } else {
               // Generic splitting advice (fallback)
           }
       } else if self.total_lines > 500 {
           ...
       }
   }
   ```

3. **Update WHY Message Generation**

   The `generate_why_message()` function (in formatter.rs) should also check for boilerplate:

   ```rust
   fn generate_why_message(...) -> String {
       if let Some(boilerplate_info) = check_for_boilerplate(...) {
           format!(
               "BOILERPLATE DETECTED: {} ({:.0}% confidence). This file contains \
               repetitive patterns that should be macro-ified, not split into modules.",
               boilerplate_info.pattern_description,
               boilerplate_info.confidence * 100.0
           )
       } else if is_god_object {
           // Existing god object WHY message
       } else {
           ...
       }
   }
   ```

4. **Propagate GodObjectType Through Analysis Pipeline**

   Ensure `GodObjectType` is passed from god object detection through to `FileDebtMetrics`:

   ```rust
   // In god_object_detector.rs
   pub struct GodObjectAnalysis {
       pub classification: GodObjectType,  // Already exists
       ...
   }

   // In file_analysis.rs aggregate_file_metrics()
   fn aggregate_file_metrics(...) -> FileDebtMetrics {
       ...
       FileDebtMetrics {
           god_object_type: Some(god_object_analysis.classification),
           ...
       }
   }
   ```

### Data Structures

No new data structures needed. Just need to pass existing `GodObjectType` through the pipeline:

```rust
// Already exists in src/organization/god_object_analysis.rs
pub enum GodObjectType {
    GodClass { ... },
    GodFile { ... },
    GodModule { ... },
    BoilerplatePattern {
        pattern: BoilerplatePattern,
        confidence: f64,
        recommendation: String,  // Already contains the macro recommendation!
    },
    RegistryPattern { ... },
}
```

### Files to Modify

1. **src/priority/file_metrics.rs**
   - Add `god_object_type: Option<GodObjectType>` field to `FileDebtMetrics`
   - Update `generate_recommendation()` to check for boilerplate
   - Update `Default` implementation

2. **src/priority/file_analysis.rs**
   - Pass `god_object_analysis.classification` to `FileDebtMetrics`

3. **src/priority/formatter.rs**
   - Update `generate_why_message()` to check for boilerplate
   - Update `determine_file_type_label()` to show "BOILERPLATE PATTERN"

4. **Tests**
   - Add unit test for boilerplate recommendation generation
   - Add integration test with ripgrep flags/defs.rs

## Dependencies

- **Prerequisites**: None (boilerplate detection already works)
- **Affected Components**:
  - `src/priority/file_metrics.rs` - Core fix
  - `src/priority/file_analysis.rs` - Pass classification through
  - `src/priority/formatter.rs` - Update output formatting
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_boilerplate_recommendation_used() {
    let boilerplate_type = GodObjectType::BoilerplatePattern {
        pattern: BoilerplatePattern::TraitImplementation {
            trait_name: "Flag".to_string(),
            impl_count: 104,
            shared_methods: vec!["name_long".to_string()],
            method_uniformity: 1.0,
        },
        confidence: 0.878,
        recommendation: "BOILERPLATE DETECTED: Create declarative macro...".to_string(),
    };

    let metrics = FileDebtMetrics {
        god_object_indicators: GodObjectIndicators {
            is_god_object: true,
            ...
        },
        god_object_type: Some(boilerplate_type),
        ...
    };

    let recommendation = metrics.generate_recommendation();

    assert!(recommendation.contains("BOILERPLATE DETECTED"));
    assert!(recommendation.contains("declarative macro"));
    assert!(recommendation.contains("NOT a god object requiring module splitting"));
}

#[test]
fn test_regular_god_object_still_gets_splitting_advice() {
    let god_file_type = GodObjectType::GodFile { ... };

    let metrics = FileDebtMetrics {
        god_object_indicators: GodObjectIndicators {
            is_god_object: true,
            ...
        },
        god_object_type: Some(god_file_type),
        ...
    };

    let recommendation = metrics.generate_recommendation();

    assert!(recommendation.contains("Split"));
    assert!(!recommendation.contains("BOILERPLATE"));
    assert!(!recommendation.contains("macro"));
}
```

### Integration Tests

```rust
#[test]
fn test_ripgrep_flags_shows_boilerplate_recommendation() {
    // Run full analysis on ripgrep
    let analysis = analyze_project("../ripgrep").unwrap();

    // Find flags/defs.rs in results
    let flags_item = analysis.file_items.iter()
        .find(|item| item.metrics.path.ends_with("flags/defs.rs"))
        .expect("Should find flags/defs.rs");

    // Should have boilerplate recommendation
    assert!(flags_item.recommendation.contains("BOILERPLATE DETECTED"));
    assert!(flags_item.recommendation.contains("macro"));
    assert!(flags_item.recommendation.contains("NOT a god object"));

    // Verify output formatting
    let mut output = String::new();
    format_file_priority_item(&mut output, 1, flags_item, FormattingConfig::default());

    assert!(output.contains("BOILERPLATE"));
    assert!(output.contains("87") || output.contains("88")); // Confidence %
}
```

## Documentation Requirements

### Code Documentation

- Document the `god_object_type` field in `FileDebtMetrics`
- Explain the precedence: boilerplate > god object > size > complexity
- Add examples of each recommendation type

### User Documentation

- Update README to explain boilerplate detection
- Show examples of boilerplate recommendations
- Clarify difference between "god object" and "boilerplate pattern"

## Implementation Notes

### Order of Checks

The precedence for recommendations should be:
1. **Boilerplate Pattern** - Highest priority (use macros, not splitting)
2. **Registry Pattern** - Special architectural pattern (preserve, don't split)
3. **God Object/Class/Module** - Splitting recommendations
4. **Large File** - Generic size reduction
5. **High Complexity** - Refactoring suggestions
6. **Low Coverage** - Testing recommendations

### Why This Matters

Currently, users see:
> "Extract complex functions, reduce file to <500 lines"

This is **wrong advice** for boilerplate! Splitting 888 flag structs into multiple files doesn't help. The right advice is:
> "Create a declarative macro to reduce 7800 lines to ~832 lines (89% reduction)"

This is a **critical** fix because:
- Wrong advice wastes developer time
- May lead to worse architecture (spreading boilerplate across files)
- Undermines trust in debtmap's recommendations

### Edge Cases

- File is both boilerplate AND has complex functions → Show boilerplate recommendation
- Boilerplate confidence < 70% → Fall back to god object recommendation
- No god object but large file → Existing size recommendation (no change)

## Migration and Compatibility

### Breaking Changes

- Output format changes for boilerplate files
- JSON/YAML structure adds `god_object_type` field

### Backward Compatibility

- Non-boilerplate files show identical recommendations
- New field is `Option<GodObjectType>` so missing = None (backward compatible)
- Existing tests should pass unchanged

### Migration Path

1. Add `god_object_type` field as `Option<>` (backward compatible)
2. Update code to populate field from god object analysis
3. Update recommendation generation to check field
4. Update tests to verify new behavior
5. Document new output format

## Success Metrics

- Ripgrep flags/defs.rs shows boilerplate recommendation (not splitting advice)
- Output contains "BOILERPLATE DETECTED"
- Output says "NOT a god object requiring module splitting"
- Confidence percentage shown (87.8% for ripgrep)
- All existing god object recommendations unchanged
- No test regressions
- User reports confirm recommendations are more helpful

## Example Output

### Before (Current - Wrong)
```
#1 SCORE: 474 [CRITICAL - FILE - HIGH COMPLEXITY]
└─ ./crates/core/flags/defs.rs (7775 lines, 888 functions)
└─ WHY: File exceeds recommended size with 7775 lines. Large files are harder to navigate...
└─ ACTION: Extract complex functions, reduce file to <500 lines. Current: 7775 lines
```

### After (Fixed - Correct)
```
#1 SCORE: 474 [CRITICAL - FILE - BOILERPLATE PATTERN]
└─ ./crates/core/flags/defs.rs (7775 lines, 888 functions)
└─ WHY: BOILERPLATE DETECTED: 104 implementations of Flag trait (100% method uniformity, 88% confidence).
   This file contains repetitive trait implementations that should be macro-ified or code-generated.
└─ ACTION: Create a declarative macro to generate Flag implementations
   - Replace 104 trait impl blocks with macro invocations
   - Expected reduction: 7800 lines → ~832 lines (89% reduction)
   - Shared methods: doc_long, is_switch, name_long, update, doc_category, and 1 more

   This is NOT a god object requiring module splitting. The high method/line count
   is due to declarative boilerplate, not complexity. Focus on reducing repetition
   through macros rather than splitting into multiple files.
```
