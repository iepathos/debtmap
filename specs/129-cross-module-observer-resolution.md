---
number: 129
title: Cross-Module Observer Pattern Resolution
category: foundation
priority: high
status: draft
dependencies: [127, 128]
created: 2025-10-25
---

# Specification 129: Cross-Module Observer Pattern Resolution

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Specs 127 (Type Flow Tracking), 128 (Observer Interface Discovery)

## Context

The failing test `test_observer_pattern_cross_module` involves observer patterns split across multiple Python files - a common real-world pattern where interface and concrete implementations live in separate modules:

```python
# observer.py
class Observer:
    def update(self, event): pass

class Subject:
    def notify(self):
        for obs in self.observers:
            obs.update(event)

# concrete_observer.py
from observer import Observer

class ConcreteObserver(Observer):
    def update(self, event):
        self.handle_event(event)
```

**Current Limitation**: While specs 127 and 128 handle type flow tracking and interface discovery within a single file, they don't handle the cross-module aspects:

1. **Import resolution**: `from observer import Observer` needs to resolve to the Observer class in observer.py
2. **Cross-file type flow**: When `ConcreteObserver` is added to `Subject.observers`, the type must be tracked across file boundaries
3. **Interface inheritance across modules**: `ConcreteObserver(Observer)` where `Observer` is imported needs to register the cross-module inheritance
4. **Call edge creation**: `Subject.notify` calling `obs.update()` must create edges to `ConcreteObserver.update` in a different file

**Existing Infrastructure**:
Debtmap already has cross-module infrastructure:
- `CrossModuleContext` tracks classes and functions across files
- `ImportTracker` maps imports to source modules
- `analyze_python_project()` builds unified context

**The Gap**: These components aren't integrated with the observer pattern detection and type flow tracking from specs 127-128.

## Objective

Integrate type flow tracking and observer interface discovery with debtmap's existing cross-module analysis infrastructure to accurately detect and resolve observer patterns that span multiple Python files.

## Requirements

### Functional Requirements

1. **Cross-Module Type Resolution**
   - Resolve imported types to their source modules
   - Track type flow across module boundaries
   - Maintain TypeId with module information

2. **Cross-Module Interface Registration**
   - Register interfaces defined in imported modules
   - Track which modules provide which observer interfaces
   - Share interface registry across file analysis

3. **Cross-Module Implementation Tracking**
   - Detect when a class in module A inherits from an interface in module B
   - Register implementations with fully-qualified names (module + class)
   - Connect implementations to interfaces across module boundaries

4. **Cross-Module Call Edge Creation**
   - Create call edges from dispatch sites to implementations in other modules
   - Handle transitive inheritance (A inherits from B which inherits from C in different modules)
   - Resolve method calls to the correct implementation across files

### Non-Functional Requirements

1. **Scalability**: Handle projects with 100+ modules efficiently
2. **Correctness**: No false negatives for cross-module observer patterns
3. **Performance**: Cross-module resolution adds <15% overhead
4. **Maintainability**: Leverage existing CrossModuleContext infrastructure

## Acceptance Criteria

- [ ] TypeId includes module path for cross-module resolution
- [ ] Type flow tracker resolves imported types to source modules
- [ ] Observer registry is shared across file analyses via CrossModuleContext
- [ ] Imported base classes are resolved and registered as interfaces
- [ ] Concrete implementations in one module connect to interfaces in another
- [ ] Dispatch loops create call edges to implementations in other modules
- [ ] Test `test_observer_pattern_cross_module` passes without `#[ignore]`
- [ ] Integration test with 3+ modules verifies transitive resolution
- [ ] Performance benchmark shows <15% overhead for cross-module projects
- [ ] No regressions in single-file observer pattern detection

## Technical Details

### Implementation Approach

