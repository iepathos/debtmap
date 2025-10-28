---
number: 140
title: Domain-Based Struct Split Recommendations for God Modules
category: optimization
priority: high
status: draft
dependencies: [133]
created: 2025-10-27
---

# Specification 140: Domain-Based Struct Split Recommendations for God Modules

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [133 - God Object vs God Module Detection]

## Context

Debtmap currently provides **generic, low-value recommendations** for god modules that are primarily composed of struct definitions rather than methods. This is evident in the analysis of `src/config.rs`:

**Current Output:**
```
#2 SCORE: 100 [CRITICAL - FILE - GOD OBJECT]
└─ ./src/config.rs (2732 lines, 217 functions)
└─ ACTION: Split by data flow: 1) Input/parsing 2) Core logic 3) Output
  - SUGGESTED SPLIT (generic - no detailed analysis available):
  -  [1] config_core.rs - Core business logic
  -  [2] config_io.rs - Input/output operations
  -  [3] config_utils.rs - Helper functions
```

**The Problem:**
1. config.rs contains **30 struct definitions** (configuration schemas) across 15+ feature domains
2. It has **90 `default_*()` functions** (serde defaults) and ~20 actual functions
3. The "data flow" split recommendation is **completely wrong** for this file type
4. The generic "core/io/utils" suggestion provides **zero actionable value**

**What's Available But Not Used:**
The codebase already has `suggest_module_splits_by_domain()` at `src/organization/god_object_analysis.rs:625` which:
- Analyzes struct names to classify them into domains (scoring, thresholds, detection, etc.)
- Groups related structs together
- Generates domain-specific module recommendations
- Would produce **excellent recommendations** like:
  - `config/scoring.rs` - ScoringWeights, RoleMultipliers
  - `config/thresholds.rs` - ThresholdsConfig, ValidationThresholds
  - `config/detection.rs` - All detection configs

**Root Cause:**
The `recommend_module_splits()` function (line 592) only works with **method-based grouping** via `responsibility_groups`. For struct-heavy files with few methods, it returns empty recommendations, triggering the generic fallback.

## Objective

Implement intelligent split recommendations that detect **struct-heavy god modules** and use domain-based struct grouping instead of generic fallbacks, providing actionable, high-value recommendations for refactoring.

## Requirements

### Functional Requirements

1. **Struct-Heavy File Detection**
   - Detect when a file is primarily struct definitions (>30% struct ratio)
   - Calculate struct count, total functions, and struct-to-function ratio
   - Distinguish between method-heavy vs struct-heavy files
   - Track domain diversity (number of distinct semantic domains)
   - Support both god object analysis AND proactive organization analysis

2. **Domain Classification Integration**
   - Use existing `suggest_module_splits_by_domain()` for struct-heavy files
   - Leverage existing `classify_struct_domain()` logic
   - Support enhanced domain classifier with 15+ domain patterns
   - Preserve struct ownership information in recommendations

3. **Recommendation Trigger Conditions**

   **Primary Trigger (CRITICAL - God Object):**
   - File flagged as god object (`is_god_object = true`)
   - AND struct-heavy characteristics (struct_count > 5, struct_ratio > 0.3)
   - Shows critical-level recommendations with refactoring guidance

   **Secondary Trigger (WARNING - Organization):**
   - NOT flagged as god object yet (`is_god_object = false`)
   - BUT has organization issues: struct_count > 8, domain_diversity >= 3, file_lines > 400
   - Shows warning-level suggestions for proactive improvement

   **No Recommendation:**
   - Few structs (< 5 for god objects, < 8 for organization)
   - Low domain diversity (< 3 distinct domains)
   - Small files (< 400 lines)
   - Cohesive single-domain files

4. **Recommendation Quality Threshold**
   - Never show generic fallback if domain-based analysis is available
   - Require minimum 2 domain groups for split recommendations
   - Filter out single-struct domains unless >200 lines
   - Prioritize splits with >5 structs or >400 estimated lines

