---
number: 140
title: Domain-Based Organization Analysis for Cross-Domain Struct Mixing
category: optimization
priority: high
status: draft
dependencies: [133]
created: 2025-10-27
---

# Specification 140: Domain-Based Organization Analysis for Cross-Domain Struct Mixing

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [133 - God Object vs God Module Detection]

## Context

Rust projects commonly suffer from **cross-domain struct mixing** - files that contain struct definitions from multiple unrelated semantic domains. This creates maintenance issues, poor discoverability, and architectural drift.

**The Core Problem: Cross-Domain Mixing**

Files that mix structs from different domains violate Single Responsibility Principle at the module level:

```rust
// config.rs - MIXING 15+ UNRELATED DOMAINS
pub struct ScoringWeights { ... }           // Scoring domain
pub struct ThresholdsConfig { ... }         // Thresholds domain
pub struct GodObjectConfig { ... }          // God object detection domain
pub struct OrchestratorDetectionConfig {...}// Orchestrator detection domain
pub struct EntropyConfig { ... }            // Complexity analysis domain
pub struct LanguagesConfig { ... }          // Language support domain
pub struct DisplayConfig { ... }            // Output formatting domain
// ... 23+ more structs across different domains
```

**Current State: God Object-Only Analysis**

Debtmap currently only analyzes struct organization when a file is flagged as a **god object**. This means:
- ❌ Files with cross-domain mixing get NO guidance until they're critically large
- ❌ Generic "core/io/utils" fallback recommendations provide zero value
- ❌ Domain analysis capability exists but isn't used proactively

**Example of Current Failure:**

A 600-line file with 10 structs across 4 unrelated domains gets no recommendations because it's not yet a "god object", even though it clearly violates SRP and would benefit from domain-based organization.

**What Already Exists:**

The codebase has `suggest_module_splits_by_domain()` at `src/organization/god_object_analysis.rs:625` which:
- Analyzes struct names to classify semantic domains
- Groups related structs together
- Generates domain-specific module recommendations
- Works independently of god object detection

**The Gap:**

This domain analysis is only triggered for god objects, and uses method-based grouping as the default. We need to **invert this**: make domain-based analysis the **primary strategy** for struct-heavy files, regardless of god object status.

## Objective

Implement **domain-based organization analysis** that proactively detects cross-domain struct mixing and provides actionable module split recommendations, regardless of whether the file is flagged as a god object. Shift from reactive (fix god objects) to proactive (prevent architectural drift through domain organization).

## Requirements

### Functional Requirements

1. **Cross-Domain Detection (Primary Analysis)**
   - **Primary goal**: Detect files mixing structs from multiple unrelated domains
   - Calculate domain diversity (count of distinct semantic domains)
   - Identify cross-domain mixing regardless of file size or god object status
   - Track struct count, domain count, and struct-to-function ratio
   - Flag files with poor domain cohesion

2. **Domain Classification**
   - Use existing `suggest_module_splits_by_domain()` for domain analysis
   - Leverage existing `classify_struct_domain()` logic
   - Support 15+ semantic domain patterns (scoring, thresholds, detection, etc.)
   - Group structs by semantic domain, not by size or complexity
   - Preserve struct ownership information in recommendations

3. **Recommendation Trigger Conditions**

   **Trigger on Cross-Domain Mixing (Primary Strategy):**
   ```
   IF file contains structs AND domain_diversity >= 3 THEN
       severity = determine_severity(struct_count, domain_count, file_lines, is_god_object)
       generate_domain_based_recommendations()
   ```

   **Severity Levels:**

   - **CRITICAL** (Red flag - immediate action needed):
     - is_god_object = true AND domain_diversity >= 3
     - OR struct_count > 15 AND domain_diversity >= 5
     - Message: "URGENT: Cross-domain mixing in god module"

   - **HIGH** (Strong recommendation):
     - struct_count >= 10 AND domain_diversity >= 4
     - OR file_lines > 800 AND domain_diversity >= 3
     - Message: "Significant cross-domain mixing detected"

   - **MEDIUM** (Proactive suggestion):
     - struct_count >= 8 AND domain_diversity >= 3
     - OR file_lines > 400 AND domain_diversity >= 3
     - Message: "Consider domain-based organization"

   - **LOW/INFO** (Informational):
     - struct_count >= 5 AND domain_diversity >= 3
     - Message: "Multiple domains detected, monitor organization"

   **No Recommendation:**
   - domain_diversity < 3 (cohesive, single or dual domain)
   - struct_count < 5 (too small to warrant splitting)
   - Single-domain files regardless of size

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

### Primary: Cross-Domain Detection
- [ ] Files with 3+ semantic domains trigger domain-based analysis regardless of god object status
- [ ] Domain count and diversity metrics calculated for all struct-heavy files
- [ ] Severity determined based on cross-domain mixing extent (not just file size)
- [ ] Cross-domain analysis runs BEFORE god object analysis (primary strategy)

### Recommendation Quality
- [ ] config.rs (30 structs, 15 domains) shows CRITICAL severity with domain-specific splits
- [ ] Mid-size file (10 structs, 4 domains, 600 lines, not god object) shows MEDIUM severity recommendations
- [ ] Single-domain files (8 structs, 1 domain) show NO cross-domain recommendations
- [ ] Each recommended split includes:
  - [ ] Severity level (CRITICAL/HIGH/MEDIUM/LOW)
  - [ ] Specific domain name (not generic "core")
  - [ ] List of structs to move (at least 3 examples)
  - [ ] Domain rationale (why structs grouped together)
  - [ ] Estimated line count within 20% accuracy