**Shared Observer Registry Pattern**:
```rust
// In CrossModuleContext
pub struct CrossModuleContext {
    // ... existing fields ...

    /// Shared observer registry across all files
    observer_registry: Arc<RwLock<ObserverRegistry>>,

    /// Shared type flow tracker (NEW)
    type_flow: Arc<RwLock<TypeFlowTracker>>,
}

impl CrossModuleContext {
    pub fn new() -> Self {
        let observer_registry = Arc::new(RwLock::new(ObserverRegistry::new()));
        let type_flow = Arc::new(RwLock::new(TypeFlowTracker::new()));

        Self {
            // ... existing fields ...
            observer_registry,
            type_flow,
        }
    }

    /// Get shared observer registry for a file analysis
    pub fn observer_registry(&self) -> Arc<RwLock<ObserverRegistry>> {
        Arc::clone(&self.observer_registry)
    }

    /// Get shared type flow tracker
    pub fn type_flow(&self) -> Arc<RwLock<TypeFlowTracker>> {
        Arc::clone(&self.type_flow)
    }
}
```

**Modified TwoPassExtractor Integration**:
```rust
impl TwoPassExtractor {
    pub fn new_with_context(
        file_path: PathBuf,
        source: &str,
        context: CrossModuleContext
    ) -> Self {
        Self {
            // ... existing fields ...

            // Use shared registries from context
            observer_registry: context.observer_registry(),
            type_flow: context.type_flow(),

            cross_module_context: Some(context),
        }
    }
}
```

### Architecture Changes

**Modified**: `src/analysis/python_call_graph/cross_module.rs`

```rust
pub struct CrossModuleContext {
    // Existing fields
    pub classes: HashMap<String, ClassInfo>,
    pub functions: HashMap<String, FunctionInfo>,
    pub imports: HashMap<PathBuf, ImportMap>,

    // NEW: Shared analysis infrastructure
    observer_registry: Arc<RwLock<ObserverRegistry>>,
    type_flow: Arc<RwLock<TypeFlowTracker>>,
}

impl CrossModuleContext {
    /// Resolve an imported type to its source module
    pub fn resolve_imported_type(&self, import_name: &str, current_file: &Path) -> Option<TypeId> {
        let import_map = self.imports.get(current_file)?;
        let source_module = import_map.resolve_module(import_name)?;

        Some(TypeId {
            name: import_name.to_string(),
            module: Some(source_module),
        })
    }

    /// Get observer registry (shared across all files)
    pub fn observer_registry(&self) -> Arc<RwLock<ObserverRegistry>> {
        Arc::clone(&self.observer_registry)
    }

    /// Record that a type flows into a collection across modules
    pub fn record_cross_module_type_flow(
        &self,
        collection: &str,
        type_id: TypeId,
    ) {
        let mut flow = self.type_flow.write().unwrap();
        flow.record_collection_add(collection, type_id);
    }
}
```

**Modified**: `src/analysis/type_flow_tracker.rs`

```rust
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TypeId {
    /// Type name (e.g., "ConcreteObserver")
    pub name: String,

    /// Module where type is defined (None for builtins)
    /// CHANGED: Was Option<PathBuf>, now required for cross-module
    pub module: Option<PathBuf>,
}

impl TypeId {
    /// Create a TypeId with module resolution via imports
    pub fn from_name_with_context(
        name: &str,
        current_file: &Path,
        context: &CrossModuleContext,
    ) -> Self {
        // Try to resolve via imports
        if let Some(resolved) = context.resolve_imported_type(name, current_file) {
            resolved
        } else {
            // Local type in current file
            TypeId {
                name: name.to_string(),
                module: Some(current_file.to_path_buf()),
            }
        }
    }
}
```

### Key Algorithms

**Algorithm 1: Cross-Module Type Resolution**
```
Input: Type name, current file, CrossModuleContext
Output: Fully-qualified TypeId with module

1. Check if name is in current file's import map
2. If imported:
   a. Resolve to source module via import tracker
   b. Return TypeId { name, module: source_module }
3. If not imported:
   a. Assume defined in current file
   b. Return TypeId { name, module: current_file }
4. Cache result for performance
```

