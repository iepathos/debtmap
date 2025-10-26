---
number: 127
title: Type Flow Tracking Infrastructure for Python
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-25
---

# Specification 127: Type Flow Tracking Infrastructure for Python

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap's Python call graph analyzer currently has an ignored test `test_observer_pattern_cross_module` that fails because the observer pattern detection cannot track what types flow into observer collections like `self.observers`. The existing infrastructure in `src/analysis/python_type_tracker.rs` has basic type tracking but lacks the ability to follow data flow through assignments, method parameters, and collection operations.

**Current Limitation**:
```python
# observer.py
class Observer:
    def update(self, event):
        pass

# concrete_observer.py
from observer import Observer

class ConcreteObserver(Observer):
    def update(self, event):
        self.handle_event(event)

# Subject.notify() does: for obs in self.observers: obs.update(event)
# Problem: Can't determine that self.observers contains Observer/ConcreteObserver types
```

The code has observer dispatch detection (`ObserverDispatchDetector`) and registry infrastructure (`ObserverRegistry`), but cannot identify which concrete types are stored in observer collections to create proper call graph edges.

**Why This Matters**:
- Observer pattern is common in Python (event handling, pub/sub, callbacks)
- False positives in dead code detection for observer implementations
- Missing call graph edges for dynamically dispatched method calls
- Test coverage regression (ignored test blocks quality gates)

## Objective

Implement a **type flow tracking system** that traces how types propagate through Python code via assignments, method calls, and collection operations. This provides the foundation for accurate observer pattern detection and reduces false positives in dead code analysis.

## Requirements

### Functional Requirements

1. **Assignment Type Flow**
   - Track type flow through simple assignments: `x = ConcreteObserver()`
   - Track type flow through attribute assignments: `self.observers = []`
   - Track type flow through augmented assignments: `self.observers += [obs]`
   - Handle multiple assignment targets: `a = b = ConcreteObserver()`

2. **Collection Type Tracking**
   - Track types added via `.append()`: `self.observers.append(observer)`
   - Track types added via list literals: `self.observers = [obs1, obs2]`
   - Track types added via list operations: `self.observers + [new_obs]`
   - Track types in comprehensions: `[Observer() for _ in range(n)]`

3. **Parameter Type Flow**
   - Track types flowing through method parameters
   - Connect call sites to parameter types: `subject.attach(concrete_obs)`
   - Handle positional and keyword arguments
   - Track `self` parameter for method calls

4. **Cross-Statement Flow**
   - Connect variable assignments across statements
   - Track type evolution through reassignments
   - Handle conditional branches (conservative approach)
   - Track through simple function calls

### Non-Functional Requirements

1. **Performance**: Type flow tracking adds <5% overhead to analysis time
2. **Memory**: Flow graph storage scales linearly with code size
3. **Accuracy**: Conservative approach (no false negatives for tracked types)
4. **Maintainability**: Pure functional data structures for flow graph

## Acceptance Criteria

- [ ] `TypeFlowTracker` struct tracks type flows through assignments
- [ ] Collection operations (append, extend, +=) record type additions
- [ ] Method parameter flows connect call sites to parameter types
- [ ] `get_types_in_collection()` returns all types that flow into a collection
- [ ] Cross-statement flows connect related assignments
- [ ] Unit tests verify basic assignment tracking (x = Type())
- [ ] Unit tests verify collection tracking (list.append, list literals)
- [ ] Unit tests verify parameter flow (func(concrete_type))
- [ ] Integration with existing `PythonTypeTracker` via composition
- [ ] Documentation explains flow graph data structure

## Technical Details

### Implementation Approach

**Core Data Structure**:
```rust
pub struct TypeFlowTracker {
    /// Variable -> Set of types that have flowed into it
    variable_types: HashMap<String, HashSet<TypeId>>,

    /// Collection (e.g., "self.observers") -> Types added to it
    collection_types: HashMap<String, HashSet<TypeId>>,

    /// Parameter (func_name, param_index) -> Types passed to it
    parameter_types: HashMap<(String, usize), HashSet<TypeId>>,

    /// Type identifier with source location
    type_registry: HashMap<TypeId, TypeInfo>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TypeId {
    /// Type name (e.g., "ConcreteObserver")
    name: String,
    /// Module where type is defined
    module: Option<PathBuf>,
}

pub struct TypeInfo {
    type_id: TypeId,
    /// Where this type was instantiated/defined
    source_location: Location,
    /// Base classes if known
    base_classes: Vec<TypeId>,
}
```

