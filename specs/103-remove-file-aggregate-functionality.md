---
number: 103
title: Remove File Aggregate Functionality
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-09-15
---

# Specification 103: Remove File Aggregate Functionality

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current file aggregation system (`FileAggregateScore`, `AggregationPipeline`) creates misleading and redundant debt recommendations. File aggregates always dominate the top 10 recommendations because they sum complexity scores from multiple functions, but then only recommend fixing 2 individual functions - providing no actionable insight beyond what individual function analysis already offers.

Current problems:
- **Redundant scoring**: Aggregates sum function scores, making them always rank highest
- **Conflicting guidance**: Suggests file-level action but recommends function-level fixes
- **User confusion**: All top recommendations are "FILE AGGREGATE" items that obscure actual priorities
- **No unique value**: Provides no insight beyond "this file has many complex functions"

Example problematic output:
```
#1 SCORE: 6367 [CRITICAL - FILE AGGREGATE]
├─ ./src/cook/execution/mapreduce.rs (66 functions, total score: 3918.9)
├─ ACTION: Focus on the top 2 high-complexity functions listed below
```

This conflates file-level issues (which are legitimate architectural concerns) with function-level complexity aggregation (which is redundant). Users get better guidance by seeing individual function priorities directly.

## Objective

Remove file aggregation functionality while preserving legitimate file-level debt detection (god objects, oversized files, and other architectural file-level issues). Ensure the top 10 recommendations show actionable items ranked by actual priority rather than aggregated complexity.

## Requirements

### Functional Requirements

1. **Remove File Aggregation Components**
   - Remove `FileAggregateScore` struct and related types
   - Remove `AggregationPipeline` and aggregation logic
   - Remove `AggregationConfig` and aggregation settings
   - Remove file aggregate formatting in priority output

2. **Preserve Legitimate File-Level Issues**
   - Keep god object detection for classes/modules with too many responsibilities
   - Keep large file detection for files exceeding size thresholds
   - Keep file-level debt patterns that represent architectural issues
   - Maintain file-level metrics collection and analysis

3. **Enhanced File-Level Debt Detection**
   - Files with consistently poor test coverage (< 30% across many functions)
   - Files with high average complexity (> 15 average cyclomatic complexity)
   - Files with excessive TODO/FIXME density (> 5 per 100 lines)
   - Files with significant code duplication patterns
   - Files violating single responsibility principle (mixed concerns)
   - Files with excessive dependencies (> 20 imports)

4. **Simplified Priority Display**
   - Remove mixed priority system that combines functions and aggregates
   - Display function-level items ranked by actual priority
   - Show file-level items only when they represent genuine architectural issues
   - Provide clear distinction between function-level and file-level recommendations

### Non-Functional Requirements

- Maintain performance levels without aggregation overhead
- Preserve existing file-level metrics collection
- Ensure output is clearer and more actionable
- Maintain compatibility with existing analysis pipeline

## Acceptance Criteria

- [ ] `FileAggregateScore` struct is completely removed from codebase
- [ ] `AggregationPipeline` and related aggregation logic is removed
- [ ] `AggregationConfig` is removed from configuration system
- [ ] Top 10 recommendations show individual function priorities (no aggregates)
- [ ] God object detection remains functional and appears in recommendations
- [ ] Large file detection remains functional and appears in recommendations
- [ ] Enhanced file-level debt detection identifies architectural issues
- [ ] Priority formatter no longer includes file aggregate formatting
- [ ] `get_top_mixed_priorities` is simplified or replaced
- [ ] All aggregation-related tests are removed or updated
- [ ] Output is clearer with actionable function-level recommendations
- [ ] File-level items only appear for legitimate architectural concerns

## Technical Details

### Implementation Approach

1. **Phase 1: Remove Aggregation Infrastructure**
   - Delete `src/priority/aggregation.rs` module
   - Remove aggregation imports and usage throughout codebase
   - Remove `FileAggregateScore` from `UnifiedAnalysis`
   - Clean up aggregation configuration

2. **Phase 2: Enhance File-Level Detection**
   - Extend `FileDebtMetrics` with new architectural indicators
   - Add detection for excessive TODO/FIXME density
   - Add detection for high dependency counts
   - Add detection for poor overall test coverage patterns
   - Add detection for high average complexity thresholds

3. **Phase 3: Simplify Priority Display**
   - Modify `get_top_mixed_priorities` to exclude aggregates
   - Update formatters to remove aggregate-specific formatting
   - Ensure file-level items only appear for architectural issues
   - Maintain clear separation between function and file recommendations