**Algorithm 2: Cross-Module Interface Registration**
```
Input: Base class name, current file, CrossModuleContext
Output: Interface registered in shared registry

1. Resolve base class to TypeId via algorithm 1
2. Get shared observer registry from context
3. Register TypeId as interface in shared registry
4. Record module where interface is defined
5. Future analyses in other modules see this registration
```

**Algorithm 3: Cross-Module Implementation Tracking**
```
Input: Class definition with imported base, CrossModuleContext
Output: Implementation registered with interface

1. For each base class in class definition:
   a. Resolve base class to TypeId (includes module)
   b. Check shared registry if base is an interface
   c. If yes, register current class as implementation
2. For each method in class:
   a. Find matching interface methods in base classes
   b. Register method as implementation of interface method
3. Store implementation with full path (module + class + method)
```

**Algorithm 4: Cross-Module Dispatch Resolution**
```
Input: Dispatch loop, observer collection, CrossModuleContext
Output: Call edges to implementations across modules

1. Get types in collection from shared type flow tracker
2. For each type (includes module information):
   a. Get interface from shared registry
   b. Find all implementations of interface method
   c. Filter to implementations matching type
3. For each matching implementation:
   a. Create call edge from dispatcher to implementation
   b. Edge includes source file (dispatch site) and target file (implementation)
```

### Data Structures

**Enhanced TypeId**:
```rust
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TypeId {
    /// Fully qualified type name
    pub name: String,

    /// Module defining this type (for cross-module resolution)
    pub module: Option<PathBuf>,
}

impl TypeId {
    /// Get qualified name for disambiguation
    pub fn qualified_name(&self) -> String {
        if let Some(module) = &self.module {
            format!("{}:{}", module.display(), self.name)
        } else {
            self.name.clone()
        }
    }

    /// Check if this type matches an imported name in a given context
    pub fn matches_in_context(&self, name: &str, context: &CrossModuleContext) -> bool {
        // Handle both local and imported name matches
        self.name == name || context.resolves_to(name, self)
    }
}
```

**Cross-Module Observer Info**:
```rust
#[derive(Debug, Clone)]
pub struct CrossModuleObserverInfo {
    /// Interface type with module
    pub interface: TypeId,

    /// All implementations across all modules
    pub implementations: HashMap<TypeId, Vec<FunctionId>>,

    /// Dispatch sites that call this interface
    pub dispatch_sites: Vec<FunctionId>,
}
```

### APIs and Interfaces

**CrossModuleContext API Extensions**:
```rust
impl CrossModuleContext {
    /// Resolve a type name to its defining module
    pub fn resolve_type_module(&self, name: &str, current_file: &Path) -> Option<PathBuf>;

    /// Register an interface discovered in a specific module
    pub fn register_interface_in_module(&mut self, interface: TypeId);

    /// Register an implementation of a cross-module interface
    pub fn register_cross_module_implementation(
        &mut self,
        interface: &TypeId,
        implementation: FunctionId,
    );

    /// Get all implementations of an interface across modules
    pub fn get_cross_module_implementations(&self, interface: &TypeId) -> Vec<FunctionId>;

    /// Merge observer registries from parallel file analyses
    pub fn merge_observer_registries(&mut self, registries: Vec<ObserverRegistry>);
}
```

## Dependencies

**Prerequisites**:
- **Spec 127**: Type Flow Tracking Infrastructure
- **Spec 128**: Observer Interface Discovery

**Affected Components**:
- `src/analysis/python_call_graph/cross_module.rs` - Add shared registries
- `src/analysis/python_call_graph/analyze.rs` - Integrate shared context
- `src/analysis/python_type_tracker.rs` - Use cross-module resolution
- `src/analysis/type_flow_tracker.rs` - Support cross-module TypeIds

**External Dependencies**: None

## Testing Strategy

### Unit Tests

**File**: `src/analysis/python_call_graph/cross_module.rs`