**Integration Pattern**:
```rust
// In PythonTypeTracker
pub struct PythonTypeTracker {
    // ... existing fields ...
    type_flow: TypeFlowTracker,  // NEW: Composed type flow tracker
}

impl PythonTypeTracker {
    pub fn track_assignment(&mut self, target: &ast::Expr, value: &ast::Expr) {
        // Existing type inference logic...

        // NEW: Track type flow
        if let Some(type_id) = self.infer_type_from_expr(value) {
            self.type_flow.record_assignment(target, type_id);
        }
    }
}
```

### Key Algorithms

**1. Type Inference from Expressions**:
```rust
fn infer_type_from_expr(&self, expr: &ast::Expr) -> Option<TypeId> {
    match expr {
        // Direct instantiation: Type()
        ast::Expr::Call(call) => {
            if let ast::Expr::Name(name) = &*call.func {
                Some(TypeId::new(name.id.to_string(), self.current_module()))
            } else {
                None
            }
        }
        // Variable reference: use existing flow
        ast::Expr::Name(name) => {
            self.type_flow.get_variable_type(&name.id)
        }
        _ => None
    }
}
```

**2. Collection Type Tracking**:
```rust
fn track_collection_operation(&mut self, collection: &str, operation: CollectionOp) {
    match operation {
        CollectionOp::Append(type_id) => {
            self.collection_types
                .entry(collection.to_string())
                .or_default()
                .insert(type_id);
        }
        CollectionOp::Extend(type_ids) => {
            self.collection_types
                .entry(collection.to_string())
                .or_default()
                .extend(type_ids);
        }
    }
}
```

**3. Parameter Flow Tracking**:
```rust
fn track_call(&mut self, func_name: &str, args: &[ast::Expr]) {
    for (idx, arg) in args.iter().enumerate() {
        if let Some(type_id) = self.infer_type_from_expr(arg) {
            self.parameter_types
                .entry((func_name.to_string(), idx))
                .or_default()
                .insert(type_id);
        }
    }
}
```

### Architecture Changes

**New Module**: `src/analysis/type_flow_tracker.rs`
- Pure functional type flow tracking
- No side effects, composable with existing trackers
- Owned by `PythonTypeTracker` via composition

**Modified**: `src/analysis/python_type_tracker.rs`
- Add `type_flow: TypeFlowTracker` field
- Integrate flow tracking in `track_assignment()`
- Add helper methods to query flow information

**Modified**: `src/analysis/python_call_graph/mod.rs`
- Use type flow information for method resolution
- Query collection types when analyzing for loops

### Data Structures

```rust
/// Tracks types flowing through Python code
pub struct TypeFlowTracker {
    variable_types: HashMap<String, HashSet<TypeId>>,
    collection_types: HashMap<String, HashSet<TypeId>>,
    parameter_types: HashMap<(String, usize), HashSet<TypeId>>,
    type_registry: HashMap<TypeId, TypeInfo>,
}

impl TypeFlowTracker {
    /// Record that a type flows into a variable
    pub fn record_assignment(&mut self, target: &ast::Expr, type_id: TypeId);

    /// Record that a type is added to a collection
    pub fn record_collection_add(&mut self, collection: &str, type_id: TypeId);

    /// Record that a type flows into a parameter
    pub fn record_parameter_flow(&mut self, func: &str, param_idx: usize, type_id: TypeId);

    /// Get all types that have flowed into a collection
    pub fn get_collection_types(&self, collection: &str) -> Vec<&TypeInfo>;

    /// Get all types that have flowed into a parameter
    pub fn get_parameter_types(&self, func: &str, param_idx: usize) -> Vec<&TypeInfo>;
}
```

### APIs and Interfaces

**Public API for Observer Detection**:
```rust
// In PythonTypeTracker
impl PythonTypeTracker {
    /// Get all types stored in a collection field
    pub fn get_collection_member_types(&self, class: &str, field: &str) -> Vec<TypeId> {
        let collection_name = format!("{}.{}", class, field);
        self.type_flow
            .get_collection_types(&collection_name)
            .into_iter()
            .map(|info| info.type_id.clone())
            .collect()
    }
}
```

## Dependencies

**Prerequisites**: None (foundational spec)

**Affected Components**:
- `src/analysis/python_type_tracker.rs` - Add type flow field and integration
- `src/analysis/python_call_graph/mod.rs` - Query type flow for method resolution

**External Dependencies**: None (uses existing rustpython-parser)

## Testing Strategy

### Unit Tests

