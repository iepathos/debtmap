---
number: 133
title: Refine God Object vs God Module Detection Logic
category: optimization
priority: medium
status: draft
dependencies: [130]
created: 2025-10-27
---

# Specification 133: Refine God Object vs God Module Detection Logic

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: [130 - God Object Detection with DetectionType]

## Context

After implementing Spec 130 to use `DetectionType` for classifying god objects vs god modules, we discovered edge cases where the classification is incorrect:

1. **Config struct files**: Files like `src/config.rs` with a large configuration struct (26 fields) and many module-level functions (217 functions) are classified as GOD OBJECT when they should be GOD MODULE.

2. **Hybrid files**: Files that contain both a struct with fields AND many standalone module functions don't have clear logic for which should dominate the classification.

Current behavior:
- `src/priority/formatter.rs`: Shows as GOD OBJECT (0 methods, 10 fields, 116 functions)
- `src/config.rs`: Shows as GOD OBJECT (1 method, 26 fields, 217 functions)

Both should be GOD MODULE because the module-level functions far outnumber the struct methods.

The current detection logic in `god_object_detector.rs` (lines 520-551) uses a simple rule:
```rust
if primary_type.is_some() {
    detection_type = DetectionType::GodClass
} else {
    detection_type = DetectionType::GodFile
}
```

This doesn't account for the **proportion** of module functions vs struct methods.

## Objective

Refine the god object detection logic to correctly classify files based on the **dominant characteristic**:
- Files where a **struct with methods** is the primary source of complexity → GOD OBJECT (GodClass)
- Files where **module-level functions** dominate → GOD MODULE (GodFile)

## Requirements

### Functional Requirements

1. **Dominance-Based Classification**
   - Calculate the ratio of struct methods to total functions
   - Use a threshold to determine which characteristic dominates
   - Consider both method count AND field count in the decision

2. **Classification Heuristics**
   - If methods_count >= 20 AND methods represent >50% of total functions → GodClass
   - If standalone_functions_count >= 50 AND standalone functions >70% of total → GodFile
   - For hybrid files (30-70% split), use weighted scoring based on:
     - Field count (more fields → suggests GodClass)
     - Method complexity vs function complexity
     - Responsibilities distribution

3. **WHY Message Accuracy**
   - GOD OBJECT messages should mention struct/class with methods and fields
   - GOD MODULE messages should mention module functions only
   - Messages should accurately reflect the actual counts

4. **Backwards Compatibility**
   - Maintain existing `DetectionType` enum
   - Keep existing serialization format
   - Don't break existing API or output format

### Non-Functional Requirements

1. **Performance**: Detection logic should add <5% overhead to analysis time
2. **Maintainability**: Logic should be clearly documented with rationale
3. **Testability**: Add unit tests for all classification edge cases
4. **Accuracy**: Classification should match human judgment in >95% of cases

## Acceptance Criteria

- [ ] Files with >70% standalone functions are classified as GodFile
- [ ] Files with >50% struct methods AND significant fields (>5) are classified as GodClass
- [ ] Config structs with many fields but few methods are classified as GodFile
- [ ] WHY messages accurately describe the actual file structure
- [ ] All existing tests continue to pass
- [ ] New tests cover edge cases:
  - Pure module file (0 methods, 100 functions)
  - Pure class file (100 methods, 0 standalone functions)
  - Hybrid file (50 methods, 50 functions)
  - Config file (1 method, 26 fields, 200 functions)
- [ ] METRICS section shows correct method vs function counts
- [ ] Run debtmap on itself and verify formatter.rs and config.rs are GOD MODULE

## Technical Details

### Implementation Approach

1. **Add Classification Metrics**
   ```rust
   struct ClassificationMetrics {
       struct_methods: usize,
       standalone_functions: usize,
       field_count: usize,
       method_complexity_avg: f64,
       function_complexity_avg: f64,
   }
   ```

2. **Implement Dominance Calculation**
   ```rust
   fn determine_detection_type(metrics: &ClassificationMetrics) -> DetectionType {
       let total_functions = metrics.struct_methods + metrics.standalone_functions;
       let method_ratio = metrics.struct_methods as f64 / total_functions as f64;

       // Strong indicators of GodClass
       if metrics.struct_methods >= 20
           && method_ratio > 0.5
           && metrics.field_count > 5 {
           return DetectionType::GodClass;
       }

       // Strong indicators of GodFile
       if metrics.standalone_functions >= 50
           && method_ratio < 0.3 {
           return DetectionType::GodFile;
       }

       // Hybrid case - use weighted scoring
       calculate_weighted_classification(metrics)
   }
   ```

3. **Update god_object_detector.rs**
   - Modify `analyze_comprehensive()` to collect classification metrics
   - Replace simple primary_type check with dominance-based logic
   - Preserve purity weighting and complexity weighting
   - Update tests for new classification logic

4. **Update Formatter Messages**
   - Ensure GOD OBJECT messages reference struct/class terminology
   - Ensure GOD MODULE messages reference module functions
   - Display accurate counts in METRICS section

### Architecture Changes

**Files to modify**:
- `src/organization/god_object_detector.rs`: Add classification metrics and dominance logic
- `src/priority/formatter.rs`: Verify messages use detection_type correctly (already done in Spec 130)
- `tests/god_object_detection_tests.rs`: Add edge case tests