```rust
#[cfg(test)]
mod cross_module_observer_tests {
    #[test]
    fn test_resolve_imported_type() {
        let mut context = CrossModuleContext::new();

        // Register import: observer.py exports Observer
        let observer_file = PathBuf::from("observer.py");
        let concrete_file = PathBuf::from("concrete.py");

        context.register_import(
            &concrete_file,
            "Observer",
            &observer_file
        );

        // Resolve Observer in concrete.py context
        let type_id = context.resolve_imported_type("Observer", &concrete_file);

        assert!(type_id.is_some());
        assert_eq!(type_id.unwrap().module, Some(observer_file));
    }

    #[test]
    fn test_shared_observer_registry() {
        let context = CrossModuleContext::new();

        // Simulate two files sharing registry
        {
            let mut registry = context.observer_registry().write().unwrap();
            registry.register_interface("Observer");
        }

        // Check from different "file" analysis
        {
            let registry = context.observer_registry().read().unwrap();
            assert!(registry.is_interface("Observer"));
        }
    }

    #[test]
    fn test_cross_module_type_flow() {
        let context = CrossModuleContext::new();

        let observer_type = TypeId {
            name: "ConcreteObserver".to_string(),
            module: Some(PathBuf::from("concrete.py")),
        };

        // Record type flow
        context.record_cross_module_type_flow(
            "Subject.observers",
            observer_type.clone()
        );

        // Verify type is in collection
        let flow = context.type_flow().read().unwrap();
        let types = flow.get_collection_types("Subject.observers");

        assert_eq!(types.len(), 1);
        assert_eq!(types[0].type_id, observer_type);
    }
}
```

### Integration Tests

**File**: `tests/python_cross_module_test.rs`

```rust
#[test]
// REMOVE #[ignore] - This test should now pass!
fn test_observer_pattern_cross_module() {
    let temp_dir = TempDir::new().unwrap();

    // Create observer.py with observer base class
    let observer_path = temp_dir.path().join("observer.py");
    fs::write(
        &observer_path,
        r#"
class Observer:
    def update(self, event):
        """Called when an event occurs."""
        pass

class Subject:
    def __init__(self):
        self.observers = []

    def attach(self, observer):
        """Attach an observer."""
        self.observers.append(observer)

    def notify(self, event):
        """Notify all observers."""
        for obs in self.observers:
            obs.update(event)
"#,
    )
    .unwrap();

    // Create concrete_observer.py
    let concrete_path = temp_dir.path().join("concrete_observer.py");
    fs::write(
        &concrete_path,
        r#"
from observer import Observer, Subject

class ConcreteObserver(Observer):
    def update(self, event):
        """Handle the update event."""
        print(f"Received event: {event}")
        self.handle_event(event)

    def handle_event(self, event):
        """Process the event."""
        return event.upper() if isinstance(event, str) else event

def setup_observers():
    subject = Subject()
    observer = ConcreteObserver()
    subject.attach(observer)
    subject.notify("test_event")
"#,
    )
    .unwrap();

    // Analyze the project
    let files = vec![observer_path, concrete_path];
    let call_graph = analyze_python_project(&files).unwrap();

    // Check that observer pattern methods are tracked across modules
    let functions: Vec<_> = call_graph.get_all_functions().collect();

    let update_id = functions
        .iter()
        .find(|f| f.name.contains("ConcreteObserver") && f.name.contains("update"))
        .expect("ConcreteObserver.update should be found");

    let handle_event_id = functions
        .iter()
        .find(|f| f.name.contains("handle_event"))
        .expect("handle_event should be found");

    // The update method should have callers (from notify)
    assert!(
        !call_graph.get_callers(update_id).is_empty(),
        "ConcreteObserver.update should be called through observer pattern"
    );

    // The handle_event should have callers (from update)
    assert!(
        !call_graph.get_callers(handle_event_id).is_empty(),
        "handle_event should be called from update"
    );
}

#[test]
fn test_three_module_observer_chain() {
    // Test transitive inheritance: Interface -> Base -> Concrete across 3 files
    let temp_dir = TempDir::new().unwrap();

    let interface_path = temp_dir.path().join("interface.py");
    fs::write(&interface_path, r#"
class EventInterface:
    def handle(self): pass
"#).unwrap();

    let base_path = temp_dir.path().join("base.py");
    fs::write(&base_path, r#"
from interface import EventInterface

class BaseHandler(EventInterface):
    def handle(self):
        self.process()

    def process(self): pass
"#).unwrap();

    let concrete_path = temp_dir.path().join("concrete.py");
    fs::write(&concrete_path, r#"
from base import BaseHandler

class ConcreteHandler(BaseHandler):
    def process(self):
        print("processing")
"#).unwrap();

    let manager_path = temp_dir.path().join("manager.py");
    fs::write(&manager_path, r#"
from concrete import ConcreteHandler

class EventManager:
    def __init__(self):
        self.handlers = [ConcreteHandler()]

    def dispatch(self):
        for h in self.handlers:
            h.handle()
"#).unwrap();

    let files = vec![interface_path, base_path, concrete_path, manager_path];
    let call_graph = analyze_python_project(&files).unwrap();

    // Verify call chain: EventManager.dispatch -> ConcreteHandler.handle -> ConcreteHandler.process
    let dispatch = call_graph.find_function("EventManager.dispatch").unwrap();
    let concrete_handle = call_graph.find_function("ConcreteHandler.handle").unwrap();
    let concrete_process = call_graph.find_function("ConcreteHandler.process").unwrap();

    let dispatch_callees = call_graph.get_callees(&dispatch);
    assert!(dispatch_callees.contains(&concrete_handle));

    let handle_callees = call_graph.get_callees(&concrete_handle);
    assert!(handle_callees.contains(&concrete_process));
}
```