5. **Recommendation Content**
   - Show specific struct names to move (first 3-5 per module)
   - Include estimated line counts per module
   - Suggest appropriate module paths (e.g., `config/detection/`)
   - Provide rationale for grouping (domain explanation)
   - Include severity level (CRITICAL for god objects, WARNING for organization issues)

6. **Hybrid File Handling**
   - For files with both structs and methods:
     - If struct-heavy (>60% structs): Use domain-based grouping
     - If method-heavy (>60% methods): Use responsibility-based grouping
     - If balanced (40-60%): Combine both approaches with clear sections

### Non-Functional Requirements

1. **Performance**: Domain analysis should add <100ms per file
2. **Maintainability**: Clear separation between struct-based and method-based logic
3. **Extensibility**: Easy to add new domain patterns
4. **Accuracy**: Domain classification should be >90% accurate for common patterns

## Acceptance Criteria

- [ ] Struct-heavy files (>10 structs, struct/method ratio >3:1) trigger domain-based recommendations
- [ ] config.rs shows domain-specific splits (scoring, thresholds, detection, etc.) instead of generic core/io/utils
- [ ] Each recommended split includes:
  - [ ] Specific domain name (not generic "core")
  - [ ] List of structs to move (at least 3 examples shown)
  - [ ] Estimated line count within 20% accuracy
  - [ ] Suggested module path (e.g., `config/scoring.rs`)
- [ ] Generic fallback only shown when:
  - [ ] File has <5 structs
  - [ ] Domain analysis finds <2 distinct groups
  - [ ] No clear domain patterns detected
- [ ] Method-heavy files continue to use responsibility-based grouping
- [ ] Output clearly indicates which analysis method was used
- [ ] No performance regression >5% on existing benchmarks

## Technical Details

### Implementation Approach

**File**: `src/organization/god_object_detector.rs`

**Current Code (line 671):**
```rust
let recommended_splits = if is_god_object {
    let file_name = path.file_stem()...;
    crate::organization::recommend_module_splits(
        file_name,
        &all_methods,
        &responsibility_groups,
    )
} else {
    vec![]
};
```

**New Code:**
```rust
let recommended_splits = {
    let file_name = path.file_stem()...;
    let struct_count = per_struct_metrics.len();
    let total_functions = all_methods.len();
    let struct_ratio = if total_functions > 0 {
        struct_count as f64 / total_functions as f64
    } else {
        0.0
    };

    // Determine if this is a struct-heavy file
    let is_struct_heavy = struct_count > 5 && struct_ratio > 0.3;

    // Primary trigger: God object detection
    if is_god_object {
        if is_struct_heavy {
            // Use domain-based struct grouping for struct-heavy god objects
            crate::organization::suggest_module_splits_by_domain(&per_struct_metrics)
        } else {
            // Use method-based responsibility grouping for method-heavy god objects
            crate::organization::recommend_module_splits(
                file_name,
                &all_methods,
                &responsibility_groups,
            )
        }
    }
    // Secondary trigger: Proactive organization analysis
    else if struct_count > 8 && lines_of_code > 400 {
        // Calculate domain diversity
        let domain_count = count_distinct_domains(&per_struct_metrics);

        if domain_count >= 3 {
            // File has organization issues - suggest domain-based splits
            // These will be marked as WARNING level in output
            crate::organization::suggest_module_splits_by_domain(&per_struct_metrics)
        } else {
            vec![]
        }
    } else {
        vec![]
    }
};
```

### Architecture Changes

1. **Detection Logic Enhancement**
   - Add `is_struct_heavy()` helper function
   - Calculate struct-to-function ratio
   - Store ratio in `GodObjectIndicators` for visibility

2. **Domain Classifier Improvements**
   - Enhance `classify_struct_domain()` with more patterns
   - Add support for nested config domains (e.g., `detection/god_objects`)
   - Improve heuristics for ambiguous names

