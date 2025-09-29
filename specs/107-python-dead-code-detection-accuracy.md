---
number: 107
title: Python Dead Code Detection Accuracy
category: optimization
priority: critical
status: draft
dependencies: [103, 104, 105, 106]
created: 2025-09-29
---

# Specification 107: Python Dead Code Detection Accuracy

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: [103 - Framework Patterns, 104 - Test Detection, 105 - Callback Tracking, 106 - Import Resolution]

## Context

The current Python dead code detection produces numerous false positives due to incomplete understanding of Python's dynamic features and framework patterns. This undermines user trust and makes the tool less useful for Python projects.

Current false positive sources:
- Framework entry points marked as dead code
- Event handlers with no static callers flagged incorrectly
- Test fixtures and helpers marked as unused
- Callback functions appearing as dead
- Dynamically called functions not recognized
- Property getters/setters marked as unused
- Magic methods not properly tracked
- Plugin system entry points missed

Impact:
- Users ignore dead code warnings
- Reduced confidence in analysis results
- Manual verification required for each warning
- Risk of removing actually-used code

## Objective

Implement a confidence-based dead code detection system for Python that dramatically reduces false positives by leveraging framework detection, test patterns, callback tracking, and import resolution to provide accurate, actionable dead code identification with confidence scores.

## Requirements

### Functional Requirements

- Integrate all detection improvements:
  - Framework entry point detection (Spec 103)
  - Test pattern recognition (Spec 104)
  - Callback tracking (Spec 105)
  - Import resolution (Spec 106)
- Implement confidence scoring system:
  - High confidence: No callers, not entry point, not test
  - Medium confidence: Dynamic patterns possible
  - Low confidence: Framework code, callbacks likely
- Consider multiple factors:
  - Static call graph
  - Framework patterns
  - Test coverage data
  - Export status (`__all__`, public API)
  - Decorator patterns
  - Name conventions
- Provide detailed explanations for dead code determination
- Support suppression comments (`# debtmap: not-dead`)
- Generate actionable removal suggestions

### Non-Functional Requirements

- False positive rate < 10% for framework code
- Clear confidence scoring explanation
- Fast incremental analysis
- Configurable confidence thresholds
- Comprehensive test coverage

## Acceptance Criteria

- [ ] Framework entry points never marked as dead code
- [ ] Event handlers correctly identified as used
- [ ] Test functions and fixtures not flagged as dead
- [ ] Callbacks tracked and not marked as dead
- [ ] Confidence scores provided for all dead code items
- [ ] False positive rate < 10% on real projects
- [ ] Detailed explanations for each detection
- [ ] Suppression comments respected
- [ ] Documentation includes confidence interpretation

## Technical Details

### Implementation Approach

1. Create `DeadCodeAnalyzer` integrating all detection systems
2. Implement multi-factor confidence scoring
3. Add explanation generation for decisions
4. Integrate with coverage data for validation
5. Support configuration and suppression

### Architecture Changes

```rust
// src/analysis/python_dead_code_enhanced.rs
pub struct EnhancedDeadCodeAnalyzer {
    framework_detector: FrameworkPatternRegistry,
    test_detector: PythonTestDetector,
    callback_tracker: CallbackTracker,
    import_resolver: EnhancedImportResolver,
    coverage_data: Option<CoverageData>,
}

pub struct DeadCodeResult {
    function_id: FunctionId,
    confidence: DeadCodeConfidence,
    reasons: Vec<DeadCodeReason>,
    suggestion: RemovalSuggestion,
}

pub enum DeadCodeConfidence {
    High(f32),     // 0.8-1.0: Very likely dead
    Medium(f32),   // 0.5-0.8: Possibly dead
    Low(f32),      // 0.0-0.5: Unlikely dead
}

pub enum DeadCodeReason {
    NoStaticCallers,
    NoCoverage,
    NotExported,
    PrivateFunction,
    NotInTestFile,
    NoFrameworkPattern,
}

pub struct RemovalSuggestion {
    can_remove: bool,
    safe_to_remove: bool,
    explanation: String,
    risks: Vec<String>,
}
```

### Data Structures

- `ConfidenceFactors`: Weights for different signals
- `DeadCodeContext`: Analysis context for function
- `SuppressionRule`: User-defined suppression

### APIs and Interfaces

```rust
impl EnhancedDeadCodeAnalyzer {
    pub fn analyze_function(&self, func: &FunctionMetrics) -> DeadCodeResult;
    pub fn calculate_confidence(&self, factors: &ConfidenceFactors) -> DeadCodeConfidence;
    pub fn generate_explanation(&self, result: &DeadCodeResult) -> String;
    pub fn should_suppress(&self, func: &FunctionMetrics) -> bool;
}
```

## Dependencies

- **Prerequisites**:
  - [103 - Framework Pattern Detection]
  - [104 - Test Detection Enhancement]
  - [105 - Callback Tracking]
  - [106 - Import Resolution]
- **Affected Components**:
  - `src/analysis/python_dead_code.rs`
  - `src/priority/scoring.rs`
  - Report generation modules
- **External Dependencies**: Coverage data integration

## Testing Strategy

- **Unit Tests**: Each confidence factor
- **Integration Tests**: Full dead code analysis
- **Accuracy Tests**: False positive/negative rates
- **Framework Tests**: Major framework patterns
- **Regression Tests**: Known false positive cases

## Documentation Requirements

- **Code Documentation**: Confidence calculation algorithm
- **User Documentation**:
  - Interpreting confidence scores
  - Suppression comment syntax
  - Common false positive patterns
- **Migration Guide**: Changes from previous detection
- **Examples**: Dead code analysis scenarios

## Implementation Notes

- Start with conservative confidence (prefer false negatives)
- Allow user calibration of confidence thresholds
- Log detailed decision process for debugging
- Consider project type (web, GUI, CLI) for defaults
- Cache analysis results for performance
- Provide batch suppression for similar patterns

## Migration and Compatibility

- Backward compatible with existing dead code detection
- Confidence scores added to existing output
- Gradual rollout with feature flag
- Existing suppressions remain valid
- Clear migration path for users