### Performance Tests

```rust
#[test]
fn test_cross_module_observer_performance() {
    // Generate 50 modules with observer patterns
    let temp_dir = TempDir::new().unwrap();
    let files = generate_multi_module_observer_project(&temp_dir, 50);

    let start = Instant::now();
    let call_graph = analyze_python_project(&files).unwrap();
    let duration = start.elapsed();

    // Cross-module resolution should add <15% overhead
    // Baseline for 50 modules: ~500ms, with cross-module: <575ms
    assert!(duration < Duration::from_millis(575));

    // Verify correctness
    let observer_calls = call_graph
        .get_all_calls()
        .filter(|call| call.call_type == CallType::ObserverDispatch)
        .count();

    assert!(observer_calls > 0, "Should detect observer dispatches");
}
```

## Documentation Requirements

### Code Documentation

**Module-level docs** for cross-module integration:
```rust
//! Cross-Module Observer Pattern Resolution
//!
//! Integrates observer pattern detection with debtmap's cross-module analysis
//! infrastructure to handle observer patterns split across multiple files.
//!
//! # Architecture
//!
//! Uses shared state pattern to enable multiple file analyses to contribute to
//! a unified observer registry:
//!
//! 1. CrossModuleContext maintains shared ObserverRegistry
//! 2. Each file's TwoPassExtractor receives Arc<RwLock<ObserverRegistry>>
//! 3. Interfaces and implementations registered across all files
//! 4. Final call graph includes cross-module observer dispatch edges
//!
//! # Example
//!
//! ```text
//! observer.py:
//!   class Observer: ...
//!   class Subject:
//!     def notify(self): for o in self.observers: o.update()
//!
//! concrete.py:
//!   from observer import Observer
//!   class Concrete(Observer): ...
//!
//! Analysis:
//!   1. observer.py analysis registers Observer as interface
//!   2. concrete.py analysis sees imported Observer, registers Concrete as impl
//!   3. Subject.notify dispatch creates edge to Concrete.update (cross-module!)
//! ```
```

### Architecture Updates

**Update**: `ARCHITECTURE.md`

```markdown
### Cross-Module Observer Pattern Resolution

