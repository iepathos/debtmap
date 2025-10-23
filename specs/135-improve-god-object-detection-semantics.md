---
number: 135
title: Improve God Object Detection Semantics for Multi-Struct Modules
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-22
---

# Specification 135: Improve God Object Detection Semantics for Multi-Struct Modules

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current god object detection in debtmap identifies files that exceed thresholds for methods, fields, lines of code, and responsibilities. While this works well for detecting monolithic classes, it produces misleading results and recommendations for well-organized Rust modules that contain multiple small, focused structs.

### Current Behavior

When analyzing `src/config.rs` (2,459 lines, 201 functions across 20+ separate structs), debtmap reports:

```
#1 SCORE: 56.8 [CRITICAL - FILE - GOD OBJECT]
├─ ./src/config.rs (2459 lines, 201 functions)
├─ WHY: This class violates single responsibility principle with 174 methods,
│       20 fields, and 4 distinct responsibilities.
└─ ACTION: Split by data flow: 1) Input/parsing 2) Core logic 3) Output
```

**Issues with current detection:**
1. **Misleading terminology**: Says "This class violates..." but it's not a class - it's a module with 20+ small, focused structs
2. **Generic recommendation**: Suggests splitting by data flow, but the file is already well-organized by configuration domain
3. **No struct-level analysis**: Doesn't distinguish between:
   - A single 200-method god class (actual problem)
   - 20 structs with 10 methods each (acceptable module organization)

### Real-World Impact

**Example: config.rs structure**
```rust
// Each struct is small and focused:
pub struct ScoringWeights { /* 6 fields, 5 methods */ }
pub struct RoleMultipliers { /* 8 fields, 3 methods */ }
pub struct OrchestratorDetectionConfig { /* 5 fields, 4 methods */ }
// ... 17+ more small, focused structs
```

Each individual struct follows SRP, but the aggregate is flagged as a god object.

## Objective

Enhance god object detection to distinguish between:
1. **Actual god objects**: Single structs/classes with excessive methods and responsibilities
2. **Large but organized modules**: Collections of small, focused types that happen to share a file

Provide contextually appropriate recommendations based on the actual code structure.

## Requirements

### Functional Requirements

1. **Per-Struct Analysis**
   - Detect and analyze each struct/impl block independently
   - Calculate method count, field count, and responsibilities per struct
   - Identify the largest struct in each file
   - Track standalone module functions separately

2. **Multi-Struct Module Detection**
   - Identify files containing multiple separate structs
   - Calculate aggregate metrics (total methods, total lines)
   - Determine if file is a "god module" vs "god class"
   - Differentiate between struct methods and standalone functions

3. **Context-Aware Recommendations**
   - For single large structs: Recommend class-level refactoring
   - For multi-struct modules: Recommend module splitting by domain
   - Provide specific suggestions based on detected organization patterns
   - Include struct-level metrics in recommendations

4. **Improved Reporting**
   - Report both file-level and struct-level metrics
   - Use accurate terminology ("module" vs "class")
   - Show breakdown of methods across structs
   - Highlight actual problem areas (individual god classes)

### Non-Functional Requirements

1. **Performance**: Struct-level analysis should add <5% to analysis time
2. **Accuracy**: Must correctly identify 95%+ of actual god objects
3. **False Positives**: Reduce false positive rate for well-organized modules by 80%+
4. **Backward Compatibility**: Maintain existing god object scoring for actual god classes

## Acceptance Criteria

- [ ] **AC1**: Analyze each struct independently and report per-struct metrics (method count, field count, responsibilities)
- [ ] **AC2**: Distinguish between "god class" (single large struct) and "god module" (many small structs) in report output
- [ ] **AC3**: Provide different recommendations for god classes ("Extract 6 functions from X struct") vs god modules ("Split into sub-modules: scoring.rs, detection.rs")
- [ ] **AC4**: Use accurate terminology: "module" for files, "struct/class" for types
- [ ] **AC5**: Show struct-level breakdown in verbose output (e.g., "Largest struct: DebtmapConfig with 25 methods")
- [ ] **AC6**: Only flag as "god class" when individual struct exceeds thresholds, not just file aggregate
- [ ] **AC7**: For multi-struct modules, recommend domain-based splitting with suggested module names
- [ ] **AC8**: Maintain backward compatibility with existing god object scores for actual monolithic classes
- [ ] **AC9**: Add test coverage for both god class and god module scenarios
- [ ] **AC10**: Update documentation to explain the distinction between file-level and struct-level god objects

