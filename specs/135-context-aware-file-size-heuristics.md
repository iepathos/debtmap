---
number: 135
title: Context-Aware File Size Heuristics
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-27
---

# Specification 135: Context-Aware File Size Heuristics

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Current file size recommendations are too aggressive and don't account for file type context. For example:
- Recommending reducing a 7775-line flags definition file to <500 lines (93.6% reduction)
- This file has 888 functions, making the target 0.56 lines per function (impossible)
- Generated code, declarative configurations, and test files have different appropriate sizes
- One-size-fits-all thresholds produce impractical recommendations

This undermines user trust when debtmap suggests impossible or architecturally inappropriate changes.

## Objective

Implement context-aware file size heuristics that adjust thresholds based on file type, purpose, and characteristics, producing practical and architecturally sound recommendations.

## Requirements

### Functional Requirements

1. **File Type Classification**
   - Detect generated code (markers like "DO NOT EDIT", code generation tools)
   - Identify declarative/configuration files (flag definitions, schema definitions)
   - Recognize test files (test modules, integration tests, property tests)
   - Distinguish business logic from infrastructure code
   - Identify procedural macros and build scripts

2. **Context-Based Thresholds**
   - Different size limits for different file types:
     - Business logic: 300-500 lines (strict)
     - Generated code: 2000+ lines (lenient, may suppress)
     - Test files: 500-800 lines (moderate)
     - Declarative config: 1000-1500 lines (lenient)
     - Procedural macros: 400-600 lines (moderate-strict)
   - Adjust thresholds based on function density (lines per function)
   - Consider architectural patterns (builder patterns may be verbose)

3. **Smart Reduction Targets**
   - Calculate achievable reduction targets based on:
     - Current function density
     - File type and purpose
     - Complexity distribution
     - Architectural constraints
   - Never suggest reduction targets that would require <3 lines per function
   - Provide phased reduction targets for large files (50% → 30% → final)

4. **Recommendation Quality**
   - Suppress or downgrade recommendations for generated code
   - Focus on high-value splitting opportunities (high complexity + large size)
   - Provide file-type-specific splitting strategies
   - Distinguish "must split" from "could split" recommendations

### Non-Functional Requirements

1. **Accuracy**: File type detection should be >95% accurate on common patterns
2. **Clarity**: Users should understand why different files have different thresholds
3. **Flexibility**: Thresholds should be configurable via CLI or config file
4. **Performance**: Classification should not significantly slow analysis

## Acceptance Criteria

- [ ] Generated code is correctly identified and gets lenient thresholds or suppressed warnings
- [ ] Declarative/config files (like flags/defs.rs) get appropriate thresholds (1000-1500 lines)
- [ ] Business logic files maintain strict thresholds (300-500 lines)
- [ ] Test files get moderate thresholds (500-800 lines)
- [ ] No recommendation suggests reducing to <3 lines per function
- [ ] Function density is factored into threshold calculation
- [ ] Phased reduction targets are provided for files >2x threshold
- [ ] File type classification is shown in output with rationale
- [ ] Ripgrep flags/defs.rs gets appropriate threshold (not 500 lines)
- [ ] Reduction targets are achievable and architecturally sound
- [ ] Users can override thresholds via configuration
- [ ] Documentation explains threshold rationale for each file type

## Technical Details

### Implementation Approach

1. **File Type Detection Pipeline**
   ```rust
   pub enum FileType {
       BusinessLogic,
       GeneratedCode { tool: Option<String> },
       TestCode { test_type: TestType },
       DeclarativeConfig { config_type: ConfigType },
       ProceduralMacro,
       BuildScript,
       Unknown,
   }

   pub fn classify_file(source: &str, path: &Path) -> FileType {
       // Multi-stage classification
       if is_generated_code(source) {
           FileType::GeneratedCode { tool: detect_generator(source) }
       } else if is_test_file(path, source) {
           FileType::TestCode { test_type: detect_test_type(source) }
       } else if is_declarative_config(source) {
           FileType::DeclarativeConfig { config_type: detect_config_type(source) }
       } else if is_proc_macro(path) {
           FileType::ProceduralMacro
       } else if is_build_script(path) {
           FileType::BuildScript
       } else {
           FileType::BusinessLogic
       }
   }
   ```

2. **Context-Aware Thresholds**
   ```rust
   pub struct FileSizeThresholds {
       base_threshold: usize,
       max_threshold: usize,
       min_lines_per_function: f32,
   }

   pub fn get_threshold(file_type: &FileType, metrics: &FileMetrics) -> FileSizeThresholds {
       let base = match file_type {
           FileType::BusinessLogic => 400,
           FileType::GeneratedCode { .. } => 5000, // Lenient or suppressed
           FileType::TestCode { .. } => 650,
           FileType::DeclarativeConfig { .. } => 1200,
           FileType::ProceduralMacro => 500,
           FileType::BuildScript => 300,
           FileType::Unknown => 400,
       };

       // Adjust based on function density
       let density = metrics.lines as f32 / metrics.functions.max(1) as f32;
       let adjusted = adjust_for_density(base, density);

       FileSizeThresholds {
           base_threshold: adjusted,
           max_threshold: adjusted * 2,
           min_lines_per_function: 3.0,
       }
   }
   ```

