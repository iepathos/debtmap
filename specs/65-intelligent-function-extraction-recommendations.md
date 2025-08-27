---
number: 65
title: Intelligent Function Extraction Recommendations
category: optimization
priority: high
status: draft
dependencies: [45, 46]
created: 2025-01-27
---

# Specification 65: Intelligent Function Extraction Recommendations

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [45, 46]

## Context

Currently, debtmap provides simplistic function extraction recommendations based solely on cyclomatic complexity thresholds (e.g., "Extract 4 pure functions to reduce complexity from 16 to <10"). These recommendations:

1. Use arbitrary thresholds without considering actual code structure
2. Don't identify what specific code blocks can be extracted
3. Provide the same generic advice for all code types (parsers, business logic, etc.)
4. Don't leverage the existing AST traversal and data flow analysis capabilities
5. Lack actionable guidance on what to extract and how

The system already has sophisticated AST traversal and data flow analysis for multiple languages (Rust, Python, JavaScript/TypeScript). This infrastructure should be leveraged to provide intelligent, specific recommendations about what code can actually be extracted into pure functions.

## Objective

Enhance debtmap's recommendation engine to analyze code structure and provide specific, actionable function extraction recommendations that:
- Identify actual extractable code patterns within functions
- Provide concrete suggestions with line numbers and suggested names
- Adapt to different programming languages while maintaining consistency
- Leverage existing AST and data flow analysis infrastructure
- Calculate realistic complexity reduction estimates based on actual extraction possibilities

## Requirements

### Functional Requirements

1. **Pattern Detection Engine**
   - Detect language-agnostic extractable patterns (accumulation loops, guard chains, transformation pipelines)
   - Support pattern detection for Rust, Python, JavaScript/TypeScript
   - Identify pure computation blocks without side effects
   - Recognize common refactoring opportunities (similar switch branches, nested extractions)
   - Calculate confidence scores for each extraction opportunity

2. **Extraction Analysis**
   - Analyze data dependencies to determine extraction feasibility
   - Identify required parameters and return types for extracted functions
   - Detect side effects that would prevent extraction
   - Calculate coupling and cohesion metrics for proposed extractions
   - Determine optimal extraction boundaries

3. **Recommendation Generation**
   - Generate specific extraction suggestions with line ranges
   - Provide meaningful names for extracted functions based on operations
   - Include confidence scores and expected complexity reduction
   - Offer language-idiomatic extraction patterns
   - Prioritize extractions by value and ease of implementation

4. **Multi-Language Support**
   - Implement language-specific pattern matchers for each supported language
   - Maintain consistent pattern detection across languages
   - Generate language-appropriate extraction suggestions
   - Handle language-specific constraints (ownership in Rust, async in JS, etc.)

### Non-Functional Requirements

1. **Performance**
   - Pattern detection should add <10% overhead to existing analysis
   - Support incremental analysis for large codebases
   - Cache pattern detection results for unchanged functions

2. **Accuracy**
   - 90%+ confidence extractions should compile without modification
   - False positive rate <5% for high-confidence suggestions
   - Correctly identify side effects and dependencies

3. **Usability**
   - Recommendations include example code transformations
   - Clear explanation of why extraction is beneficial
   - Progressive disclosure (summary → detailed analysis)
   - Integration with existing verbosity levels

## Acceptance Criteria

- [ ] Pattern detection identifies at least 5 common extractable patterns
- [ ] Extraction recommendations include specific line ranges and suggested names
- [ ] Confidence scoring accurately predicts extraction success (>85% accuracy)
- [ ] Multi-language support works consistently across Rust, Python, and JavaScript
- [ ] Generated function names are meaningful and follow language conventions
- [ ] Complexity reduction estimates are within 20% of actual post-extraction complexity
- [ ] Performance overhead is less than 10% of base analysis time
- [ ] High-confidence extractions (>90%) compile successfully when applied
- [ ] Documentation includes pattern catalog with examples
- [ ] Integration tests cover all supported patterns and languages

## Technical Details

### Implementation Approach

1. **Pattern Definition System**
```rust
enum ExtractablePattern {
    AccumulationLoop {
        iterator_binding: String,
        accumulator: String,
        operation: AccumulationOp,
        filter: Option<Expression>,
        transform: Option<Expression>,
    },
    GuardChainSequence {
        checks: Vec<GuardCheck>,
        early_return: ReturnType,
    },
    TransformationPipeline {
        stages: Vec<TransformStage>,
        input_binding: String,
        output_type: Type,
    },
    SimilarBranches {
        condition_var: String,
        common_operations: Vec<Statement>,
        branch_specific: Vec<Vec<Statement>>,
    },
}
```

2. **Pattern Matcher Interface**
```rust
trait PatternMatcher {
    fn match_patterns(&self, ast: &ASTNode) -> Vec<MatchedPattern>;
    fn score_confidence(&self, pattern: &MatchedPattern, context: &AnalysisContext) -> f32;
    fn generate_extraction(&self, pattern: &MatchedPattern) -> ExtractionSuggestion;
}
```

