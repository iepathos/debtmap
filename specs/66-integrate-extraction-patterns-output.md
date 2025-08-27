---
number: 66
title: Integrate Extraction Patterns into Output
category: optimization
priority: high
status: draft
dependencies: [65]
created: 2025-08-27
---

# Specification 66: Integrate Extraction Patterns into Output

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 65 (Intelligent Function Extraction Recommendations)

## Context

Specification 65 introduced an advanced extraction pattern analyzer capable of identifying specific refactoring opportunities like map-filter-reduce chains, guard clauses, accumulation patterns, and data transformation pipelines. The implementation includes:

- Complete extraction pattern analyzer in `src/extraction_patterns/`
- Language-specific matchers for Rust, Python, and JavaScript
- Pattern confidence scoring based on clarity and side effects
- Integration with DataFlowGraph for side effect detection

However, this sophisticated analysis is currently not integrated into debtmap's output. The function `generate_intelligent_extraction_recommendations()` is marked as dead code, and the system falls back to simple heuristic-based recommendations like "Extract 3 pure functions".

Users are missing out on actionable, pattern-specific refactoring guidance that could significantly improve code quality and reduce technical debt.

## Objective

Fully integrate the extraction pattern analyzer into debtmap's analysis pipeline so that users receive specific, actionable recommendations for function extraction based on identified patterns, replacing generic advice with precise refactoring opportunities.

## Requirements

### Functional Requirements

1. **Remove Dead Code Flag**: Remove `#[allow(dead_code)]` from `generate_intelligent_extraction_recommendations()`

2. **Wire Pattern Analysis to ComplexityHotspot**: 
   - Call extraction pattern analyzer when generating ComplexityHotspot recommendations
   - Pass DataFlowGraph to enable side effect analysis
   - Use pattern-specific suggestions instead of generic counting

3. **Enhanced Recommendation Output**:
   - Include specific pattern types identified (map/filter/reduce, guard clauses, etc.)
   - Provide pattern-specific extraction suggestions
   - Show confidence scores for each recommendation
   - Include before/after code examples where applicable

4. **Verbosity Level Support**:
   - Minimal: Show only high-confidence patterns
   - Standard: Include medium-confidence patterns and brief explanations
   - Detailed: Show all patterns with full analysis and examples

5. **Integration Points**:
   - Modify `generate_recommendation()` for ComplexityHotspot debt type
   - Update `generate_infrastructure_recommendation()` to use pattern analysis
   - Enhance `implementation_steps` with pattern-specific guidance

### Non-Functional Requirements

1. **Performance**: Pattern analysis should add <100ms to analysis time for typical functions
2. **Accuracy**: Confidence scores should reflect actual extraction difficulty
3. **Clarity**: Recommendations must be immediately actionable by developers
4. **Coverage**: Support Rust, Python, and JavaScript equally well

## Acceptance Criteria

- [ ] `generate_intelligent_extraction_recommendations()` is called for ComplexityHotspot debt items
- [ ] Recommendations show specific patterns identified (e.g., "Extract map-filter chain at lines 45-52")
- [ ] Each recommendation includes a confidence score
- [ ] Pattern-specific extraction steps replace generic "Extract N functions" advice
- [ ] DataFlowGraph is properly passed to extraction analyzer
- [ ] Output includes pattern names and descriptions
- [ ] Verbosity levels correctly filter recommendations by confidence
- [ ] All existing tests pass
- [ ] New tests verify pattern-specific recommendations appear in output
- [ ] Performance impact is <100ms per function analyzed

## Technical Details

### Implementation Approach

1. **Activate Pattern Analyzer**:
```rust
// In unified_scorer.rs, modify generate_recommendation()
DebtType::ComplexityHotspot { cyclomatic, cognitive } => {
    let recommendations = generate_intelligent_extraction_recommendations(
        func,
        VerbosityLevel::Standard
    );
    
    if !recommendations.is_empty() {
        // Use pattern-based recommendations
        generate_pattern_based_recommendation(func, recommendations)
    } else {
        // Fall back to current heuristic
        generate_infrastructure_recommendation(debt_type)
    }
}
```

2. **Pass DataFlowGraph**:
```rust
// Modify analyze_function signature to accept DataFlowGraph
let suggestions = analyzer.analyze_function(
    func,
    &file_metrics,
    Some(&unified_analysis.data_flow_graph())
);
```