### Fallback Behavior
- [ ] Generic "core/io/utils" fallback NEVER shown when domain analysis possible
- [ ] Method-based analysis used only for:
  - [ ] Files with <3 domains (cohesive)
  - [ ] Method-heavy god objects (struct_count < 5)
- [ ] Output clearly indicates analysis method (CrossDomain vs MethodBased)

### Performance and Compatibility
- [ ] Domain diversity calculation adds <50ms per file
- [ ] No performance regression >5% on existing benchmarks
- [ ] Backward compatible JSON output (new fields optional)
- [ ] No breaking changes to existing API

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

**New Code (Cross-Domain Mixing as Primary Analysis):**
```rust
let recommended_splits = {
    let file_name = path.file_stem()...;
    let struct_count = per_struct_metrics.len();
    let total_functions = all_methods.len();

    // PRIMARY ANALYSIS: Check for cross-domain struct mixing
    if struct_count >= 5 {
        let domain_count = count_distinct_domains(&per_struct_metrics);

        // Cross-domain mixing detected (3+ domains)
        if domain_count >= 3 {
            // Generate domain-based recommendations
            let splits = crate::organization::suggest_module_splits_by_domain(&per_struct_metrics);

            // Determine severity for output formatting
            let severity = determine_cross_domain_severity(
                struct_count,
                domain_count,
                lines_of_code,
                is_god_object,
            );

            // Attach severity metadata to splits
            attach_severity_to_splits(splits, severity)
        }
        // Struct-heavy but single domain - check for method-based splits
        else if is_god_object {
            // Fall back to method-based analysis for god objects
            crate::organization::recommend_module_splits(
                file_name,
                &all_methods,
                &responsibility_groups,
            )
        } else {
            vec![]
        }
    }
    // Method-heavy file, use traditional god object analysis
    else if is_god_object {
        crate::organization::recommend_module_splits(
            file_name,
            &all_methods,
            &responsibility_groups,
        )
    } else {
        vec![]
    }
};

/// Determine severity of cross-domain mixing issue
fn determine_cross_domain_severity(
    struct_count: usize,
    domain_count: usize,
    lines: usize,
    is_god_object: bool,
) -> RecommendationSeverity {
    // CRITICAL: God object with cross-domain mixing
    if is_god_object && domain_count >= 3 {
        return RecommendationSeverity::Critical;
    }

    // CRITICAL: Massive cross-domain mixing
    if struct_count > 15 && domain_count >= 5 {
        return RecommendationSeverity::Critical;
    }

    // HIGH: Significant cross-domain issues
    if struct_count >= 10 && domain_count >= 4 {
        return RecommendationSeverity::High;
    }

    if lines > 800 && domain_count >= 3 {
        return RecommendationSeverity::High;
    }

    // MEDIUM: Proactive improvement opportunity
    if struct_count >= 8 || lines > 400 {
        return RecommendationSeverity::Medium;
    }

    // LOW: Informational only
    RecommendationSeverity::Low
}
```

### Architecture Changes

1. **Cross-Domain Analysis (New Primary Analysis)**
   - Add `count_distinct_domains()` function
   - Calculate domain diversity as primary metric
   - Determine cross-domain mixing severity independently of god object status
   - Store domain_count and domain_diversity in analysis results

2. **Severity-Based Recommendation System**
   - Add `RecommendationSeverity` enum (Critical, High, Medium, Low)
   - Implement `determine_cross_domain_severity()` function
   - Attach severity metadata to all recommendations
   - Format output based on severity level (color coding, urgency)

3. **Domain Classifier Enhancements**
   - Enhance `classify_struct_domain()` with more patterns
   - Add support for nested domains (e.g., `detection/god_objects`)
   - Improve disambiguation for common prefixes
   - Track domain classification confidence

4. **Recommendation Formatting**
   - Show severity level prominently (CRITICAL, HIGH, MEDIUM, LOW)
   - Display domain diversity metrics (X structs across Y domains)
   - Show domain-specific groupings with rationale
   - Include example structs per recommended module

### Data Structures

**Add to `GodObjectIndicators`:**
```rust
pub struct GodObjectIndicators {
    // ... existing fields ...

    /// Number of distinct semantic domains detected
    #[serde(default)]
    pub domain_count: usize,

    /// Domain diversity score (0.0 to 1.0)
    #[serde(default)]
    pub domain_diversity: f64,

    /// Ratio of struct definitions to total functions (0.0 to 1.0)
    #[serde(default)]
    pub struct_ratio: f64,

    /// Analysis method used for recommendations
    #[serde(default)]
    pub analysis_method: SplitAnalysisMethod,

    /// Severity of cross-domain mixing (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_domain_severity: Option<RecommendationSeverity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SplitAnalysisMethod {
    #[default]
    None,
    CrossDomain,      // domain mixing analysis (primary)
    MethodBased,      // responsibility_groups analysis
    Hybrid,           // combination of both
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecommendationSeverity {
    Critical,  // Immediate action required
    High,      // Strong recommendation
    Medium,    // Proactive improvement
    Low,       // Informational
}
```

**Enhance `ModuleSplit`:**
```rust
pub struct ModuleSplit {
    // ... existing fields ...

    /// Semantic domain this split represents
    pub domain: String,

    /// Explanation of why this split was suggested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,

    /// Analysis method that generated this split
    #[serde(default)]
    pub method: SplitAnalysisMethod,

    /// Severity of this recommendation
    #[serde(default)]
    pub severity: RecommendationSeverity,
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