3. **Recommendation Formatting**
   - Update `format_god_object_steps()` to show analysis method used
   - Display struct grouping rationale
   - Show example structs per module

### Data Structures

**Add to `GodObjectIndicators`:**
```rust
pub struct GodObjectIndicators {
    // ... existing fields ...

    /// Ratio of struct definitions to total functions (0.0 to 1.0)
    #[serde(default)]
    pub struct_ratio: f64,

    /// Analysis method used for recommendations
    #[serde(default)]
    pub analysis_method: SplitAnalysisMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SplitAnalysisMethod {
    #[default]
    None,
    MethodBased,      // responsibility_groups analysis
    StructBased,      // domain classification
    Hybrid,           // combination of both
}
```

**Enhance `ModuleSplit`:**
```rust
pub struct ModuleSplit {
    // ... existing fields ...

    /// Explanation of why this split was suggested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,

    /// Analysis method that generated this split
    #[serde(default)]
    pub method: SplitAnalysisMethod,
}
```

### APIs and Interfaces

**New Helper Functions:**

```rust
/// Determine if a file is primarily struct definitions
fn is_struct_heavy_file(
    struct_count: usize,
    total_functions: usize,
    module_functions: usize,
) -> bool {
    struct_count > 5 &&
    (struct_count as f64 / total_functions as f64) > 0.3 &&
    module_functions < struct_count * 4
}

/// Calculate struct-to-function ratio
fn calculate_struct_ratio(
    struct_count: usize,
    total_functions: usize,
) -> f64 {
    if total_functions == 0 {
        return 0.0;
    }
    (struct_count as f64) / (total_functions as f64)
}

/// Count distinct semantic domains in struct list
fn count_distinct_domains(structs: &[StructMetrics]) -> usize {
    let domains: HashSet<String> = structs
        .iter()
        .map(|s| classify_struct_domain(&s.name))
        .collect();
    domains.len()
}

/// Check if file has organization issues (even if not god object)
fn has_organization_issues(
    struct_count: usize,
    domain_count: usize,
    lines_of_code: usize,
) -> bool {
    struct_count > 8 &&
    domain_count >= 3 &&
    lines_of_code > 400
}
```

**Enhanced Domain Classifier:**

File: `src/organization/domain_classifier.rs`

```rust
/// Enhanced struct domain classification with nested path support
pub fn classify_struct_domain_with_path(
    struct_name: &str,
    methods: &[String],
) -> String {
    let domain = classify_struct_domain_enhanced(struct_name, methods);

    // For detection patterns, suggest nested structure
    if domain == "detection" {
        if struct_name.contains("GodObject") {
            return "detection/god_objects".to_string();
        } else if struct_name.contains("Orchestrator") {
            return "detection/orchestrator".to_string();
        } else if struct_name.contains("Constructor") {
            return "detection/constructors".to_string();
        }
    }

    domain
}
```

## Dependencies

### Prerequisites
- **Spec 133**: God Object vs God Module detection must be implemented
- Existing `suggest_module_splits_by_domain()` function must be available
- `classify_struct_domain()` must be functional

### Affected Components
- `src/organization/god_object_detector.rs` - Main detection logic
- `src/organization/god_object_analysis.rs` - Split recommendation logic
- `src/organization/domain_classifier.rs` - Domain classification (optional enhancement)
- `src/priority/formatter.rs` - Recommendation output formatting
- `src/priority/file_metrics.rs` - Data structures

### External Dependencies
None - uses existing functionality

## Testing Strategy

### Unit Tests

