---
number: 78
title: Python Pattern-Based Complexity Adjustments
category: optimization
priority: low
status: draft
dependencies: [76]
created: 2025-09-01
---

# Specification 78: Python Pattern-Based Complexity Adjustments

**Category**: optimization
**Priority**: low
**Status**: draft
**Dependencies**: [76]

## Context

The Rust analyzer includes sophisticated pattern-based complexity adjustments, including visitor pattern detection that reduces complexity scores for recognized patterns. This prevents false positives where legitimate design patterns would otherwise trigger high complexity warnings. The Python analyzer lacks these adjustments, potentially flagging well-structured code using common Python patterns.

Common Python patterns that warrant adjustment:
- Dictionary dispatch tables (switch statement alternative)
- Chain of responsibility patterns
- Strategy pattern implementations
- Visitor patterns (less common but present)
- Decorator chains
- Context manager patterns

## Objective

Implement pattern-based complexity adjustments for Python that recognize common design patterns and idiomatic Python constructs, reducing false positives in complexity analysis while maintaining detection of genuinely complex code.

## Requirements

### Functional Requirements
- Detect dictionary dispatch patterns
- Recognize strategy pattern implementations
- Identify chain of responsibility patterns
- Detect visitor patterns in Python
- Recognize decorator composition patterns
- Identify context manager patterns
- Apply appropriate complexity reductions
- Provide pattern detection feedback

### Non-Functional Requirements
- Accurate pattern recognition (>85% precision)
- Minimal performance impact
- Configurable adjustment factors
- Extensible pattern definitions

## Acceptance Criteria

- [ ] Pattern detector implementation for Python
- [ ] Dictionary dispatch recognition
- [ ] Strategy pattern detection
- [ ] Chain of responsibility detection
- [ ] Visitor pattern adaptation for Python
- [ ] Decorator pattern analysis
- [ ] Complexity adjustment application
- [ ] Pattern detection reporting
- [ ] Configuration for adjustments
- [ ] Unit tests for each pattern
- [ ] Integration with complexity calculation

## Technical Details

### Implementation Approach
1. Create Python pattern detector module
2. Implement pattern matching algorithms
3. Define adjustment factors for each pattern
4. Integrate with complexity calculation
5. Add pattern reporting to output

### Architecture Changes
- New module: `src/complexity/python_pattern_adjustments.rs`
- Integration with cognitive/cyclomatic calculation
- Pattern detection pipeline

### Data Structures
```rust
pub struct PythonPatternDetector {
    patterns: Vec<Box<dyn PythonPattern>>,
    adjustment_config: AdjustmentConfig,
}

pub trait PythonPattern {
    fn detect(&self, func_def: &ast::StmtFunctionDef) -> Option<PatternMatch>;
    fn adjustment_factor(&self) -> f32;
}

pub struct DictionaryDispatchPattern {
    min_branches: usize,
    key_patterns: Vec<String>,
}

pub struct StrategyPattern {
    method_signatures: Vec<String>,
    class_patterns: Vec<String>,
}

pub struct PatternMatch {
    pub pattern_type: PatternType,
    pub confidence: f32,
    pub adjustment: f32,
}
```

### APIs and Interfaces
- `detect_patterns(func_def: &ast::StmtFunctionDef) -> Vec<PatternMatch>`
- `apply_adjustments(base_complexity: u32, patterns: &[PatternMatch]) -> u32`
- Pattern configuration API

## Dependencies

- **Prerequisites**: [76] - Enhanced Complexity Thresholds
- **Affected Components**: 
  - `src/complexity/python_patterns.rs`
  - `src/analyzers/python.rs`
  - `src/complexity/mod.rs`
- **External Dependencies**: rustpython_parser (existing)

## Testing Strategy

- **Unit Tests**: Individual pattern detection
- **Adjustment Tests**: Complexity reduction validation
- **False Positive Tests**: Non-pattern similar code
- **Integration Tests**: Full complexity analysis

## Documentation Requirements

- **Code Documentation**: Pattern detection algorithms
- **User Documentation**: Recognized patterns
- **Examples**: Before/after adjustment examples

## Implementation Notes

- Focus on idiomatic Python patterns
- Consider PEP 8 and common style guides
- Handle async patterns appropriately
- Account for type hints in pattern detection
- Consider framework-specific patterns
- Balance between false positives and negatives

## Migration and Compatibility

During prototype phase: Enhancement to complexity calculation. Adjustments will be applied automatically without breaking changes. Can be disabled via configuration if needed.