Observer patterns in Python often span multiple files. Debtmap handles this
through **shared analysis state** in CrossModuleContext:

**Shared Registries**:
- `ObserverRegistry`: Tracks interfaces and implementations across all files
- `TypeFlowTracker`: Tracks type propagation across module boundaries

**Resolution Process**:

1. **Import Resolution**: Map imported names to source modules
   ```python
   # concrete.py
   from observer import Observer  # Resolves to observer.py
   ```

2. **Cross-Module Type Flow**: Track types across imports
   ```python
   # subject.py: self.observers.append(ConcreteObserver())
   # TypeId { name: "ConcreteObserver", module: "concrete.py" }
   ```

3. **Shared Interface Registry**: Interfaces registered once, visible to all
   ```rust
   // observer.py registers Observer as interface
   // concrete.py sees Observer is interface, registers as implementation
   ```

4. **Cross-Module Call Edges**: Dispatch sites link to implementations in other files
   ```
   Subject.notify (observer.py) → ConcreteObserver.update (concrete.py)
   ```

**Dependencies**: Specs 127 (Type Flow), 128 (Interface Discovery)
```

## Implementation Notes

### Thread Safety

The shared registries use `Arc<RwLock<>>` for thread-safe access during parallel
file analysis:

```rust
// Multiple files analyzed in parallel via rayon
files.par_iter()
    .map(|file| {
        // Each gets clone of Arc (cheap)
        let registry = context.observer_registry();

        // Read lock for queries
        let reg_read = registry.read().unwrap();
        if reg_read.is_interface("Observer") { ... }

        // Write lock for modifications
        let mut reg_write = registry.write().unwrap();
        reg_write.register_interface("NewInterface");
    })
```

**Lock Ordering**: Always acquire locks in consistent order to prevent deadlocks:
1. TypeFlowTracker lock
2. ObserverRegistry lock

### Import Resolution Edge Cases

1. **Relative imports**: `from .observer import Observer`
   - Resolve relative to current file's directory
   - Use path canonicalization

2. **Star imports**: `from observer import *`
   - Track all exported symbols from module
   - Conservative: assume all classes could be interfaces

3. **Aliased imports**: `from observer import Observer as Obs`
   - Map alias to original name
   - Maintain alias table in ImportTracker

4. **Circular imports**: A imports B, B imports A
   - Shared registry handles this naturally
   - Interface registration happens once

### Performance Optimization

1. **Lazy lock acquisition**: Only lock when needed
2. **Read locks dominate**: Most operations query, few modify
3. **Batch updates**: Collect changes, apply in one write lock
4. **Cache import resolutions**: Module → TypeId mapping cached

### Debugging Support

Add debug logging for cross-module resolution:
```rust
#[cfg(debug_assertions)]
{
    eprintln!(
        "Resolved {} in {} to module {}",
        type_name, current_file, resolved_module
    );
}
```

## Migration and Compatibility

**Breaking Changes**: None (internal refactoring)

**API Changes**:
- `TypeId` gains `module` field (was placeholder, now required)
- `CrossModuleContext` gains observer_registry() and type_flow() methods

**Migration Path**:
1. Spec 127: Add type flow infrastructure
2. Spec 128: Add single-file interface discovery
3. **Spec 129: Enable cross-module resolution** ← This spec
4. Remove `#[ignore]` from failing test
5. Run full test suite to verify no regressions

**Rollback Plan**:
If issues arise, can disable cross-module observer resolution by:
1. Not sharing registries in CrossModuleContext
2. Each file maintains local registry (current behavior)
3. Cross-module edges won't be created but single-file patterns still work

**Success Criteria**:
- [ ] Test `test_observer_pattern_cross_module` passes
- [ ] No regressions in existing tests
- [ ] Performance overhead <15% on multi-file projects
- [ ] Handles 3+ module transitive inheritance correctly
