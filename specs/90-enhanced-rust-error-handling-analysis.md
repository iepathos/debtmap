---
number: 90
title: Enhanced Rust Error Handling Analysis
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-09-06
---

# Specification 90: Enhanced Rust Error Handling Analysis

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current Rust error handling analyzer (`src/debt/error_swallowing.rs`) focuses primarily on Result type handling patterns such as `if let Ok()` without else branches, `.ok()` discarding error information, and match expressions that ignore Err variants. However, it misses several critical error handling anti-patterns that can lead to production failures, poor debuggability, and reliability issues.

Analysis of the codebase reveals over 108 instances of potentially problematic patterns like `.unwrap()` and `.expect()` that could cause panics in production code. Additionally, the analyzer doesn't detect error context loss, async-specific error patterns, or provide quality assessments of error propagation strategies.

Key gaps in current implementation:
- No detection of panic-inducing patterns (unwrap, expect, panic!)
- Missing analysis of error context preservation
- No async/await specific error handling patterns
- Lack of error propagation quality assessment
- No detection of error type erasure patterns

## Objective

Enhance the Rust error handling analysis to comprehensively detect panic-inducing patterns, error context loss, async-specific issues, and error propagation quality problems, providing actionable suggestions for improving error handling reliability and debuggability.

## Requirements

### Functional Requirements
- Detect panic-inducing patterns (unwrap, expect, panic!, unreachable!)
- Identify error context loss (map_err discarding original, missing anyhow context)
- Find async-specific error patterns (dropped futures, unhandled join handles)
- Assess error propagation quality (context preservation, type erasure)
- Detect overly broad error handling (Box<dyn Error>, catch-all conversions)
- Support configurable severity levels based on code location (lib vs bin vs test)
- Provide pattern-specific remediation suggestions
- Integrate with existing suppression comment system

### Non-Functional Requirements
- Minimal performance impact on analysis
- Low false positive rate through context-aware detection
- Configurable thresholds and allow-listing
- Clear, actionable error messages
- Efficient AST traversal without redundant passes

## Acceptance Criteria

- [ ] PanicPatternDetector implementation with unwrap/expect detection
- [ ] ContextLossAnalyzer for error transformation tracking
- [ ] AsyncErrorDetector for Future/Task pattern analysis
- [ ] ErrorPropagationAnalyzer for quality assessment
- [ ] Integration with existing ErrorSwallowingDetector
- [ ] Severity scoring based on location and pattern type
- [ ] Unit tests for each new pattern detector
- [ ] Performance benchmarks showing < 10% analysis overhead
- [ ] Documentation of all detected patterns
- [ ] Configuration schema for pattern enablement
- [ ] Remediation suggestions for each pattern type
- [ ] Integration tests with real-world code examples

## Technical Details

### Implementation Approach
1. Extend existing `ErrorSwallowingDetector` with new pattern types
2. Create specialized detectors for each pattern category
3. Implement visitor pattern for efficient AST traversal
4. Build context-aware analysis tracking error transformations
5. Add severity scoring engine with configurable weights
6. Generate pattern-specific remediation suggestions

### Architecture Changes
- New modules in `src/debt/`:
  - `panic_patterns.rs` - Panic-inducing pattern detection
  - `error_context.rs` - Context preservation analysis
  - `async_errors.rs` - Async-specific error patterns
  - `error_propagation.rs` - Propagation quality assessment
- Extended `ErrorSwallowingPattern` enum with new variants
- New `ErrorHandlingConfig` for pattern configuration

### Data Structures
```rust
pub enum EnhancedErrorPattern {
    // Existing patterns
    IfLetOkNoElse,
    LetUnderscoreResult,
    // New panic patterns
    UnwrapOnResult,
    UnwrapOnOption,
    ExpectWithGenericMessage,
    PanicInNonTest,
    UnreachableInReachable,
    TodoInProduction,
    // Context loss patterns
    MapErrDiscardingOriginal,
    AnyhowWithoutContext,
    QuestionMarkChain,
    StringErrorConversion,
    // Async patterns
    DroppedFuture,
    UnhandledJoinHandle,
    SilentTaskPanic,
    SelectBranchIgnored,
    // Type erasure
    BoxDynError,
    OverlyBroadConversion,
}

pub struct ErrorHandlingConfig {
    pub panic_patterns_enabled: bool,
    pub context_analysis_enabled: bool,
    pub async_patterns_enabled: bool,
    pub allow_unwrap_in_tests: bool,
    pub generic_expect_threshold: usize,
    pub context_chain_limit: usize,
}

pub struct ErrorQualityMetrics {
    pub pattern: EnhancedErrorPattern,
    pub severity: Severity,
    pub location: CodeLocation,
    pub propagation_quality: PropagationQuality,
    pub suggested_fix: String,
}

pub enum PropagationQuality {
    ProperWithContext,
    PassthroughNoContext,
    TypeErasure,
    Swallowed,
}

pub enum CodeLocation {
    Library,
    Binary,
    Test,
    Example,
    Benchmark,
}
```

### APIs and Interfaces
- `detect_enhanced_error_patterns(file: &File, config: &ErrorHandlingConfig) -> Vec<DebtItem>`
- `assess_error_propagation_quality(expr: &Expr) -> PropagationQuality`
- `suggest_error_handling_improvement(pattern: &EnhancedErrorPattern) -> String`
- `calculate_pattern_severity(pattern: &EnhancedErrorPattern, location: &CodeLocation) -> Severity`

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - `src/debt/error_swallowing.rs` - Extend existing detector
  - `src/analyzers/rust.rs` - Integration point
  - `src/core/debt.rs` - New debt type variants
  - `src/debt/mod.rs` - Module registration
- **External Dependencies**: 
  - syn (existing) - AST parsing
  - proc-macro2 (existing) - Span information

## Testing Strategy

- **Unit Tests**: 
  - Each pattern detector with positive/negative cases
  - Severity calculation logic
  - Remediation suggestion generation
  - Configuration parsing and application
- **Integration Tests**: 
  - Complete file analysis with multiple patterns
  - Suppression comment interaction
  - Performance benchmarks with large files
- **Pattern Tests**: 
  - Real-world examples from popular Rust crates
  - Edge cases and corner patterns
  - False positive validation
- **Framework Tests**: 
  - Tokio-specific async patterns
  - Anyhow/thiserror error handling
  - Standard library error traits

## Documentation Requirements

- **Code Documentation**: 
  - Detection algorithm for each pattern
  - Severity scoring rationale
  - AST traversal optimization notes
- **User Documentation**: 
  - Comprehensive pattern catalog with examples
  - Configuration guide with recommended settings
  - Best practices for Rust error handling
- **Architecture Updates**: 
  - Error handling analysis flow diagram
  - Integration with existing debt detection

## Implementation Notes

- Consider crate-specific patterns (tokio::spawn, anyhow::Context)
- Handle macro-generated code appropriately
- Account for conditional compilation (#[cfg(test)])
- Special handling for generated code (derive macros)
- Consider interaction with existing test detection
- Optimize for incremental analysis in watch mode
- Support for custom error types and traits
- Integration with IDE diagnostics format

## Migration and Compatibility

During prototype phase: This is a pure enhancement with no breaking changes. New patterns will be added alongside existing error swallowing detection. Configuration allows gradual adoption of new detections. Existing debt items remain unchanged, new patterns create additional debt items with distinct IDs.