---
number: 104
title: Python-Aware Dead Code Detection
category: foundation
priority: critical
status: draft
dependencies: [77]
created: 2025-01-20
---

# Specification 104: Python-Aware Dead Code Detection

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: [77-python-enhanced-call-graph]

## Context

The current dead code detection in debtmap incorrectly flags Python magic methods and framework-specific methods as unused. This creates significant false positives that undermine the tool's credibility for Python projects.

Analysis of the promptconstruct-frontend project revealed critical false positives:
- `__init__` methods flagged as dead code despite being constructors
- `OnInit` method in wxPython apps marked as unused
- Event handlers bound dynamically flagged as having no callers
- Framework lifecycle methods incorrectly identified as dead code

Python's dynamic nature, magic methods, and framework conventions require specialized understanding to accurately detect truly dead code versus code that's called implicitly by the runtime or framework.

## Objective

Implement Python-aware dead code detection that correctly identifies implicitly called methods, framework patterns, and Python runtime conventions, eliminating false positives while accurately detecting genuinely unused code.

## Requirements

### Functional Requirements

1. **Magic Method Recognition**
   - Never flag Python magic methods as dead code:
     - Object lifecycle: `__init__`, `__new__`, `__del__`
     - Operator overloading: `__add__`, `__sub__`, `__mul__`, etc.
     - Container protocols: `__len__`, `__getitem__`, `__setitem__`
     - Context managers: `__enter__`, `__exit__`
     - Descriptors: `__get__`, `__set__`, `__delete__`
     - Serialization: `__getstate__`, `__setstate__`, `__reduce__`
     - String representation: `__str__`, `__repr__`, `__format__`
     - Comparison: `__eq__`, `__lt__`, `__hash__`
     - Attribute access: `__getattr__`, `__setattr__`, `__delattr__`
     - Callable: `__call__`
     - Async/await: `__aiter__`, `__anext__`, `__await__`

2. **Framework Pattern Detection**
   - Recognize framework-specific lifecycle methods:
     - **wxPython**: `OnInit`, `OnExit`, `on_*` event handlers
     - **Django**: `get_*`, `post_*`, `save`, `clean`, `full_clean`
     - **Flask**: Route-decorated functions, `before_request`, `after_request`
     - **pytest**: `test_*`, `setup_*`, `teardown_*`, fixtures
     - **unittest**: `setUp`, `tearDown`, `test*` methods
     - **FastAPI**: Route handlers, dependency functions
     - **SQLAlchemy**: Model methods, event listeners

3. **Dynamic Binding Detection**
   - Track event binding patterns:
     - `self.Bind(event, handler)`
     - `signal.connect(handler)`
     - `@app.route()` decorators
     - `getattr(obj, method_name)` patterns
   - Analyze string-based method references
   - Detect reflection and introspection usage

4. **Decorator-Based Usage**
   - Recognize decorated functions as potentially used:
     - Property decorators (`@property`, `@cached_property`)
     - Class/static methods (`@classmethod`, `@staticmethod`)
     - Framework decorators (routes, signals, tasks)
     - Testing decorators (`@pytest.fixture`, `@mock.patch`)

5. **Module-Level Conventions**
   - Respect Python module conventions:
     - `if __name__ == "__main__":` blocks
     - Module-level `__all__` exports
     - Plugin/extension entry points
     - Setup.py entry points

6. **Class Inheritance Analysis**
   - Track method usage through inheritance:
     - Abstract method implementations
     - Mixin method usage
     - Protocol/interface implementations
     - Method resolution order (MRO) tracking

### Non-Functional Requirements

- Zero false positives for standard Python patterns
- Maintain performance with <10% overhead
- Configurable framework detection
- Clear reporting of why methods aren't dead

## Acceptance Criteria

- [ ] All Python magic methods are never flagged as dead code
- [ ] wxPython app lifecycle methods are correctly recognized
- [ ] Django model and view methods are properly detected
- [ ] Flask route handlers are identified as used
- [ ] Event-bound methods are tracked correctly
- [ ] Decorated functions are analyzed for implicit usage
- [ ] Test methods are recognized by naming convention
- [ ] Framework detection is configurable via config file
- [ ] Dead code report includes "safe from removal" confidence level
- [ ] Performance regression <10% on large codebases

## Technical Details

### Implementation Approach

1. Create comprehensive magic method allowlist
2. Build framework pattern recognition engine
3. Implement dynamic binding analysis
4. Add decorator usage tracking
5. Enhance call graph with implicit calls

### Architecture Changes

```rust
pub struct PythonDeadCodeDetector {
    magic_methods: HashSet<String>,
    framework_patterns: HashMap<String, FrameworkPattern>,
    decorator_handlers: Vec<DecoratorHandler>,
}

pub struct FrameworkPattern {
    pub name: String,
    pub lifecycle_methods: Vec<String>,
    pub event_patterns: Vec<Regex>,
    pub decorator_patterns: Vec<String>,
}

pub trait ImplicitCallDetector {
    fn is_implicitly_called(&self, method: &str, context: &Context) -> bool;
    fn get_implicit_callers(&self, method: &str) -> Vec<String>;
    fn confidence_level(&self, method: &str) -> RemovalConfidence;
}

pub enum RemovalConfidence {
    Safe,        // Definitely unused
    Likely,      // Probably unused, manual check recommended
    Unsafe,      // May be implicitly called
    Framework,   // Framework method, don't remove
    Magic,       // Python magic method, never remove
}
```

### Data Structures

- `MagicMethodRegistry`: Comprehensive list of Python magic methods
- `FrameworkRegistry`: Configurable framework patterns
- `DynamicBindingTracker`: Tracks runtime method bindings
- `ImplicitCallGraph`: Augments regular call graph with implicit calls

### APIs and Interfaces

- `--python-frameworks`: Specify active frameworks for better detection
- `--strict-dead-code`: Only flag with high confidence
- Export dead code confidence levels in JSON output

## Dependencies

- **Prerequisites**: Spec 77 (Python enhanced call graph)
- **Affected Components**:
  - `analyzers/python.rs`
  - `analysis/call_graph.rs`
  - `debt/dead_code.rs`
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Each magic method category
- **Integration Tests**: Real framework codebases
- **Performance Tests**: Large Python projects
- **User Acceptance**: Zero false positives on major frameworks

## Documentation Requirements

- **Code Documentation**: Document each magic method category
- **User Documentation**: Guide for Python dead code interpretation
- **Architecture Updates**: Document implicit call detection flow

## Implementation Notes

- Start with comprehensive magic method list from Python docs
- Use configuration file for framework patterns
- Consider creating framework detection heuristics
- Add "why not dead" explanation to output

## Migration and Compatibility

During prototype phase: Breaking changes allowed. Dead code detection output will change format to include confidence levels and reasoning. Existing Python analysis will be significantly more accurate with fewer false positives.