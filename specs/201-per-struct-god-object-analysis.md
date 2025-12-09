---
number: 201
title: Per-Struct God Object Analysis
category: optimization
priority: high
status: draft
dependencies: [133]
created: 2025-12-09
---

# Specification 201: Per-Struct God Object Analysis

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [133 - God Object Detection Refinement]

## Context

The god object detector currently aggregates metrics at the **file level** but reports them as if they belong to individual structs. This causes severe false positives.

### The Bug

When analyzing a file like `validation.rs` (1003 lines) containing multiple types:
- `CallGraphValidationConfig` (2 fields, ~6 methods)
- `StructuralIssue` (enum)
- `ValidationWarning` (enum)
- `ValidationReport` (5 fields, ~3 methods)
- `ValidationStatistics` (6 fields, 0 methods)
- `CallGraphValidator` (0 fields, ~15 methods)
- `Expectation` (2 fields, ~2 methods)

The detector incorrectly reports `ValidationStatistics` as:
- **1002 LOC** (entire file, not struct's ~10 lines)
- **24 methods** (all impl methods in file, not struct's 0)
- **15 fields** (all fields in file, not struct's 6)
- **4 responsibilities** (file-level grouping)

### Root Cause

Three bugs in `detector.rs`:

1. **Line 333-337**: LOC counts entire file content
   ```rust
   let lines_of_code = self.source_content.as_ref()
       .map(|content| content.lines().count())  // Entire file!
   ```

2. **Line 303**: Field count aggregated across all structs
   ```rust
   let field_count: usize = visitor.types.values().map(|t| t.field_count).sum();
   ```

3. **Line 255**: Method count aggregated across all impl blocks
   ```rust
   let impl_method_count: usize = visitor.types.values().map(|t| t.method_count).sum();
   ```

### Impact

- Simple DTOs in large files flagged as god objects
- Health scores incorrectly penalized
- User trust in tool accuracy undermined
- False positives obscure real issues

## Objective

Refactor god object detection to analyze **each struct/type individually**, using per-struct metrics (LOC, methods, fields, responsibilities) instead of file-level aggregates.

## Requirements

### Functional Requirements

1. **Per-Struct Metric Collection**
   - Calculate LOC using struct's `location.line` to `location.end_line`
   - Count only methods in that struct's impl block(s)
   - Count only fields belonging to that struct
   - Group responsibilities within that struct's methods only

2. **Per-Struct Scoring**
   - Score each struct independently
   - Only flag structs that individually exceed thresholds
   - Generate separate `GodObjectAnalysis` per problematic struct

3. **Multi-Struct File Handling**
   - Analyze all structs in a file
   - Return list of problematic structs (may be 0, 1, or many)
   - Each result references its specific struct, not the file

4. **Simple Struct Filtering**
   - Structs with 0 impl methods should never be flagged as god objects
   - DTOs (data-only structs) should be recognized and excluded
   - Apply DTO detection per-struct, not per-file

5. **Accurate Reporting**
   - Report struct name, not just file path
   - Show actual per-struct metrics in output
   - Location should point to struct definition, not file start

### Non-Functional Requirements

1. **Performance**: Analysis time should not increase significantly
2. **Backwards Compatibility**: Output format should remain compatible
3. **Accuracy**: Zero false positives for zero-method structs
4. **Testability**: Per-struct analysis enables easier unit testing

## Acceptance Criteria

- [ ] `ValidationStatistics` (6 fields, 0 methods) is NOT flagged as god object
- [ ] Per-struct LOC uses line span from `TypeAnalysis.location`
- [ ] Per-struct method count uses `TypeAnalysis.method_count`
- [ ] Per-struct field count uses `TypeAnalysis.field_count`
- [ ] Structs with 0 impl methods are never flagged as god objects
- [ ] Multi-struct files can have 0, 1, or multiple god objects flagged
- [ ] Each flagged struct shows its own metrics, not file aggregates
- [ ] All existing tests pass (with updates for new behavior)
- [ ] New tests cover:
  - File with one god object struct and many simple structs
  - File with only DTOs (no god objects)
  - File with multiple god object structs
  - Single-struct file (current behavior preserved)
- [ ] Self-analysis: debtmap run on itself shows no false positives for DTOs

## Technical Details

### Implementation Approach

#### Phase 1: Refactor Core Analysis Loop

Change from file-level to per-struct analysis:

```rust
// Current (file-level):
pub fn analyze_comprehensive(&self, path: &Path, ast: &syn::File) -> GodObjectAnalysis {
    let field_count: usize = visitor.types.values().map(|t| t.field_count).sum();
    let impl_method_count: usize = visitor.types.values().map(|t| t.method_count).sum();
    let lines_of_code = content.lines().count();
    // ... score file as a whole
}

// New (per-struct):
pub fn analyze_comprehensive(&self, path: &Path, ast: &syn::File) -> Vec<GodObjectAnalysis> {
    visitor.types.values()
        .filter_map(|type_analysis| {
            self.analyze_single_struct(path, type_analysis, &visitor)
        })
        .collect()
}

fn analyze_single_struct(
    &self,
    path: &Path,
    type_analysis: &TypeAnalysis,
    visitor: &TypeVisitor,
) -> Option<GodObjectAnalysis> {
    // Skip zero-method structs immediately
    if type_analysis.method_count == 0 {
        return None;
    }

    let lines_of_code = type_analysis.location.end_line
        .unwrap_or(type_analysis.location.line)
        .saturating_sub(type_analysis.location.line) + 1;

    let field_count = type_analysis.field_count;
    let method_count = type_analysis.method_count;

    // Score THIS struct only
    let score = calculate_god_object_score(
        method_count, field_count, responsibilities, lines_of_code, &thresholds
    );

    if score < threshold {
        return None; // Not a god object
    }

    Some(GodObjectAnalysis {
        struct_name: Some(type_analysis.name.clone()),
        // ... per-struct metrics
    })
}
```

#### Phase 2: Update GodObjectAnalysis Struct

Add struct-specific fields:

```rust
pub struct GodObjectAnalysis {
    // Existing fields...

    /// Name of the specific struct/type (if per-struct analysis)
    pub struct_name: Option<String>,

    /// Location of the struct definition
    pub struct_location: Option<SourceLocation>,
}
```

#### Phase 3: Update Callers

Update `analyze_enhanced` and callers to handle `Vec<GodObjectAnalysis>`:

```rust
// In file_analyzer.rs
let analyses = detector.analyze_comprehensive(path, &ast);
for analysis in analyses {
    if analysis.is_god_object {
        // Report each god object separately
    }
}
```

#### Phase 4: Update Formatters

Modify output to show struct name:

```
GOD OBJECT: src/validation.rs::CallGraphValidator
  Methods: 15, Fields: 0, Responsibilities: 4
  LOC: 200 (lines 185-384)
```

### Data Flow

```
File AST
    ↓
TypeVisitor (collects per-type data)
    ↓
For each TypeAnalysis:
    ↓
    analyze_single_struct()
        ↓
        Skip if method_count == 0
        ↓
        Calculate per-struct metrics
        ↓
        Score per-struct
        ↓
        Return GodObjectAnalysis if over threshold
    ↓
Filter to non-None results
    ↓
Vec<GodObjectAnalysis>
```

### Existing Infrastructure

The per-struct data already exists in `TypeAnalysis`:

```rust
pub struct TypeAnalysis {
    pub name: String,
    pub method_count: usize,      // Per-struct methods
    pub field_count: usize,       // Per-struct fields
    pub methods: Vec<String>,     // Method names for this struct
    pub fields: Vec<String>,      // Field names for this struct
    pub location: SourceLocation, // Includes line and end_line
}
```

And `build_per_struct_metrics()` in `metrics.rs` already builds accurate per-struct data:

```rust
pub fn build_per_struct_metrics(visitor: &TypeVisitor) -> Vec<StructMetrics> {
    visitor.types.values()
        .map(|type_analysis| StructMetrics {
            name: type_analysis.name.clone(),
            method_count: type_analysis.method_count,
            field_count: type_analysis.field_count,
            line_span: (
                type_analysis.location.line,
                type_analysis.location.end_line.unwrap_or(type_analysis.location.line),
            ),
            // ...
        })
        .collect()
}
```

### Files to Modify

| File | Changes |
|------|---------|
| `src/organization/god_object/detector.rs` | Refactor `analyze_comprehensive` to per-struct loop |
| `src/organization/god_object/core_types.rs` | Add `struct_name`, `struct_location` to `GodObjectAnalysis` |
| `src/analyzers/file_analyzer.rs` | Handle `Vec<GodObjectAnalysis>` return |
| `src/analyzers/enhanced_analyzer.rs` | Handle `Vec<GodObjectAnalysis>` return |
| `src/priority/formatter.rs` | Display struct name in output |

### Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| DTO struct (fields only, no methods) | Never flagged, skip analysis |
| Empty struct (no fields, no methods) | Never flagged, skip analysis |
| Enum with no impl | Never flagged, skip analysis |
| Struct with only test methods | Skip test methods, may not be flagged |
| Multiple structs in one impl block | Track impl target, attribute correctly |
| Nested types | Each analyzed independently |

## Dependencies

- **Prerequisites**:
  - Spec 133 (God Object Detection Refinement) - provides DetectionType
  - Current `TypeAnalysis` infrastructure with per-struct data

- **Affected Components**:
  - `src/organization/god_object/detector.rs` (main changes)
  - `src/organization/god_object/core_types.rs` (struct updates)
  - `src/analyzers/file_analyzer.rs` (caller update)
  - `src/analyzers/enhanced_analyzer.rs` (caller update)
  - `src/priority/formatter.rs` (output format)

- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_dto_not_flagged() {
    let code = r#"
        pub struct ValidationStatistics {
            pub total_functions: usize,
            pub entry_points: usize,
            pub leaf_functions: usize,
        }
        // No impl block
    "#;
    let ast = syn::parse_file(code).unwrap();
    let detector = GodObjectDetector::with_source_content(code);
    let results = detector.analyze_comprehensive(Path::new("test.rs"), &ast);

    assert!(results.is_empty(), "DTO should not be flagged");
}

#[test]
fn test_per_struct_metrics() {
    let code = r#"
        pub struct SmallDto { a: i32, b: i32 }

        pub struct LargeService {
            cache: HashMap<String, String>,
            // ... many fields
        }

        impl LargeService {
            pub fn method1(&self) { /* ... */ }
            pub fn method2(&self) { /* ... */ }
            // ... 20+ methods
        }
    "#;
    let ast = syn::parse_file(code).unwrap();
    let detector = GodObjectDetector::with_source_content(code);
    let results = detector.analyze_comprehensive(Path::new("test.rs"), &ast);

    // Only LargeService should be flagged
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].struct_name, Some("LargeService".to_string()));
}

#[test]
fn test_multiple_god_objects() {
    let code = r#"
        pub struct GodA { /* many fields */ }
        impl GodA { /* 20+ methods */ }

        pub struct GodB { /* many fields */ }
        impl GodB { /* 20+ methods */ }
    "#;
    let ast = syn::parse_file(code).unwrap();
    let detector = GodObjectDetector::with_source_content(code);
    let results = detector.analyze_comprehensive(Path::new("test.rs"), &ast);

    assert_eq!(results.len(), 2);
}
```

### Integration Tests

- Run debtmap on itself
- Verify `ValidationStatistics` is NOT flagged
- Verify actual god objects (if any) ARE flagged with correct metrics

### Regression Tests

- All existing tests updated to handle `Vec<GodObjectAnalysis>`
- Behavior for single-struct files should match previous behavior

## Documentation Requirements

### Code Documentation

- Document per-struct analysis approach in module docs
- Explain why file-level aggregation was incorrect
- Document zero-method struct exclusion logic

### User Documentation

Update any user docs to clarify:
- God object detection is per-struct, not per-file
- DTOs and data structs are excluded from detection
- Each flagged item references a specific struct

## Migration and Compatibility

### Breaking Changes

- Return type changes from `GodObjectAnalysis` to `Vec<GodObjectAnalysis>`
- Callers must be updated to iterate results

### Backwards Compatibility

- Output format remains similar (one entry per god object)
- Serialization format compatible
- Single-struct files produce equivalent results

### Migration Steps

1. Update `GodObjectAnalysis` struct with new fields
2. Refactor `analyze_comprehensive` to per-struct loop
3. Update all callers to handle Vec
4. Update formatters to show struct names
5. Update tests

## Success Metrics

1. **Zero false positives** for zero-method structs
2. **Accurate metrics**: reported LOC/methods/fields match actual struct
3. **Test coverage**: >90% of new code covered
4. **Self-analysis clean**: debtmap on itself shows no DTO false positives

## Implementation Notes

### Zero-Method Check

The most critical fix is the early return for zero-method structs:

```rust
if type_analysis.method_count == 0 {
    return None; // Cannot be a god OBJECT without behavior
}
```

This alone eliminates most false positives.

### LOC Calculation

Use the existing line span data:

```rust
let lines_of_code = type_analysis.location.end_line
    .unwrap_or(type_analysis.location.line)
    .saturating_sub(type_analysis.location.line) + 1;
```

### Method Attribution

Methods are already correctly attributed per-struct in `TypeAnalysis.method_count` and `TypeAnalysis.methods`. The bug was in the aggregation step, not the collection step.

## Future Enhancements

- Per-method complexity breakdown within each struct
- Responsibility clustering per-struct (not per-file)
- Struct-level refactoring suggestions
- Visualization of per-struct metrics
