---
number: 72
title: Python Organization Anti-Pattern Detection
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-09-01
---

# Specification 72: Python Organization Anti-Pattern Detection

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The Rust analyzer includes comprehensive organization anti-pattern detection including God objects, feature envy, magic values, long parameter lists, primitive obsession, and data clumps. These patterns are critical for identifying maintainability issues and architectural problems. The Python analyzer completely lacks these capabilities, significantly limiting its ability to identify structural code quality issues.

Organization anti-patterns indicate:
- Poor separation of concerns
- Tight coupling between components
- Maintenance difficulties
- Refactoring opportunities

Python's object-oriented features and dynamic typing make these patterns particularly important to detect.

## Objective

Implement comprehensive organization anti-pattern detection for Python code, matching or exceeding the Rust analyzer's capabilities in identifying God objects, feature envy, magic values, long parameter lists, primitive obsession, and data clumps.

## Requirements

### Functional Requirements
- Detect God objects (classes with too many responsibilities)
- Identify feature envy (methods using other classes excessively)
- Find magic values (hardcoded literals used repeatedly)
- Detect long parameter lists
- Identify primitive obsession patterns
- Find data clumps (parameters that travel together)
- Provide refactoring suggestions for each pattern

### Non-Functional Requirements
- Configurable thresholds for each pattern
- Minimal false positive rate (< 15%)
- Efficient analysis without full program analysis
- Language-idiomatic detection rules

## Acceptance Criteria

- [ ] PythonOrganizationDetector trait implementation
- [ ] God object detection with method/attribute counting
- [ ] Feature envy detection with call analysis
- [ ] Magic value detection across module
- [ ] Long parameter list identification
- [ ] Primitive obsession pattern matching
- [ ] Data clump detection algorithm
- [ ] Refactoring suggestions for each pattern
- [ ] Configurable thresholds via config
- [ ] Unit tests for each detector
- [ ] Integration with Python analyzer

## Technical Details

### Implementation Approach
1. Create `organization::python` module
2. Implement detectors for each pattern type
3. Port existing Rust detectors to Python AST
4. Add Python-specific pattern detection
5. Integrate with debt item generation

### Architecture Changes
- New module: `src/organization/python/`
- Individual detector modules for each pattern
- Integration with Python analyzer pipeline

### Data Structures
```rust
pub trait PythonOrganizationDetector {
    fn detect_anti_patterns(&self, module: &ast::Mod) -> Vec<OrganizationAntiPattern>;
    fn estimate_maintainability_impact(&self, pattern: &OrganizationAntiPattern) -> MaintainabilityImpact;
}

pub struct PythonGodObjectDetector {
    method_threshold: usize,
    attribute_threshold: usize,
    complexity_threshold: u32,
}

pub struct PythonFeatureEnvyDetector {
    external_call_ratio: f32,
    min_calls_threshold: usize,
}

pub struct PythonMagicValueDetector {
    min_occurrences: usize,
    exclude_patterns: Vec<String>,
}
```

### APIs and Interfaces
- Detector trait implementations for each pattern
- Integration point in `analyze_python_file`
- Configuration through existing config system

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - `src/analyzers/python.rs`
  - `src/organization/mod.rs`
  - `src/core/debt.rs`
- **External Dependencies**: rustpython_parser (existing)

## Testing Strategy

- **Unit Tests**: Each detector with known patterns
- **Integration Tests**: Full module analysis
- **False Positive Tests**: Common valid patterns
- **Threshold Tests**: Configurable threshold validation

## Documentation Requirements

- **Code Documentation**: Detection algorithms and thresholds
- **User Documentation**: Pattern descriptions and remediation
- **Examples**: Before/after refactoring examples

## Implementation Notes

- Consider Python naming conventions (PEP 8)
- Handle dynamic attribute access appropriately
- Account for Python metaclasses and descriptors
- Special handling for Django/Flask patterns
- Consider dataclasses and named tuples

## Migration and Compatibility

During prototype phase: New feature addition with no breaking changes. Organization patterns will be added to debt items without affecting existing analysis.