**New functions**:
```rust
// In god_object_detector.rs
fn collect_classification_metrics(visitor: &RustVisitor) -> ClassificationMetrics;
fn calculate_dominance_score(metrics: &ClassificationMetrics) -> f64;
fn determine_detection_type_refined(metrics: &ClassificationMetrics) -> DetectionType;
```

### Data Structures

No new data structures needed. Use existing:
- `DetectionType` enum (GodClass, GodFile, GodModule)
- `GodObjectAnalysis` struct
- `RustVisitor` for collecting function/method counts

### Edge Case Handling

| Scenario | Methods | Functions | Fields | Classification | Rationale |
|----------|---------|-----------|--------|----------------|-----------|
| Pure module | 0 | 100 | 0-2 | GodFile | No struct methods |
| Pure class | 100 | 0-10 | 10+ | GodClass | All functions are methods |
| Config struct | 0-5 | 200 | 20+ | GodFile | Functions dominate despite fields |
| Hybrid balanced | 50 | 50 | 8 | GodClass | Tie-breaker: significant fields |
| Service with utils | 30 | 70 | 3 | GodFile | Functions dominate (70%) |

## Dependencies

- **Prerequisites**:
  - Spec 130 (God Object Detection with DetectionType) - COMPLETE
  - Current god_object_detector.rs implementation

- **Affected Components**:
  - `src/organization/god_object_detector.rs` (main changes)
  - `src/priority/formatter.rs` (verify messages)
  - Test files for god object detection

- **External Dependencies**: None

## Testing Strategy

### Unit Tests

1. **Classification Logic Tests**
   ```rust
   #[test]
   fn test_pure_module_file() {
       let metrics = ClassificationMetrics {
           struct_methods: 0,
           standalone_functions: 100,
           field_count: 0,
           ...
       };
       assert_eq!(determine_detection_type(&metrics), DetectionType::GodFile);
   }

   #[test]
   fn test_config_struct_file() {
       let metrics = ClassificationMetrics {
           struct_methods: 1,
           standalone_functions: 200,
           field_count: 26,
           ...
       };
       assert_eq!(determine_detection_type(&metrics), DetectionType::GodFile);
   }

   #[test]
   fn test_pure_class_file() {
       let metrics = ClassificationMetrics {
           struct_methods: 100,
           standalone_functions: 5,
           field_count: 15,
           ...
       };
       assert_eq!(determine_detection_type(&metrics), DetectionType::GodClass);
   }
   ```

2. **Integration Tests**
   - Test on real debtmap files (formatter.rs, config.rs)
   - Verify output classification matches expected
   - Check METRICS section displays correct counts

### Regression Tests

- All existing god object detection tests must pass
- Existing files that were correctly classified remain correct
- No changes to serialization format

### Manual Validation

- Run `debtmap analyze .` on debtmap codebase
- Verify formatter.rs shows as GOD MODULE
- Verify config.rs shows as GOD MODULE
- Verify shared_cache/mod.rs shows as GOD OBJECT
- Check WHY messages are accurate

## Documentation Requirements

### Code Documentation

1. **Inline Comments**
   - Document classification thresholds and rationale
   - Explain dominance calculation algorithm
   - Note edge cases and special handling

2. **Function Documentation**
   ```rust
   /// Determines the detection type based on classification metrics.
   ///
   /// Uses a dominance-based approach:
   /// - GodClass: Struct methods dominate (>50%) with significant fields (>5)
   /// - GodFile: Standalone functions dominate (>70%)
   /// - Hybrid: Weighted scoring based on multiple factors
   ///
   /// # Arguments
   /// * `metrics` - Classification metrics including method/function counts
   ///
   /// # Returns
   /// The appropriate DetectionType for this file
   fn determine_detection_type_refined(metrics: &ClassificationMetrics) -> DetectionType;
   ```

### User Documentation

Update README.md or ARCHITECTURE.md with:
- Explanation of god object vs god module distinction
- Classification criteria and thresholds
- Examples of each category

## Implementation Notes

### Threshold Selection

The proposed thresholds are based on empirical analysis:
- **50% method ratio for GodClass**: Files where half or more functions are methods likely focus on a class
- **70% function ratio for GodFile**: Strong indication of module-based organization
- **5 field threshold**: Significant state suggests class-based design

These may need tuning based on real-world testing.

### Complexity Weighting

Consider incorporating complexity weighting:
- Methods with high complexity may indicate GodClass even if count is lower
- Simple standalone functions (e.g., getters) shouldn't dominate classification

### Purity Weighting Integration

Maintain existing purity weighting from Spec 130:
- Pure functions suggest better module organization
- Impure methods suggest stateful class design

## Migration and Compatibility

### Breaking Changes

None. This is a refinement of classification logic, not a breaking API change.

### Data Migration

No data migration needed. Existing cached results will be regenerated on next analysis.

### Compatibility Considerations

- Serialization format unchanged (DetectionType enum values stay same)
- Output format compatible with existing tools
- No breaking changes to public APIs

## Success Metrics

1. **Classification Accuracy**: >95% agreement with manual review of 50 random files
2. **Edge Case Coverage**: All identified edge cases classified correctly
3. **Performance Impact**: <5% increase in analysis time
4. **User Satisfaction**: No user complaints about misclassification in GitHub issues

## Future Enhancements

Potential improvements beyond this spec:
- Machine learning-based classification
- Language-specific thresholds (Python vs Rust vs JavaScript)
- User-configurable classification thresholds
- Interactive classification correction tool