3. **Smart Reduction Calculation**
   ```rust
   pub fn calculate_reduction_target(
       current_lines: usize,
       threshold: &FileSizeThresholds,
       function_count: usize
   ) -> ReductionTarget {
       // Minimum achievable size based on function count
       let min_achievable = (function_count as f32 * threshold.min_lines_per_function) as usize;

       // Don't suggest reducing below achievable minimum
       let target = threshold.base_threshold.max(min_achievable);

       if current_lines > threshold.base_threshold * 3 {
           // Phased reduction for very large files
           ReductionTarget::Phased {
               phase1: current_lines / 2,
               phase2: threshold.base_threshold * 1.5,
               final_target: target,
           }
       } else {
           ReductionTarget::Single(target)
       }
   }
   ```

### File Type Detection Strategies

1. **Generated Code Detection**
   ```rust
   fn is_generated_code(source: &str) -> bool {
       let markers = [
           "DO NOT EDIT",
           "automatically generated",
           "AUTO-GENERATED",
           "@generated",
           "Code generated by",
       ];
       source.lines().take(10).any(|line|
           markers.iter().any(|m| line.contains(m))
       )
   }

   fn detect_generator(source: &str) -> Option<String> {
       // Detect common generators: protobuf, thrift, diesel, sea-orm, etc.
       if source.contains("prost::Message") { Some("prost".to_string()) }
       else if source.contains("diesel::") { Some("diesel".to_string()) }
       else if source.contains("tonic::") { Some("tonic".to_string()) }
       else { None }
   }
   ```

2. **Declarative Config Detection**
   ```rust
   fn is_declarative_config(source: &str) -> bool {
       // High density of similar patterns
       let pattern_indicators = [
           // Flag/option definitions
           r"(?m)^\s*pub\s+\w+:\s+\w+,\s*$",
           // Schema definitions
           r"(?m)^\s*#\[derive\(",
           // Builder patterns
           r"(?m)^\s*pub\s+fn\s+\w+\(mut\s+self",
       ];

       let matches: usize = pattern_indicators.iter()
           .map(|pat| Regex::new(pat).unwrap().find_iter(source).count())
           .sum();

       // If >70% of lines match declarative patterns
       let total_lines = source.lines().count();
       matches as f32 / total_lines as f32 > 0.7
   }
   ```

3. **Function Density Analysis**
   ```rust
   fn adjust_for_density(base_threshold: usize, density: f32) -> usize {
       // Very low density (many small functions) → stricter threshold
       // High density (few large functions) → may need lenient threshold
       match density {
           d if d < 5.0 => base_threshold,           // Many small functions: strict
           d if d < 10.0 => (base_threshold as f32 * 1.2) as usize,
           d if d < 20.0 => (base_threshold as f32 * 1.5) as usize,
           _ => (base_threshold as f32 * 2.0) as usize,  // Few large functions: lenient
       }
   }
   ```

### Data Structures

```rust
#[derive(Debug, Clone)]
pub enum TestType {
    Unit,
    Integration,
    Property,
    Benchmark,
}

#[derive(Debug, Clone)]
pub enum ConfigType {
    Flags,
    Schema,
    Routes,
    Builder,
}

#[derive(Debug, Clone)]
pub enum ReductionTarget {
    Single(usize),
    Phased {
        phase1: usize,
        phase2: usize,
        final_target: usize,
    },
    NotRecommended { reason: String },
}

#[derive(Debug)]
pub struct FileSizeAnalysis {
    file_type: FileType,
    current_lines: usize,
    threshold: FileSizeThresholds,
    reduction_target: ReductionTarget,
    function_density: f32,
    recommendation_level: RecommendationLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum RecommendationLevel {
    Critical,  // >2x threshold, business logic
    High,      // >1.5x threshold, business logic
    Medium,    // >threshold but <1.5x
    Low,       // Slightly over threshold
    Suppressed, // Generated/declarative code
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/debt/file_metrics.rs` - File-level analysis
  - `src/io/output.rs` - Recommendation formatting
  - `src/analysis/file_classifier.rs` - New module for classification
- **External Dependencies**:
  - `regex` (already in use)
  - May need `lazy_static` for regex compilation

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_generated_code_detection() {
    let generated = r#"
        // DO NOT EDIT
        // This file is automatically generated
        pub struct Generated {}
    "#;
    assert!(is_generated_code(generated));
}

#[test]
fn test_declarative_config_detection() {
    let flags = r#"
        pub struct Flags {
            pub verbose: bool,
            pub quiet: bool,
            pub output: PathBuf,
            // ... 800 more lines of similar fields
        }
    "#;
    assert!(is_declarative_config(flags));
}