### Architecture Changes

**Removed Components:**
- `src/priority/aggregation.rs` - entire module
- `FileAggregateScore` struct
- `AggregationPipeline` struct
- `AggregationConfig` configuration
- Aggregation methods in `UnifiedAnalysis`

**Enhanced Components:**
- `FileDebtMetrics` - add new architectural indicators
- `FileDebtItem` - enhance with better architectural debt detection
- Priority formatting - remove aggregate formatting, enhance file formatting

**Modified Components:**
- `UnifiedAnalysis::get_top_mixed_priorities` - exclude aggregates
- Priority formatters - remove aggregate-specific code
- Configuration system - remove aggregation settings

### Data Structures

**New File-Level Indicators:**
```rust
pub struct EnhancedFileIndicators {
    pub todo_fixme_density: f64,        // TODOs/FIXMEs per 100 lines
    pub dependency_count: usize,        // Number of imports/dependencies
    pub average_complexity: f64,        // Average function complexity
    pub coverage_consistency: f64,      // Variance in coverage across functions
    pub responsibility_indicators: Vec<String>, // Mixed concerns detected
}
```

**Simplified Priority Types:**
```rust
pub enum PriorityItem {
    Function(UnifiedDebtItem),
    ArchitecturalFile(FileDebtItem), // Only for legitimate architectural issues
}
```

### APIs and Interfaces

**Removed APIs:**
- `AggregationPipeline::new()`
- `AggregationPipeline::aggregate_file_scores()`
- `UnifiedAnalysis::add_file_aggregate()`
- `format_file_aggregate_item()`

**Enhanced APIs:**
- `detect_architectural_file_issues()` - comprehensive file-level analysis
- `calculate_file_debt_indicators()` - enhanced metrics calculation
- `get_priority_recommendations()` - simplified priority retrieval

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - Priority analysis system
  - Output formatting
  - Configuration management
- **External Dependencies**: No new dependencies required

## Testing Strategy

- **Unit Tests**:
  - Test file-level architectural issue detection
  - Test enhanced file metrics calculation
  - Verify aggregation removal doesn't break existing functionality
- **Integration Tests**:
  - Test complete priority analysis pipeline without aggregation
  - Verify output quality and clarity improvements
- **Performance Tests**:
  - Measure performance impact of removing aggregation overhead
  - Ensure enhanced file detection doesn't significantly impact performance
- **User Acceptance**:
  - Validate output is clearer and more actionable
  - Confirm recommendations are properly prioritized

## Documentation Requirements

- **Code Documentation**: Document new file-level indicators and detection logic
- **User Documentation**: Update help text to reflect removal of aggregation
- **Architecture Updates**: Update priority analysis documentation

## Implementation Notes

### File-Level Debt Detection Guidelines

**Legitimate File-Level Issues (Keep):**
- God objects: Classes with > 10 methods and > 5 fields
- God modules: Files with > 30 functions
- Oversized files: Files with > 1000 lines
- Poor architecture: High coupling, low cohesion indicators
- Excessive complexity: Average complexity > 15
- Poor coverage consistency: High variance in function coverage
- TODO/FIXME hotspots: > 5 per 100 lines
- Dependency violations: > 20 imports or circular dependencies

**Invalid Aggregations (Remove):**
- Sum of function complexity scores
- Count of functions exceeding thresholds
- Average function scores weighted by count
- Any metric that's just a rollup of function-level data

### Migration Strategy

**Breaking Changes Allowed:** This is prototype phase, optimize for correctness over compatibility.

**Migration Steps:**
1. Remove aggregation infrastructure completely
2. Enhance file-level detection with architectural indicators
3. Update all output formatting to exclude aggregates
4. Test with real codebases to ensure output quality
5. Update documentation and help text

**Validation Approach:**
- Test on prodigy codebase to ensure better recommendations
- Compare before/after output for clarity improvements
- Verify that legitimate file-level issues are still detected
- Confirm function-level priorities are properly visible

## Expected Impact

**Positive Outcomes:**
- Clearer, more actionable recommendations
- Better priority ranking based on actual impact
- Reduced user confusion about file vs function issues
- Improved development workflow efficiency
- Elimination of redundant "aggregate" recommendations

**Risk Mitigation:**
- Preserve all legitimate file-level architectural issue detection
- Maintain comprehensive function-level analysis
- Ensure enhanced file indicators catch real architectural problems
- Test thoroughly on real codebases before release