3. **Language-Specific Implementations**
   - Implement `RustPatternMatcher`, `PythonPatternMatcher`, `JavaScriptPatternMatcher`
   - Share common pattern detection logic through traits
   - Generate language-idiomatic suggestions

### Architecture Changes

1. **New Modules**
   - `extraction_patterns/mod.rs` - Core pattern definitions and matching logic
   - `extraction_patterns/language_specific/` - Language-specific implementations
   - `extraction_patterns/confidence.rs` - Confidence scoring system
   - `extraction_patterns/naming.rs` - Function name inference

2. **Integration Points**
   - Extend `UnifiedAnalysis` to include extraction recommendations
   - Integrate with existing `FunctionMetrics` structure
   - Leverage existing AST traversal in language analyzers
   - Use data flow analysis for dependency detection

### Data Structures

```rust
struct ExtractionSuggestion {
    pattern_type: ExtractablePattern,
    start_line: usize,
    end_line: usize,
    suggested_name: String,
    confidence: f32,
    parameters: Vec<Parameter>,
    return_type: Type,
    complexity_reduction: ComplexityImpact,
    example_transformation: String,
}

struct ComplexityImpact {
    current_cyclomatic: u32,
    predicted_cyclomatic: u32,
    current_cognitive: u32,
    predicted_cognitive: u32,
    extracted_function_complexity: u32,
}
```

### APIs and Interfaces

```rust
pub trait ExtractionAnalyzer {
    fn analyze_function(
        &self,
        func: &FunctionMetrics,
        ast: &ASTNode,
        data_flow: &DataFlowGraph,
    ) -> Vec<ExtractionSuggestion>;
    
    fn generate_recommendation(
        &self,
        suggestion: &ExtractionSuggestion,
        verbosity: VerbosityLevel,
    ) -> String;
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 45: Actionable Recommendation System (for recommendation format)
  - Spec 46: Intelligent Pattern Learning System (for pattern recognition infrastructure)
- **Affected Components**:
  - `priority/unified_scorer.rs` - Replace simplistic extraction calculation
  - `analyzers/` - Extend language analyzers with pattern detection
  - `refactoring/` - Enhance with specific extraction patterns
- **External Dependencies**: None required

## Testing Strategy

- **Unit Tests**:
  - Pattern detection for each supported pattern type
  - Confidence scoring accuracy
  - Name inference correctness
  - Complexity prediction validation

- **Integration Tests**:
  - End-to-end extraction recommendations for sample functions
  - Multi-language pattern detection consistency
  - Performance benchmarks
  - False positive/negative rates

- **Performance Tests**:
  - Measure overhead on large codebases
  - Cache effectiveness
  - Memory usage for pattern storage

- **User Acceptance**:
  - Apply high-confidence suggestions and verify compilation
  - Measure actual vs. predicted complexity reduction
  - Validate recommendation clarity and actionability

## Documentation Requirements

- **Code Documentation**:
  - Document each extractable pattern with examples
  - Explain confidence scoring algorithm
  - Provide pattern matching implementation details

- **User Documentation**:
  - Pattern catalog with before/after examples
  - Guide for interpreting extraction recommendations
  - Language-specific extraction best practices

- **Architecture Updates**:
  - Update ARCHITECTURE.md with pattern detection system
  - Document integration with existing analysis pipeline
  - Add data flow diagrams for extraction analysis

## Implementation Notes

### Pattern Priority Order

1. **Accumulation Loops** - Most common, highest value
2. **Guard Clause Chains** - Improves readability significantly  
3. **Transformation Pipelines** - Natural functional decomposition
4. **Similar Branches** - Reduces duplication
5. **Nested Extractions** - Flattens complexity

### Confidence Factors

- **High (>90%)**: Pure functions, no external state, clear boundaries
- **Medium (70-90%)**: Some external references, may need parameter passing
- **Low (<70%)**: Complex dependencies, side effects, unclear boundaries

### Language-Specific Considerations

- **Rust**: Consider ownership, borrowing, and lifetime implications
- **Python**: Handle dynamic typing and list comprehensions
- **JavaScript**: Account for closures, async/await, and this binding

### Edge Cases

- Recursive functions requiring self-reference
- Functions with complex error handling
- Template/generic functions with type constraints
- Async functions with await points
- Functions with inline assembly or unsafe blocks

## Migration and Compatibility

During prototype phase:
- Replace existing `calculate_functions_to_extract()` with new system
- Backward compatibility not required
- Focus on accuracy over maintaining old behavior
- Existing tests may need updating for improved recommendations

## Success Metrics

- 80% of high-confidence extractions compile without modification
- 50% reduction in average extraction suggestion to actual complexity reduction error
- 90% user satisfaction with recommendation specificity
- 25% increase in developers acting on extraction recommendations
- <10% performance overhead on analysis time