3. **Format Pattern Recommendations**:
```rust
fn format_pattern_recommendation(pattern: &ExtractablePattern) -> String {
    match pattern {
        ExtractablePattern::MapFilterReduce { .. } => {
            format!("Extract map-filter-reduce chain: {}", pattern.description)
        }
        ExtractablePattern::GuardClauses { .. } => {
            format!("Extract guard clauses for early returns")
        }
        // ... other patterns
    }
}
```

### Architecture Changes

- Modify `UnifiedDebtItem` to optionally include extraction patterns
- Update `ActionableRecommendation` to support pattern-specific guidance
- Enhance `implementation_steps` to be pattern-aware

### Data Structures

```rust
// Add to UnifiedDebtItem
pub extraction_patterns: Option<Vec<IdentifiedPattern>>,

pub struct IdentifiedPattern {
    pub pattern_type: ExtractablePattern,
    pub confidence: f32,
    pub location: SourceLocation,
    pub suggested_name: String,
    pub complexity_reduction: u32,
}
```

### APIs and Interfaces

No external API changes, but internal interfaces will be enhanced:
- `generate_recommendation()` gains pattern awareness
- `calculate_expected_impact()` uses pattern-based estimates
- New helper functions for pattern formatting

## Dependencies

- **Prerequisites**: Spec 65 (already implemented)
- **Affected Components**: 
  - `src/priority/unified_scorer.rs`
  - `src/extraction_patterns/mod.rs`
  - `src/io/writers/markdown.rs`
  - `src/io/writers/terminal.rs`
- **External Dependencies**: None

## Testing Strategy

### Unit Tests
- Test pattern identification for each supported pattern type
- Verify confidence scoring accuracy
- Test recommendation generation for each pattern

### Integration Tests
- Create test files with known patterns
- Verify patterns appear in JSON and Markdown output
- Test verbosity level filtering
- Ensure DataFlowGraph integration works

### Performance Tests
- Measure analysis time with/without pattern detection
- Verify <100ms overhead for typical functions
- Test scalability with large codebases

### User Acceptance
- Run on real-world complex functions
- Verify recommendations are actionable
- Confirm pattern detection accuracy

## Documentation Requirements

### Code Documentation
- Document pattern types and confidence thresholds
- Explain integration flow in unified_scorer.rs
- Add examples of each pattern type

### User Documentation
- Update README with pattern-based recommendations
- Add examples of extraction pattern output
- Document verbosity level differences

### Architecture Updates
- Update ARCHITECTURE.md with pattern analysis flow
- Document DataFlowGraph usage in extraction

## Implementation Notes

### Priority Order
1. Remove dead code annotation and activate function
2. Wire to ComplexityHotspot handling
3. Pass DataFlowGraph properly
4. Format pattern recommendations
5. Add tests
6. Update documentation

### Gotchas
- Some patterns may overlap - prioritize by confidence
- Language-specific patterns need proper file type detection
- DataFlowGraph may be incomplete for some functions

### Migration Path
- Gracefully fall back to heuristic recommendations if pattern analysis fails
- Preserve existing recommendation format for non-pattern cases
- Ensure backward compatibility with existing output parsers

## Migration and Compatibility

Since we're in prototype phase, breaking changes are acceptable:
- Output format may change to include pattern information
- JSON structure will gain new fields for patterns
- Existing integrations may need updates to handle richer recommendations

## Success Metrics

- 80%+ of ComplexityHotspot items show pattern-based recommendations
- Average confidence score >0.7 for identified patterns
- User feedback indicates recommendations are more actionable
- Measurable reduction in complexity after applying recommendations

## Example Output

### Before (Current)
```
Function: process_data (complexity: 15)
Recommendation: Extract 3 pure functions to reduce complexity
Steps:
1. Identify and extract pure functions
2. Add tests for extracted functions
```

### After (With Pattern Integration)
```
Function: process_data (complexity: 15)
Identified Patterns:
  - Map-Filter-Reduce chain (confidence: 0.92)
    Location: lines 45-52
    Suggested: extract as 'transform_and_filter_items()'
  - Guard clauses (confidence: 0.85)
    Location: lines 23-30
    Suggested: extract as 'validate_input()'
  - Accumulation pattern (confidence: 0.78)
    Location: lines 67-75
    Suggested: extract as 'calculate_totals()'

Recommendation: Extract 3 identified patterns to reduce complexity from 15 to 6
Steps:
1. Extract map-filter-reduce chain as pure function 'transform_and_filter_items()'
2. Extract guard clauses into 'validate_input()' for early returns
3. Extract accumulation logic into 'calculate_totals()' with fold operation
4. Add property-based tests for each extracted pure function
Expected complexity reduction: 60%
```