#[test]
fn test_reduction_target_respects_function_count() {
    let threshold = FileSizeThresholds {
        base_threshold: 500,
        max_threshold: 1000,
        min_lines_per_function: 3.0,
    };

    let target = calculate_reduction_target(2000, &threshold, 600);
    // Should not suggest <1800 lines (600 functions * 3 lines)
    match target {
        ReductionTarget::Single(t) => assert!(t >= 1800),
        _ => panic!("Expected single target"),
    }
}

#[test]
fn test_function_density_adjustment() {
    let low_density = adjust_for_density(400, 4.0); // Many small functions
    let high_density = adjust_for_density(400, 25.0); // Few large functions

    assert_eq!(low_density, 400); // Strict threshold
    assert!(high_density > 600); // More lenient
}
```

### Integration Tests

```rust
#[test]
fn test_ripgrep_flags_defs_appropriate_threshold() {
    let analysis = analyze_file("../ripgrep/crates/core/flags/defs.rs").unwrap();

    // Should be classified as declarative config
    assert!(matches!(analysis.file_type, FileType::DeclarativeConfig { .. }));

    // Should get lenient threshold (not 500 lines)
    assert!(analysis.threshold.base_threshold >= 1000);

    // Reduction target should be achievable
    if let ReductionTarget::Single(target) = analysis.reduction_target {
        let min_achievable = (888 * 3) as usize; // 888 functions * 3 lines
        assert!(target >= min_achievable);
    }
}

#[test]
fn test_business_logic_strict_threshold() {
    let analysis = analyze_file("src/debt/god_object.rs").unwrap();

    // Business logic should get strict threshold
    assert!(matches!(analysis.file_type, FileType::BusinessLogic));
    assert!(analysis.threshold.base_threshold <= 500);
}
```

### Property-Based Tests

```rust
proptest! {
    #[test]
    fn reduction_target_never_below_minimum(
        lines in 100..10000usize,
        functions in 10..1000usize
    ) {
        let threshold = FileSizeThresholds::default();
        let target = calculate_reduction_target(lines, &threshold, functions);

        let min_achievable = functions * 3;
        match target {
            ReductionTarget::Single(t) => prop_assert!(t >= min_achievable),
            ReductionTarget::Phased { final_target, .. } => {
                prop_assert!(final_target >= min_achievable)
            }
            _ => {}
        }
    }
}
```

## Documentation Requirements

### Code Documentation

- Document file type classification algorithm and heuristics
- Explain threshold calculation and density adjustments
- Provide examples of each file type with rationale
- Document configuration options for threshold overrides

### User Documentation

- Explain why different files have different thresholds
- Provide guidance on when to split vs when to accept large files
- Document how to configure custom thresholds
- Show examples of context-appropriate recommendations

### Architecture Updates

Update ARCHITECTURE.md:
- Add section on file classification pipeline
- Document threshold calculation strategy
- Explain the balance between strictness and practicality

## Implementation Notes

### Classification Accuracy

Start with high-confidence heuristics:
1. **Generated code**: Very clear markers, low false positive rate
2. **Test files**: File path + `#[test]` attributes
3. **Declarative config**: Pattern density analysis

Iterate on less obvious cases:
- Build scripts (build.rs)
- Procedural macros (proc_macro crate)
- Mixed files (both business logic and config)

### Configuration Interface

```toml
# .debtmap.toml
[thresholds]
business_logic = 400
test_code = 650
declarative_config = 1200
generated_code = 5000

[thresholds.overrides]
# File-specific overrides
"src/flags/defs.rs" = 2000
"tests/integration/*.rs" = 1000
```

### Edge Cases

- **Mixed-purpose files**: Use weighted classification
- **Incrementally generated code**: User might edit generated files
- **Large test matrices**: Property tests can be legitimately large
- **Macro-heavy code**: May appear declarative but isn't

### Phased Rollout

1. **Phase 1**: Implement classification, log detected types (no behavior change)
2. **Phase 2**: Apply different thresholds, make recommendations
3. **Phase 3**: Add configuration support
4. **Phase 4**: Refine based on user feedback

## Migration and Compatibility

### Breaking Changes

- File size recommendations will change (some will get more lenient thresholds)
- Debt scores may change due to different thresholds

### Backward Compatibility

- Add `--legacy-thresholds` flag for old behavior
- Configuration file can restore previous thresholds if needed
- JSON output includes `file_type` and `threshold_rationale` fields

### Migration Path

1. Announce threshold changes in release notes
2. Provide migration guide showing new vs old thresholds
3. Offer configuration to restore legacy behavior
4. Deprecate legacy mode after 2-3 releases

## Success Metrics

- Zero "impossible" reduction targets (<3 lines per function)
- Generated code recommendations suppressed or downgraded
- User satisfaction with recommendation practicality
- Reduction in GitHub issues about unrealistic recommendations
- >95% file type classification accuracy on test corpus
