---
number: 105
title: Python Dynamic Dispatch and Callback Tracking
category: foundation
priority: high
status: draft
dependencies: [103]
created: 2025-09-29
---

# Specification 105: Python Dynamic Dispatch and Callback Tracking

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: [103 - Python Framework Pattern Detection]

## Context

Python's dynamic nature allows functions and methods to be passed as callbacks, stored in data structures, and invoked indirectly. The current analyzer fails to track these dynamic relationships, resulting in incomplete call graphs and false dead code detection.

Current limitations:
- Method references (`self.method`) passed as callbacks not tracked
- Event binding patterns (`button.Bind(wx.EVT_BUTTON, self.on_click)`) not resolved
- Signal/slot connections not recognized
- Decorator-based callbacks (`@app.route`) partially supported
- Lambda and partial function callbacks ignored
- Callbacks stored in dictionaries/lists not tracked

This impacts:
- Event-driven GUI applications
- Web frameworks with routing
- Async/await callback patterns
- Observer pattern implementations
- Plugin architectures

## Objective

Implement comprehensive dynamic dispatch and callback tracking for Python to accurately resolve indirect function calls, callback registrations, and dynamic invocation patterns in the call graph.

## Requirements

### Functional Requirements

- Track function/method references passed as arguments
- Resolve event binding patterns:
  - wxPython: `Bind(event, handler)`
  - Tkinter: `command=handler`
  - Qt: `connect(handler)`
  - Custom observer patterns
- Support callback storage patterns:
  - Callbacks in instance variables
  - Callbacks in dictionaries/lists
  - Callback registration methods
- Handle partial functions and lambdas
- Track decorator-based dispatch (`@app.route`, `@click.command`)
- Support async callback patterns
- Two-phase resolution for deferred callbacks

### Non-Functional Requirements

- Minimal false positives in callback detection
- Reasonable performance for large codebases
- Clear reporting of callback chains
- Extensible for custom patterns

## Acceptance Criteria

- [ ] Event handler bindings create proper call graph edges
- [ ] Method references as callbacks correctly tracked
- [ ] wxPython event handlers show correct callers
- [ ] Flask route decorators create call relationships
- [ ] Callbacks in dictionaries resolved when invoked
- [ ] Lambda callbacks tracked where possible
- [ ] Two-phase resolution handles forward references
- [ ] 90%+ accuracy on common callback patterns
- [ ] Performance impact < 10% on analysis time

## Technical Details

### Implementation Approach

1. Create `CallbackTracker` in `src/analysis/python_callback_tracker.rs`
2. Implement pattern matching for callback registration
3. Add deferred resolution for forward references
4. Integrate with two-pass extractor
5. Create callback resolution phase after initial pass

### Architecture Changes

```rust
// src/analysis/python_callback_tracker.rs
pub struct CallbackTracker {
    pending_callbacks: Vec<PendingCallback>,
    callback_storage: HashMap<String, Vec<CallbackRef>>,
    registration_patterns: Vec<RegistrationPattern>,
}

pub struct PendingCallback {
    callback_expr: String,
    registration_point: Location,
    registration_type: CallbackType,
    context: CallbackContext,
}

pub enum CallbackType {
    EventBinding,
    RouteDecorator,
    SignalConnection,
    DirectAssignment,
    DictionaryStorage,
    ListStorage,
}

pub struct CallbackRef {
    target_function: FunctionId,
    confidence: f32,
}
```

### Data Structures

- `CallbackPattern`: Defines callback registration patterns
- `CallbackContext`: Tracks scope and available symbols
- `ResolutionResult`: Callback resolution with confidence

### APIs and Interfaces

```rust
impl CallbackTracker {
    pub fn track_callback(&mut self, call: &ast::ExprCall, context: &Context);
    pub fn track_assignment(&mut self, target: &ast::Expr, value: &ast::Expr);
    pub fn resolve_callbacks(&self, known_functions: &HashSet<FunctionId>) -> Vec<FunctionCall>;
    pub fn get_callback_confidence(&self, callback: &PendingCallback) -> f32;
}
```

## Dependencies

- **Prerequisites**: [103 - Framework Pattern Detection]
- **Affected Components**:
  - `src/analysis/python_type_tracker.rs`
  - `src/analysis/python_call_graph/`
  - `src/priority/call_graph.rs`
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Each callback pattern type
- **Integration Tests**: Complete callback chains
- **Framework Tests**: Real framework callback code
- **Performance Tests**: Large callback-heavy codebases
- **Accuracy Tests**: False positive/negative rates

## Documentation Requirements

- **Code Documentation**: Callback pattern examples
- **User Documentation**: Supported callback patterns
- **Architecture Updates**: Callback resolution flow
- **Troubleshooting Guide**: Debugging unresolved callbacks

## Implementation Notes

- Use confidence scoring for uncertain callbacks
- Prioritize common patterns (event binding, decorators)
- Consider callback chains (callback calling callback)
- Handle method binding (bound vs unbound methods)
- Track callback modifications/reassignments
- Log callback resolution for debugging

## Migration and Compatibility

- No breaking changes to existing analysis
- Gradual improvement as patterns added
- Backward compatible with current call graph
- Optional callback tracking (can be disabled)