**File**: `src/organization/god_object_detector.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_struct_heavy_detection() {
        // config.rs pattern: 30 structs, 217 functions
        assert!(is_struct_heavy_file(30, 217, 150));

        // Method-heavy file: 5 structs, 100 methods
        assert!(!is_struct_heavy_file(5, 100, 95));

        // Balanced file: 10 structs, 30 functions
        assert!(is_struct_heavy_file(10, 30, 20));
    }

    #[test]
    fn test_struct_ratio_calculation() {
        assert_eq!(calculate_struct_ratio(30, 100), 0.3);
        assert_eq!(calculate_struct_ratio(10, 20), 0.5);
        assert_eq!(calculate_struct_ratio(0, 100), 0.0);
        assert_eq!(calculate_struct_ratio(10, 0), 0.0);
    }

    #[test]
    fn test_domain_recommendations_for_config_file() {
        // Simulate config.rs analysis
        let structs = vec![
            create_struct_metrics("ScoringWeights", 50),
            create_struct_metrics("RoleMultipliers", 80),
            create_struct_metrics("ThresholdsConfig", 100),
            create_struct_metrics("GodObjectConfig", 150),
            create_struct_metrics("LanguagesConfig", 120),
        ];

        let splits = suggest_module_splits_by_domain(&structs);

        // Should have multiple domain groups
        assert!(splits.len() >= 3);

        // Should have scoring domain
        assert!(splits.iter().any(|s| s.suggested_name.contains("scoring")));

        // Should have thresholds domain
        assert!(splits.iter().any(|s| s.suggested_name.contains("threshold")));
    }
}
```

### Integration Tests

**File**: `tests/god_object_struct_recommendations.rs`

```rust
#[test]
fn test_config_file_recommendations() {
    let config = load_test_fixture("fixtures/config.rs");
    let analysis = analyze_rust_file(&config);

    // Should detect as god module
    assert!(analysis.god_object_indicators.is_god_object);

    // Should have struct-based recommendations
    assert_eq!(
        analysis.god_object_indicators.analysis_method,
        SplitAnalysisMethod::StructBased
    );

    // Should have domain-specific splits
    let splits = &analysis.god_object_indicators.recommended_splits;
    assert!(splits.len() >= 5);

    // Should NOT have generic recommendations
    assert!(!splits.iter().any(|s| s.suggested_name.contains("_core")));
    assert!(!splits.iter().any(|s| s.suggested_name.contains("_io")));
    assert!(!splits.iter().any(|s| s.suggested_name.contains("_utils")));

    // Should have specific domains
    let domains: Vec<_> = splits.iter()
        .map(|s| &s.suggested_name)
        .collect();
    assert!(domains.iter().any(|d| d.contains("scoring")));
    assert!(domains.iter().any(|d| d.contains("threshold")));
}

#[test]
fn test_method_heavy_file_uses_responsibility_grouping() {
    let formatter = load_test_fixture("fixtures/formatter.rs");
    let analysis = analyze_rust_file(&formatter);

    // Should use method-based analysis
    assert_eq!(
        analysis.god_object_indicators.analysis_method,
        SplitAnalysisMethod::MethodBased
    );
}
```

### Performance Tests

```rust
#[bench]
fn bench_domain_classification(b: &mut Bencher) {
    let structs = create_large_struct_list(100);
    b.iter(|| {
        suggest_module_splits_by_domain(&structs)
    });
}

#[test]
fn test_no_performance_regression() {
    let config_file = load_large_config_file();

    let start = Instant::now();
    let _ = analyze_enhanced(&config_file);
    let duration = start.elapsed();

    // Should complete in <200ms even for large files
    assert!(duration.as_millis() < 200);
}
```

### User Acceptance

**Validation Criteria:**
1. Run debtmap on its own codebase
2. Verify config.rs shows domain-specific recommendations
3. Verify formatter.rs shows method-based recommendations
4. Verify output is actionable and specific
5. Verify no "generic - no detailed analysis" messages for struct-heavy files

## Documentation Requirements

### Code Documentation

1. **Function Documentation**
   - Document `is_struct_heavy_file()` with rationale for thresholds
   - Document domain classification algorithm
   - Add examples showing struct vs method-based analysis

2. **Algorithm Documentation**
   - Explain struct ratio calculation
   - Document threshold values and their rationale
   - Provide decision tree for analysis method selection

### User Documentation

**Update**: README.md or user guide