## Technical Details

### Implementation Approach

1. **Enhanced AST Analysis**
   - Modify `GodObjectDetector` to track per-struct metrics
   - Group methods by their parent impl block
   - Calculate responsibilities per struct, not per file
   - Track standalone functions separately from struct methods

2. **Classification Logic**
   ```rust
   enum GodObjectType {
       GodClass {
           struct_name: String,
           method_count: usize,
           field_count: usize,
           responsibilities: usize,
       },
       GodModule {
           total_structs: usize,
           total_methods: usize,
           largest_struct: StructMetrics,
           suggested_splits: Vec<ModuleSplit>,
       },
       NotGodObject,
   }
   ```

3. **Recommendation Engine**
   - Pattern matching on struct names to suggest logical groupings
   - For config files: group by domain (scoring, detection, thresholds)
   - For other files: suggest splitting by responsibility groups
   - Provide concrete module names in recommendations

### Architecture Changes

**Modified Components:**
- `src/organization/god_object_detector.rs`: Add per-struct analysis
- `src/organization/god_object_metrics.rs`: Add `GodObjectType` enum
- `src/organization/god_object_analysis.rs`: Update analysis logic
- `src/priority/formatter_markdown.rs`: Improve recommendation formatting

**New Data Structures:**
```rust
pub struct StructMetrics {
    pub name: String,
    pub method_count: usize,
    pub field_count: usize,
    pub responsibilities: Vec<String>,
    pub line_span: (usize, usize),
}

pub struct ModuleSplit {
    pub suggested_name: String,
    pub included_structs: Vec<String>,
    pub estimated_lines: usize,
}

pub struct EnhancedGodObjectAnalysis {
    pub file_metrics: GodObjectMetrics,
    pub per_struct_metrics: Vec<StructMetrics>,
    pub classification: GodObjectType,
    pub recommendation: String,
}
```

### Domain-Based Split Suggestions

Implement heuristics to suggest logical module splits:

```rust
fn suggest_module_splits(structs: &[StructMetrics]) -> Vec<ModuleSplit> {
    // Group by common patterns:
    // - "*Config" structs together
    // - "*Weights" / "*Multipliers" as scoring module
    // - "*Thresholds" as thresholds module
    // - "*Detection*" as detection module

    // For config.rs example:
    vec![
        ModuleSplit {
            suggested_name: "config/scoring.rs",
            included_structs: vec!["ScoringWeights", "RoleMultipliers", "RoleCoverageWeights"],
            estimated_lines: 350,
        },
        ModuleSplit {
            suggested_name: "config/detection.rs",
            included_structs: vec!["OrchestratorDetectionConfig", "ConstructorDetectionConfig", "AccessorDetectionConfig"],
            estimated_lines: 450,
        },
        // ...
    ]
}
```

## Dependencies

**Prerequisites**: None

**Affected Components**:
- God object detection module (`src/organization/god_object_*.rs`)
- Markdown formatter for recommendations (`src/priority/formatter_markdown.rs`)
- God object tests (`tests/god_object_*.rs`)

**External Dependencies**: None (uses existing `syn` crate for AST analysis)

## Testing Strategy

### Unit Tests

1. **Test per-struct analysis**
   ```rust
   #[test]
   fn test_multi_struct_module_analysis() {
       let code = r#"
           pub struct Small1 { field: u32 }
           impl Small1 { fn method1(&self) {} fn method2(&self) {} }

           pub struct Small2 { field: String }
           impl Small2 { fn method1(&self) {} }
       "#;

       let analysis = analyze_god_object(code);
       assert_eq!(analysis.per_struct_metrics.len(), 2);
       assert_eq!(analysis.classification, GodObjectType::NotGodObject);
   }

   #[test]
   fn test_single_god_class_detection() {
       let code = r#"
           pub struct GodClass {
               // 20 fields
           }
           impl GodClass {
               // 50 methods spanning multiple responsibilities
           }
       "#;

       let analysis = analyze_god_object(code);
       assert!(matches!(analysis.classification, GodObjectType::GodClass { .. }));
   }
   ```

