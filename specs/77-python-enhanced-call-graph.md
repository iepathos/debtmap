---
number: 77
title: Python Enhanced Call Graph with Type Tracking
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-09-01
---

# Specification 77: Python Enhanced Call Graph with Type Tracking

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The Rust analyzer uses sophisticated two-pass call graph resolution with type tracking, trait resolution, function signature registry, and global type registry for accurate call resolution. The Python analyzer has basic call graph support with callback patterns but lacks type tracking, making it difficult to resolve method calls accurately and leading to false positives in dead code detection.

Python's dynamic typing makes accurate call graph construction challenging but critical for:
- Dead code detection accuracy
- Understanding code dependencies
- Impact analysis for changes
- Method resolution for object-oriented code

## Objective

Implement enhanced call graph analysis for Python with type inference, method resolution, and two-pass analysis to significantly improve accuracy of call tracking and reduce false positives in dead code detection.

## Requirements

### Functional Requirements
- Implement two-pass call graph resolution
- Add type inference for local variables
- Track class hierarchies and inheritance
- Resolve method calls based on receiver type
- Handle duck typing patterns
- Support property and descriptor access
- Track decorator transformations
- Handle dynamic attribute access patterns

### Non-Functional Requirements
- Incremental type inference without full program analysis
- Efficient caching of type information
- Configurable inference depth
- Graceful degradation for unresolved types

## Acceptance Criteria

- [ ] Two-pass call graph extraction
- [ ] Type tracker for Python implementation
- [ ] Class hierarchy tracking
- [ ] Method resolution based on types
- [ ] Property/descriptor handling
- [ ] Decorator flow tracking
- [ ] Type inference for common patterns
- [ ] Function signature extraction
- [ ] Integration with existing call graph
- [ ] Reduced false positive rate (>30% improvement)
- [ ] Unit tests for type inference
- [ ] Integration tests for call resolution

## Technical Details

### Implementation Approach
1. Create Python type tracking system
2. Implement two-pass resolution strategy
3. Build class hierarchy analyzer
4. Add method resolution logic
5. Integrate with existing call graph

### Architecture Changes
- New module: `src/analysis/python_type_tracker.rs`
- Enhanced call graph extraction
- Type registry for Python

### Data Structures
```rust
pub struct PythonTypeTracker {
    local_types: HashMap<String, PythonType>,
    class_hierarchy: HashMap<String, ClassInfo>,
    function_signatures: HashMap<FunctionId, Signature>,
    current_scope: ScopeStack,
}

pub enum PythonType {
    Class(String),
    Instance(String),
    Function(FunctionSignature),
    Module(String),
    Union(Vec<PythonType>),
    Unknown,
}

pub struct ClassInfo {
    pub name: String,
    pub bases: Vec<String>,
    pub methods: HashMap<String, FunctionId>,
    pub attributes: HashMap<String, PythonType>,
}

pub struct TwoPassExtractor {
    phase_one_calls: Vec<UnresolvedCall>,
    type_tracker: PythonTypeTracker,
    call_graph: CallGraph,
}
```

### APIs and Interfaces
- `PythonTypeTracker::infer_type(expr: &ast::Expr) -> PythonType`
- `TwoPassExtractor::extract(module: &ast::Mod) -> CallGraph`
- Method resolution API

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - `src/analysis/python_call_graph.rs`
  - `src/priority/call_graph.rs`
  - `src/analyzers/python.rs`
- **External Dependencies**: rustpython_parser (existing)

## Testing Strategy

- **Unit Tests**: Type inference accuracy
- **Method Resolution Tests**: Class hierarchy handling
- **Integration Tests**: Full call graph extraction
- **Benchmark Tests**: Performance with large modules

## Documentation Requirements

- **Code Documentation**: Type inference algorithms
- **User Documentation**: Call graph accuracy improvements
- **Architecture**: Two-pass resolution strategy

## Implementation Notes

- Start with basic type patterns (assignments, returns)
- Handle common built-in types
- Support type hints when available
- Infer from constructor calls
- Track self/cls parameters
- Handle multiple inheritance
- Consider metaclasses for advanced cases

## Migration and Compatibility

During prototype phase: Enhancement to existing call graph analysis. Will improve accuracy without breaking existing functionality. Graceful fallback to current behavior when type inference fails.