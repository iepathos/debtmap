---
number: 103
title: Python Framework Pattern Detection
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-09-29
---

# Specification 103: Python Framework Pattern Detection

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The current Python analyzer fails to recognize framework-specific patterns as entry points in the call graph, resulting in false "dead code" detection for event handlers and framework callbacks. Functions like `on_paint()`, `on_message_drag()`, and other GUI event handlers are incorrectly marked as having "no callers" when they are actually called by frameworks like wxPython, Tkinter, Django, Flask, and others.

Current limitations:
- Only recognizes limited framework patterns (e.g., "OnInit", "OnPaint")
- Misses common `on_*`, `handle_*`, and `process_*` patterns
- No auto-detection of framework usage from imports
- No support for framework-specific lifecycle methods
- Event handlers appear as dead code in analysis output

This significantly impacts the accuracy of technical debt analysis for Python projects using GUI frameworks or web frameworks with implicit calling conventions.

## Objective

Implement comprehensive framework pattern detection for Python that automatically identifies framework entry points, event handlers, and lifecycle methods to eliminate false positive dead code detection and improve call graph accuracy.

## Requirements

### Functional Requirements

- Auto-detect framework usage from import statements
- Recognize common event handler patterns (`on_*`, `handle_*`, `process_*`)
- Support major Python frameworks:
  - GUI: wxPython, Tkinter, PyQt, Kivy, PySide
  - Web: Django, Flask, FastAPI, Tornado
  - Testing: pytest, unittest, nose
  - Async: asyncio, trio
- Mark framework entry points in call graph
- Support custom pattern configuration
- Handle decorator-based entry points (e.g., `@app.route`)

### Non-Functional Requirements

- Zero performance impact when no frameworks detected
- Configurable pattern registry for extensibility
- Clear documentation of supported patterns
- Maintain backward compatibility

## Acceptance Criteria

- [ ] Framework auto-detection from imports works for 10+ major frameworks
- [ ] Event handler patterns (`on_*`, `handle_*`) correctly marked as entry points
- [ ] wxPython event handlers no longer show as dead code
- [ ] Django view functions recognized as entry points
- [ ] Flask route handlers identified correctly
- [ ] Framework detection covered by unit tests
- [ ] Configuration file for custom patterns supported
- [ ] Documentation updated with supported frameworks list

## Technical Details

### Implementation Approach

1. Create `FrameworkPatternRegistry` in `src/analysis/framework_patterns.rs`
2. Implement framework detection from import analysis
3. Define pattern sets for each framework
4. Integrate with `PythonTypeTracker` to mark entry points
5. Update call graph builder to respect framework entry points

### Architecture Changes

```rust
// src/analysis/framework_patterns.rs
pub struct FrameworkPatternRegistry {
    patterns: HashMap<FrameworkType, FrameworkPattern>,
    custom_patterns: Vec<CustomPattern>,
}

pub struct FrameworkPattern {
    name: String,
    import_indicators: Vec<String>,
    event_handler_patterns: Vec<String>,
    lifecycle_methods: Vec<String>,
    decorator_patterns: Vec<String>,
}

pub enum FrameworkType {
    WxPython,
    Tkinter,
    Django,
    Flask,
    FastAPI,
    Pytest,
    // ... more
}
```

### Data Structures

- `FrameworkPattern`: Defines patterns for a specific framework
- `CustomPattern`: User-defined pattern rules
- `FrameworkContext`: Tracks detected frameworks per file

### APIs and Interfaces

```rust
impl FrameworkPatternRegistry {
    pub fn detect_frameworks(&self, imports: &[String]) -> Vec<FrameworkType>;
    pub fn is_entry_point(&self, func_name: &str, context: &FrameworkContext) -> bool;
    pub fn is_event_handler(&self, func_name: &str) -> bool;
    pub fn load_custom_patterns(&mut self, path: &Path) -> Result<()>;
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/analysis/python_type_tracker.rs`
  - `src/analyzers/python.rs`
  - `src/analysis/python_call_graph/`
- **External Dependencies**: None (patterns defined internally)

## Testing Strategy

- **Unit Tests**: Pattern matching for each framework
- **Integration Tests**: Full Python file analysis with framework code
- **Regression Tests**: Ensure no false positives introduced
- **Performance Tests**: Verify no impact on non-framework code

## Documentation Requirements

- **Code Documentation**: Document each framework's patterns
- **User Documentation**: List of supported frameworks and patterns
- **Configuration Guide**: How to add custom patterns
- **Architecture Updates**: Update ARCHITECTURE.md with pattern detection flow

## Implementation Notes

- Start with most common frameworks (wxPython, Django, Flask)
- Use lazy evaluation for pattern matching
- Consider caching framework detection per project
- Allow disabling framework detection via configuration
- Log detected frameworks for debugging

## Migration and Compatibility

- No breaking changes to existing API
- Existing analysis results remain valid
- New patterns applied automatically on next analysis
- Configuration migration not required