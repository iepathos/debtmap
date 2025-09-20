---
number: 103
title: Python-Specific Complexity Patterns
category: compatibility
priority: high
status: draft
dependencies: [76, 77, 78]
created: 2025-01-20
---

# Specification 103: Python-Specific Complexity Patterns

**Category**: compatibility
**Priority**: high
**Status**: draft
**Dependencies**: [76-python-enhanced-complexity-thresholds, 77-python-enhanced-call-graph, 78-python-pattern-based-adjustments]

## Context

The current debtmap implementation uses language-agnostic complexity metrics that don't accurately capture Python-specific programming patterns and idioms. Python's dynamic nature, extensive use of special methods, and unique constructs like generators, decorators, and context managers require specialized analysis to accurately assess code complexity and technical debt.

Analysis of real-world Python codebases (such as promptconstruct-frontend) reveals that the tool misses critical complexity indicators specific to Python:
- Event-driven patterns in frameworks like wxPython, Django, Flask
- Generator functions and comprehensions that hide complexity
- Context managers and decorators that add layers of indirection
- Metaclass usage and multiple inheritance complexity
- Dynamic attribute access and reflection patterns

## Objective

Enhance debtmap's Python analyzer to recognize and properly weight Python-specific complexity patterns, providing more accurate complexity metrics and debt assessment for Python codebases.

## Requirements

### Functional Requirements

1. **Generator and Comprehension Complexity**
   - Detect generator functions (`yield` statements)
   - Analyze list/dict/set comprehensions for nested complexity
   - Weight generator expressions based on nesting depth
   - Account for async generators (`async for`, `async with`)

2. **Decorator Pattern Analysis**
   - Track decorator stack depth
   - Analyze decorator factories (decorators that return decorators)
   - Weight complexity for property decorators (`@property`, `@setter`, `@deleter`)
   - Detect class decorators and their impact on complexity

3. **Context Manager Complexity**
   - Identify `__enter__` and `__exit__` implementations
   - Analyze `with` statement nesting
   - Track context manager composition
   - Weight async context managers appropriately

4. **Event-Driven Pattern Detection**
   - Recognize common event handler patterns (on_*, handle_*, process_*)
   - Detect framework-specific patterns:
     - wxPython: EVT_* bindings, event propagation
     - Django: signal handlers, middleware
     - Flask: route decorators, before/after handlers
   - Weight event chain complexity

5. **Metaclass and Inheritance Complexity**
   - Detect metaclass usage and custom `__new__`/`__init__` patterns
   - Analyze multiple inheritance resolution order (MRO)
   - Weight diamond inheritance patterns
   - Track mixin usage and composition

6. **Dynamic Python Features**
   - Detect `getattr`/`setattr`/`hasattr` usage
   - Track `__getattribute__` and descriptor protocol usage
   - Analyze `exec`/`eval` usage (high complexity weight)
   - Monitor monkey patching patterns

### Non-Functional Requirements

- Performance impact: <5% increase in analysis time
- Memory usage: <10MB additional for pattern detection
- Backward compatibility with existing Python analysis
- Clear documentation of Python-specific weights

## Acceptance Criteria

- [ ] Generator functions add +2 cognitive complexity per yield statement
- [ ] Nested comprehensions properly accumulate complexity (exponential for deep nesting)
- [ ] Decorator stacks add +1 complexity per decorator beyond the first
- [ ] Context managers add +1 complexity, +2 for nested contexts
- [ ] Event handlers are weighted based on framework patterns
- [ ] Metaclass usage adds +5 base complexity to affected classes
- [ ] Multiple inheritance adds +3 complexity per additional base class
- [ ] Dynamic attribute access adds +2 complexity per usage
- [ ] Framework-specific patterns are correctly identified and weighted
- [ ] All Python-specific metrics are reported separately in analysis output

## Technical Details

### Implementation Approach

1. Extend `PythonAnalyzer` with pattern recognition modules
2. Create `PythonComplexityWeights` configuration structure
3. Implement pattern matchers for each Python construct
4. Add framework detection heuristics
5. Integrate with existing cognitive complexity calculations

### Architecture Changes

```rust
pub struct PythonComplexityPatterns {
    pub generator_weight: f64,
    pub comprehension_depth_multiplier: f64,
    pub decorator_stack_weight: f64,
    pub context_manager_weight: f64,
    pub metaclass_weight: f64,
    pub multiple_inheritance_weight: f64,
    pub dynamic_access_weight: f64,
    pub event_handler_weight: f64,
}

pub trait PythonPatternDetector {
    fn detect_generators(&self, ast: &PythonAst) -> Vec<GeneratorPattern>;
    fn detect_decorators(&self, ast: &PythonAst) -> Vec<DecoratorPattern>;
    fn detect_event_handlers(&self, ast: &PythonAst) -> Vec<EventHandlerPattern>;
    fn detect_metaclasses(&self, ast: &PythonAst) -> Vec<MetaclassPattern>;
}
```

### Data Structures

- `GeneratorPattern`: Tracks yield statements and async generators
- `DecoratorPattern`: Decorator stack information
- `EventHandlerPattern`: Framework-specific event handling
- `MetaclassPattern`: Metaclass and MRO complexity
- `DynamicAccessPattern`: Dynamic attribute access tracking

### APIs and Interfaces

- Extend `FileMetrics` with `python_patterns: Option<PythonPatterns>`
- Add `--python-weights` CLI option for customizing weights
- Export Python-specific metrics in JSON output

## Dependencies

- **Prerequisites**: Specs 76, 77, 78 (Python enhancement foundations)
- **Affected Components**:
  - `analyzers/python.rs`
  - `complexity/cognitive.rs`
  - `debt/patterns.rs`
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Pattern detection for each Python construct
- **Integration Tests**: Real Python codebases with known patterns
- **Performance Tests**: Benchmark against large Python projects
- **User Acceptance**: Validate with wxPython, Django, Flask projects

## Documentation Requirements

- **Code Documentation**: Document each pattern weight and rationale
- **User Documentation**: Guide for Python-specific metrics interpretation
- **Architecture Updates**: Update ARCHITECTURE.md with Python pattern detection flow

## Implementation Notes

- Start with most common patterns (generators, decorators)
- Use visitor pattern for AST traversal efficiency
- Cache framework detection results per project
- Consider making weights configurable via .debtmap.toml

## Migration and Compatibility

During prototype phase: Breaking changes allowed. Existing Python analysis will be enhanced with additional metrics. Output format will change to include Python-specific pattern information.