**File**: `src/analysis/type_flow_tracker.rs`

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_simple_assignment() {
        // x = ConcreteObserver()
        let mut tracker = TypeFlowTracker::new();
        let type_id = TypeId::new("ConcreteObserver", None);
        tracker.record_assignment("x", type_id.clone());

        let types = tracker.get_variable_types("x");
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].type_id, type_id);
    }

    #[test]
    fn test_collection_append() {
        // self.observers.append(ConcreteObserver())
        let mut tracker = TypeFlowTracker::new();
        let type_id = TypeId::new("ConcreteObserver", None);
        tracker.record_collection_add("self.observers", type_id.clone());

        let types = tracker.get_collection_types("self.observers");
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].type_id.name, "ConcreteObserver");
    }

    #[test]
    fn test_parameter_flow() {
        // subject.attach(observer)
        let mut tracker = TypeFlowTracker::new();
        let type_id = TypeId::new("ConcreteObserver", None);
        tracker.record_parameter_flow("Subject.attach", 0, type_id.clone());

        let types = tracker.get_parameter_types("Subject.attach", 0);
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].type_id.name, "ConcreteObserver");
    }

    #[test]
    fn test_multiple_types_in_collection() {
        // observers = [ObsA(), ObsB()]
        let mut tracker = TypeFlowTracker::new();
        tracker.record_collection_add("observers", TypeId::new("ObsA", None));
        tracker.record_collection_add("observers", TypeId::new("ObsB", None));

        let types = tracker.get_collection_types("observers");
        assert_eq!(types.len(), 2);
    }
}
```

### Integration Tests

**File**: `tests/type_flow_integration_test.rs`

```rust
#[test]
fn test_type_flow_through_method_chain() {
    let code = r#"
class Observer:
    pass

class Subject:
    def __init__(self):
        self.observers = []

    def attach(self, observer):
        self.observers.append(observer)

subject = Subject()
observer = Observer()
subject.attach(observer)
"#;

    let tracker = analyze_type_flow(code);

    // Verify Observer type flows into self.observers
    let types = tracker.get_collection_member_types("Subject", "observers");
    assert!(types.iter().any(|t| t.name == "Observer"));
}
```

## Documentation Requirements

### Code Documentation

1. **Module-level docs** for `type_flow_tracker.rs`:
   - Explain purpose: track type propagation through assignments
   - Document limitations: conservative analysis, no inter-procedural flow
   - Provide usage examples

2. **Struct documentation**:
   - `TypeFlowTracker`: Main API and usage patterns
   - `TypeId`: Type identification and equality semantics
   - `TypeInfo`: Type metadata and source tracking

3. **Method documentation**:
   - All public methods with examples
   - Explain conservative analysis approach
   - Document edge cases (reassignments, branches)

### Architecture Updates

**Update**: `ARCHITECTURE.md`

Add section under "Python Analysis":
```markdown
### Type Flow Tracking

The type flow tracker (`src/analysis/type_flow_tracker.rs`) implements a
conservative data flow analysis to track how types propagate through Python
code. It tracks three primary flows:

1. **Variable assignments**: `x = ConcreteType()`
2. **Collection operations**: `list.append(item)`
3. **Parameter passing**: `func(concrete_arg)`

The tracker uses a conservative approach: if a type *might* flow into a
location, it's recorded. This prevents false negatives in call graph
construction while accepting some over-approximation.

**Integration**: Type flow tracker is composed into `PythonTypeTracker` and
queried during observer pattern detection and method resolution.
```

## Implementation Notes

### Conservative Analysis Approach

The type flow tracker uses **conservative over-approximation**:
- If a type might flow into a location, record it
- Prefer false positives (extra types) over false negatives (missed types)
- For branches: union all possible types from all branches
- For loops: assume all iterations contribute types

**Rationale**: Better to have extra call graph edges than miss real calls.

### Performance Considerations

1. **Lazy evaluation**: Only compute flows when queried
2. **Incremental updates**: Add types to sets without recomputing
3. **Hash-based lookups**: O(1) type queries via HashMaps
4. **No global analysis**: File-local flow tracking (cross-module in spec 129)

### Known Limitations

1. **No inter-procedural flow**: Doesn't track types across function boundaries
2. **No aliasing**: `x = y; x.append(item)` doesn't track to `y`
3. **No container unpacking**: `a, b = get_observers()` not tracked
4. **Simple control flow**: Branches treated conservatively

These limitations are acceptable for the observer pattern use case and can be
enhanced incrementally.

### Gotchas

1. **String-based tracking**: Use normalized names (`"self.observers"` not `"observers"`)
2. **Module context**: TypeId needs module information for cross-module resolution
3. **Immutability**: TypeFlowTracker uses interior mutability for incremental updates
4. **Memory growth**: Sets grow monotonically (no type removal)

## Migration and Compatibility

**Breaking Changes**: None (additive feature)

**Compatibility**:
- Existing type tracking continues to work
- Type flow is opt-in via new API methods
- No changes to existing call graph structure
- Tests remain compatible

**Migration Path**:
1. Add TypeFlowTracker to PythonTypeTracker
2. Integrate in track_assignment() calls
3. Expose query methods for consumers
4. Observer detection uses new API in spec 128

**Rollback Plan**: If issues arise, type flow tracker can be disabled by not
querying it - existing analysis continues unchanged.
