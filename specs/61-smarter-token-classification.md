---
number: 61
title: Smarter Token Classification for Entropy Analysis
category: optimization
priority: high
status: draft
dependencies: [52, 53]
created: 2025-08-21
---

# Specification 61: Smarter Token Classification for Entropy Analysis

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [52 (Entropy-Based Complexity Scoring), 53 (Complete Entropy Implementation)]

## Context

The current entropy analysis implementation uses overly aggressive token normalization that loses critical semantic information. All identifiers longer than 3 characters are normalized to "VAR", making it impossible to distinguish between different types of code patterns. This leads to:

1. **False positives**: Genuinely complex orchestration code appears repetitive
2. **False negatives**: Boilerplate code (getters/setters) isn't properly identified
3. **Lost semantic signals**: Can't differentiate between resource management, validation, I/O operations
4. **Inaccurate entropy scores**: Token diversity is artificially reduced

Real-world impact shows 40-60% of complexity scores are incorrectly dampened or preserved due to this limitation.

## Objective

Implement a sophisticated token classification system that preserves semantic meaning while reducing noise, enabling accurate distinction between genuine complexity and pattern-based repetition. This will improve entropy-based complexity scoring accuracy by 40-60% and reduce false positives in technical debt detection.

## Requirements

### Functional Requirements

1. **Token Classification Hierarchy**
   - Implement multi-level token classification system with semantic categories
   - Support classification of local variables, method calls, field access, and external APIs
   - Preserve meaningful distinctions while abstracting implementation details
   - Enable pattern-specific analysis based on token types

2. **Context-Aware Classification**
   - Use AST context to determine token role (method call vs variable vs field)
   - Detect external crate/module boundaries for API classification
   - Identify well-known patterns (getters, setters, validators, I/O operations)
   - Track variable scope and lifetime for better classification

3. **Weighted Entropy Calculation**
   - Apply complexity weights to different token classes
   - Higher weights for external APIs and error handling
   - Lower weights for iterators and temporary variables
   - Configurable weight system for tuning

4. **Pattern-Specific Dampening**
   - Detect and appropriately dampen getter/setter patterns
   - Identify validation function patterns for moderate dampening
   - Preserve complexity for I/O and external API orchestration
   - Support custom pattern recognition rules

### Non-Functional Requirements

1. **Performance**
   - Classification overhead < 10% of total analysis time
   - Efficient pattern matching using lazy evaluation
   - Cache classification results for repeated tokens
   - Minimal memory overhead for classification metadata

2. **Compatibility**
   - Backward compatible with existing entropy configuration
   - Support gradual migration from old to new classification
   - Maintain existing API contracts
   - Preserve test compatibility

3. **Configurability**
   - User-configurable token classification rules
   - Adjustable complexity weights per token class
   - Enable/disable classification via configuration
   - Support per-project classification tuning

## Acceptance Criteria

- [ ] Token classification system correctly categorizes 95%+ of common patterns
- [ ] Entropy scores show 40%+ improvement in accuracy for test cases
- [ ] False positive rate reduced by 40-60% on validation/boilerplate code
- [ ] Genuine complexity preserved for orchestration and business logic
- [ ] Performance impact < 10% on large codebases
- [ ] All existing entropy tests pass with new classification
- [ ] Configuration allows fine-tuning of classification behavior
- [ ] Documentation includes classification examples and tuning guide

## Technical Details

### Implementation Approach

1. **Token Classification Enum Structure**
```rust
enum TokenClass {
    LocalVar(VarType),        // Categorized local variables
    FieldAccess(AccessType),  // self.field, obj.property patterns
    MethodCall(CallType),     // Categorized method invocations
    ExternalAPI(String),      // External crate/module calls
    ControlFlow(FlowType),    // Control flow constructs
    ErrorHandling(ErrorType), // Result, Option, error patterns
    Collection(CollectionOp), // Collection operations
    Literal(LiteralCategory), // Categorized literals
}
```

2. **Classification Logic Integration**
   - Extend `TokenExtractor` visitor with context tracking
   - Add classification phase before entropy calculation
   - Implement pattern matchers for common code patterns
   - Cache classification results for performance

3. **Weighted Entropy Formula**
   - Replace simple frequency counting with weighted distribution
   - Apply logarithmic scaling for high-frequency patterns
   - Implement configurable weight tables
   - Support dynamic weight adjustment based on context

### Architecture Changes

1. **Module Structure**
   - Add `src/complexity/token_classifier.rs` for classification logic
   - Extend `src/complexity/entropy.rs` with weighted calculations
   - Update `src/config.rs` with classification configuration
   - Add classification tests in `tests/token_classification_tests.rs`