Add section explaining recommendation types:
```markdown
### God Module Recommendations

Debtmap provides different recommendation strategies based on file characteristics:

**Struct-Heavy Modules** (e.g., configuration files):
- Grouped by semantic domain (scoring, thresholds, detection)
- Recommends splitting by feature area
- Example: config.rs → config/scoring.rs, config/thresholds.rs

**Method-Heavy Modules** (e.g., analysis classes):
- Grouped by responsibility patterns
- Recommends splitting by method purpose
- Example: analyzer.rs → parser.rs, validator.rs, formatter.rs
```

### Architecture Updates

**File**: ARCHITECTURE.md

Add section:
```markdown
## God Object Detection - Recommendation Strategy

The god object detector uses two complementary strategies:

1. **Domain-Based (Struct Analysis)**: For files with many struct definitions
   - Analyzes struct names to classify into semantic domains
   - Groups related configuration or data structures
   - Suggests module paths by domain (e.g., config/detection/)

2. **Responsibility-Based (Method Analysis)**: For files with many methods
   - Groups methods by responsibility patterns (validation, I/O, computation)
   - Suggests functional separation
   - Traditional god object refactoring

Selection criteria: struct_count > 5 && struct_ratio > 0.3
```

## Implementation Notes

### Domain Pattern Best Practices

When adding new domain patterns, consider:
1. **Specificity**: More specific patterns first (e.g., "god_object" before "detection")
2. **Nesting**: Support hierarchical domains (detection/god_objects/)
3. **Fallback**: Always have a fallback for uncategorized structs
4. **Validation**: Test with real-world struct names

### Threshold Tuning

Current thresholds for struct-heavy detection:
- `struct_count > 5`: Minimum structs to consider domain grouping
- `struct_ratio > 0.3`: 30% of file is struct definitions
- `module_functions < struct_count * 4`: Module functions don't dominate

These may need tuning based on real-world usage.

### Edge Cases

1. **Macro-generated structs**: May not have clear domain patterns
   - Solution: Use method names as secondary signal

2. **Mixed purpose files**: Both data structures and algorithms
   - Solution: Use hybrid analysis, show both groupings

3. **Single large struct**: Many fields but only one struct
   - Solution: Fall back to method-based or generic recommendations

## Migration and Compatibility

### Breaking Changes
None - this is an enhancement to recommendation quality.

### Backward Compatibility
- Existing `recommended_splits` field unchanged
- New fields are optional and default-initialized
- JSON output remains compatible
- CLI output enhanced but not breaking

### Migration Steps
1. Deploy new analysis logic
2. Verify improved recommendations on test corpus
3. Update documentation
4. No user migration needed (analysis-only change)

## Success Metrics

### Quantitative Metrics
- [ ] Struct-heavy files (>10 structs) show domain-based recommendations >90% of time
- [ ] Generic fallback usage reduced by >70%
- [ ] Recommendation specificity score (struct names shown / total structs) >60%
- [ ] Performance overhead <5% on god object detection

### Qualitative Metrics
- [ ] Recommendations are actionable (developers can implement without guessing)
- [ ] Domain groupings make semantic sense
- [ ] Module paths reflect actual refactoring strategy
- [ ] Users report improved recommendation quality

## Future Enhancements

### Phase 2: Interactive Refinement
- Allow users to provide domain hints in config
- Support custom domain classification rules
- Interactive recommendation adjustment

### Phase 3: Cross-File Analysis
- Detect when multiple files share domain patterns
- Suggest unified module structure across files
- Recommend common base modules

### Phase 4: ML-Based Classification
- Learn domain patterns from successfully refactored code
- Improve classification accuracy over time
- Personalize recommendations per project

## References

- **Spec 133**: God Object vs God Module Detection
- Existing implementation: `src/organization/god_object_analysis.rs:625`
- Domain classifier: `src/organization/domain_classifier.rs:19`
- Current gap: `src/organization/god_object_detector.rs:671`
