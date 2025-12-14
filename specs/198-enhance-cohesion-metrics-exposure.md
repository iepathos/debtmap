---
number: 198
title: Enhance Cohesion Metrics Exposure in JSON Output
category: compatibility
priority: low
status: draft
dependencies: []
created: 2025-01-13
---

# Specification 198: Enhance Cohesion Metrics Exposure in JSON Output

**Category**: compatibility
**Priority**: low
**Status**: draft
**Dependencies**: None

## Context

Debtmap computes cohesion scores in `src/organization/cohesion_calculator.rs` using the formula:
`cohesion = internal_calls / (internal_calls + external_calls)`

Currently, cohesion is only exposed as `cohesion_score: Option<f64>` within `ModuleSplit` (part of god object analysis). This limits visibility for:
- Files that aren't god objects but have cohesion issues
- Dashboard visualizations of module quality across entire codebase
- Identifying modules with poor internal organization

## Objective

Expose cohesion metrics at the file level in JSON output, providing:
1. Per-file cohesion scores independent of god object detection
2. Component breakdown (internal vs external call counts)
3. Cohesion classification (high/medium/low)

## Requirements

### Functional Requirements

1. **Add cohesion to FileDebtItemOutput**:
   ```json
   {
     "type": "File",
     "cohesion": {
       "score": 0.72,
       "internal_calls": 15,
       "external_calls": 6,
       "classification": "medium",
       "functions_analyzed": 8
     }
   }
   ```

2. **Classification thresholds**:
   - High cohesion: >= 0.7
   - Medium cohesion: 0.4 - 0.7
   - Low cohesion: < 0.4

3. **Add to summary statistics**:
   ```json
   {
     "summary": {
       "cohesion": {
         "average": 0.65,
         "high_cohesion_files": 12,
         "medium_cohesion_files": 8,
         "low_cohesion_files": 3
       }
     }
   }
   ```

### Non-Functional Requirements

- Backward compatible with `skip_serializing_if = "Option::is_none"`
- Reuse existing `calculate_cohesion_score()` function
- Only compute for files with sufficient functions (threshold: 3+)

## Acceptance Criteria

- [ ] FileDebtItemOutput includes optional `cohesion` field
- [ ] Cohesion computed for files with 3+ functions
- [ ] Classification based on defined thresholds
- [ ] Summary includes codebase-wide cohesion statistics
- [ ] TUI displays cohesion in file detail view
- [ ] All existing tests pass

## Technical Details

### Implementation Approach

1. **Create output struct** in `unified.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct CohesionOutput {
       pub score: f64,
       pub internal_calls: usize,
       pub external_calls: usize,
       pub classification: String,
       pub functions_analyzed: usize,
   }
   ```

2. **Reuse existing calculator**:
   - `cohesion_calculator.rs::calculate_cohesion_score()` already computes the metric
   - Need to expose intermediate values (internal/external counts)

3. **Wire into conversion**:
   - Compute cohesion during `convert_to_unified_format()`
   - Only for files meeting function threshold

### Data Flow

```
FileMetrics (parsed functions)
  → calculate_cohesion_for_file()
    → CohesionOutput
      → FileDebtItemOutput.cohesion
        → JSON serialization
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/organization/cohesion_calculator.rs`: Expose intermediate values
  - `src/output/unified.rs`: Add CohesionOutput struct
  - `src/tui/results/`: Display cohesion in file details

## Testing Strategy

- **Unit Tests**: Test cohesion calculation with known call patterns
- **Integration Tests**: Verify JSON output contains cohesion for qualifying files
- **Edge Cases**: Files with 0-2 functions should not have cohesion field

## Documentation Requirements

- **Code Documentation**: Document cohesion formula and thresholds
- **User Documentation**: Explain cohesion interpretation

## Implementation Notes

- Cohesion requires call graph data which may not be available for all files
- Consider caching cohesion during analysis to avoid recomputation
- Low cohesion files are candidates for splitting (relates to god object detection)

## Migration and Compatibility

- No breaking changes - new optional field
- Files without sufficient data will not have cohesion field