2. **Test recommendation generation**
   - Verify god class recommendations suggest extracting methods
   - Verify god module recommendations suggest domain-based splits
   - Validate suggested module names are sensible

3. **Test edge cases**
   - Single struct with many methods (god class)
   - Many structs with few methods each (god module)
   - Mix of large and small structs (identify the actual god class)
   - Empty files, files with only standalone functions

### Integration Tests

1. **Real-world file analysis**
   - Test against actual `src/config.rs` (should detect god module, not god class)
   - Test against files with actual god classes
   - Validate recommendations match expected patterns

2. **Regression tests**
   - Ensure existing god object detections still work
   - Verify scoring remains consistent for actual god objects
   - Check that false positive rate decreases

### Performance Tests

- Benchmark analysis time increase (<5% overhead)
- Test with large files (5000+ lines)
- Verify memory usage remains acceptable

## Documentation Requirements

### Code Documentation

- Document `GodObjectType` enum variants and their use cases
- Add examples to `StructMetrics` and `ModuleSplit` structs
- Document heuristics for domain-based split suggestions
- Include inline comments explaining classification logic

### User Documentation

Update `docs/GOD_OBJECT_DETECTION.md`:
- Explain distinction between god class and god module
- Provide examples of each detection type
- Document how recommendations differ
- Add guidance on when to split modules vs refactor classes

### Architecture Updates

Update `ARCHITECTURE.md`:
- Document enhanced god object detection approach
- Explain per-struct analysis pipeline
- Describe recommendation generation logic
- Add diagram showing classification decision tree

## Implementation Notes

### Existing Code Reference

The current implementation in `src/organization/god_object_detector.rs:66-97` combines methods from the largest struct with all standalone functions:

```rust
// Current problematic logic:
let mut all_methods = type_info.methods.clone();
all_methods.extend(visitor.standalone_functions.clone());  // ❌ Problem
```

**Fix**: Keep struct methods and standalone functions separate, analyze each struct independently.

### Heuristic Patterns

Common struct naming patterns to recognize:
- Configuration: `*Config`, `*Settings`, `*Options`
- Weights/Scoring: `*Weight`, `*Multiplier`, `*Factor`
- Thresholds: `*Threshold`, `*Limit`, `*Bound`
- Detection: `*Detector`, `*Checker`, `*Validator`
- Data: `*Data`, `*Info`, `*Metrics`

### Backward Compatibility

Ensure existing tests pass:
- `tests/god_object_detection_test.rs`
- `tests/god_object_metrics_test.rs`
- `tests/organization_test.rs`

Maintain existing scoring behavior for actual god classes to avoid breaking user workflows.

## Migration and Compatibility

### Breaking Changes

None - this is an enhancement to existing detection, not a breaking change.

### Configuration

Consider adding configuration options:
```toml
[god_object]
# Enable per-struct analysis (default: true)
per_struct_analysis = true

# Minimum structs to trigger "god module" classification (default: 5)
min_structs_for_module = 5

# Generate split suggestions (default: true)
suggest_splits = true
```

### Gradual Rollout

1. **Phase 1**: Implement per-struct analysis, maintain existing reports
2. **Phase 2**: Add classification (god class vs god module) to verbose output
3. **Phase 3**: Update default recommendations based on classification
4. **Phase 4**: Update documentation and examples

## Success Metrics

### Quantitative
- Reduce false positive rate for multi-struct modules by 80%+
- Maintain 95%+ accuracy for actual god class detection
- Performance overhead <5%
- Zero regression in existing test suite

### Qualitative
- Users report more actionable recommendations
- Reduced confusion about god object reports
- Improved clarity in distinguishing module organization from class bloat
- Better alignment with Rust module organization best practices

## Related Work

### Similar Tools
- **RustAnalyzer**: Does not detect god objects but provides code navigation
- **Clippy**: Has lints for function complexity but not god objects
- **SonarQube**: Detects god classes in OOP languages, not modules

### Prior Art in Debtmap
- Spec 83: Improved pattern recognition
- Spec 82: Enhanced insight generation
- Current god object detection implementation (2024)

## Future Enhancements

Potential follow-up improvements:
1. ML-based module split suggestions using actual code analysis
2. Automatic refactoring script generation for common patterns
3. Inter-module dependency analysis to suggest optimal splits
4. Integration with IDE for real-time god object warnings
5. Python/JavaScript/TypeScript module vs class distinction