2. **Data Flow**
   - AST → Token Extraction → Classification → Weighted Entropy → Dampening
   - Classification results cached in `EntropyAnalyzer`
   - Pattern detection results influence dampening factors
   - Final scores incorporate classification-based adjustments

### Data Structures

```rust
pub struct ClassifiedToken {
    pub class: TokenClass,
    pub raw_token: String,
    pub context: TokenContext,
    pub weight: f64,
}

pub struct TokenContext {
    pub is_method_call: bool,
    pub is_field_access: bool,
    pub is_external: bool,
    pub scope_depth: usize,
    pub parent_node_type: NodeType,
}

pub struct ClassificationConfig {
    pub enabled: bool,
    pub weights: HashMap<TokenClass, f64>,
    pub patterns: Vec<PatternRule>,
    pub cache_size: usize,
}
```

### APIs and Interfaces

```rust
pub trait TokenClassifier {
    fn classify(&self, token: &str, context: &TokenContext) -> TokenClass;
    fn get_weight(&self, class: &TokenClass) -> f64;
    fn update_weights(&mut self, weights: HashMap<TokenClass, f64>);
}

impl EntropyAnalyzer {
    pub fn calculate_entropy_with_classification(
        &self,
        block: &Block,
        classifier: &impl TokenClassifier,
    ) -> EntropyScore;
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 52: Basic entropy infrastructure must exist
  - Spec 53: Complete entropy implementation with caching
- **Affected Components**:
  - `src/complexity/entropy.rs` - Core entropy calculations
  - `src/analyzers/rust.rs` - Rust-specific token extraction
  - `src/analyzers/javascript/entropy.rs` - JS/TS token extraction
  - `src/config.rs` - Configuration structures
- **External Dependencies**: None required

## Testing Strategy

- **Unit Tests**:
  - Test each token classification category
  - Verify weight application in entropy calculation
  - Test pattern detection accuracy
  - Validate caching behavior

- **Integration Tests**:
  - Test real code samples with known complexity patterns
  - Verify false positive reduction on validation functions
  - Test preservation of genuine complexity
  - Validate configuration integration

- **Performance Tests**:
  - Benchmark classification overhead on large files
  - Test cache effectiveness with repeated analysis
  - Measure memory usage with classification enabled
  - Compare entropy calculation times before/after

- **User Acceptance**:
  - Analyze real-world codebases for accuracy improvements
  - Gather feedback on classification tuning
  - Validate false positive reduction claims
  - Test with different programming patterns

## Documentation Requirements

- **Code Documentation**:
  - Document each `TokenClass` variant with examples
  - Explain weight calculation rationale
  - Provide classification algorithm details
  - Include performance considerations

- **User Documentation**:
  - Add classification tuning guide to docs/entropy.md
  - Provide examples of classification in action
  - Document configuration options
  - Include troubleshooting section

- **Architecture Updates**:
  - Update ARCHITECTURE.md with token classification flow
  - Document caching strategy for classification
  - Explain integration with entropy system

## Implementation Notes

1. **Pattern Recognition Priority**:
   - Start with most common patterns (getters/setters)
   - Add validation and I/O patterns next
   - Implement external API detection last
   - Use iterative refinement based on test results

2. **Performance Optimizations**:
   - Use string interning for common tokens
   - Implement lazy classification for large blocks
   - Cache classification results aggressively
   - Consider bloom filters for pattern matching

3. **Backward Compatibility**:
   - Keep old normalization as fallback option
   - Allow gradual migration via configuration
   - Preserve existing test expectations initially
   - Provide migration tool for configuration

4. **Tuning Considerations**:
   - Start with conservative weights
   - Provide preset weight profiles (strict, balanced, permissive)
   - Log classification decisions in verbose mode
   - Support A/B testing of classification strategies

## Migration and Compatibility

1. **Configuration Migration**:
   - Add `classification` section to `EntropyConfig`
   - Default to disabled for existing projects
   - Provide migration command to enable with defaults
   - Document breaking changes in CHANGELOG

2. **API Compatibility**:
   - Keep existing `calculate_entropy` method signature
   - Add new methods for classification-aware calculation
   - Deprecate old token normalization gradually
   - Maintain backward compatibility for 2 versions

3. **Testing Migration**:
   - Update test expectations incrementally
   - Add feature flag for new classification tests
   - Maintain parallel test suites during transition
   - Document test migration process

4. **User Impact**:
   - Entropy scores will change (improve) when enabled
   - Some previously dampened code may show higher complexity
   - Some false positives will disappear
   - Provide before/after comparison tool