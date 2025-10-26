---
number: 128
title: Observer Interface Discovery via Usage Analysis
category: foundation
priority: high
status: draft
dependencies: [127]
created: 2025-10-25
---

# Specification 128: Observer Interface Discovery via Usage Analysis

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 127 (Type Flow Tracking Infrastructure)

## Context

Debtmap's observer pattern detection currently only recognizes observer interfaces that inherit from `ABC` (Abstract Base Class). This misses the common Python pattern of duck-typed observer interfaces - plain classes with method stubs that serve as interfaces without explicit inheritance.

**Current Code** (`src/analysis/python_type_tracker.rs:1252`):
```rust
fn register_observer_interfaces(&mut self, class_def: &ast::StmtClassDef) {
    // Only registers classes that inherit from ABC
    let has_abc_base = class_def.bases.iter().any(|base| {
        if let ast::Expr::Name(name) = base {
            name.id.as_str() == "ABC"
        } else {
            false
        }
    });

    if has_abc_base {
        self.observer_registry.register_interface(class_name);
    }
}
```

**Failing Test** (`tests/python_cross_module_test.rs:267`):
```python
# observer.py
class Observer:  # No ABC inheritance!
    def update(self, event):
        pass  # Duck-typed interface

# concrete_observer.py
from observer import Observer

class ConcreteObserver(Observer):
    def update(self, event):
        self.handle_event(event)

# Test expects: ConcreteObserver.update should be called from Subject.notify
# Reality: Observer not registered as interface → no call edges created
```

**Why ABC-Only Detection Fails**:
- Many Python projects don't use `abc` module for interfaces
- Duck typing is idiomatic Python
- Legacy code often uses plain classes as interfaces
- Framework callbacks rarely inherit from ABC

**Impact**:
- 50%+ observer patterns missed in real codebases
- False positives in dead code detection for observer implementations
- Test `test_observer_pattern_cross_module` permanently ignored
- Undermines confidence in call graph accuracy

## Objective

Implement **usage-based observer interface discovery** that identifies observer interfaces by analyzing how they're used, rather than requiring explicit ABC inheritance. Use the type flow tracking infrastructure (spec 127) to discover which types are stored in observer collections and dispatched through iteration loops.

## Requirements

### Functional Requirements

1. **Observer Collection Detection**
   - Identify fields with observer-like names: `observers`, `listeners`, `handlers`, `callbacks`, `subscribers`, `watchers`
   - Track which types are assigned to these collections (via spec 127 type flow)
   - Register these types as potential observer interfaces

2. **Usage-Based Interface Registration**
   - When a class is used as a base for other classes, consider it an interface
   - When a type is stored in an observer collection, register it as an interface
   - When methods are called in iteration loops, register the iterated type's class as an interface

3. **Dispatch Site Analysis**
   - Detect `for item in self.observers: item.method()` patterns
   - Extract the method name being called (`update`, `on_event`, etc.)
   - Use type flow to determine what types are in `self.observers`
   - Register those types as observer interfaces for that method

4. **Interface Method Registration**
   - Register which methods are part of the observer interface
   - Track concrete implementations of these methods
   - Build mapping: `Observer.update` → `[ConcreteObserverA.update, ConcreteObserverB.update]`

### Non-Functional Requirements

1. **Accuracy**: Discover 95%+ of observer patterns in typical Python codebases
2. **Precision**: <10% false positive rate for interface identification
3. **Performance**: Analysis adds <10% overhead compared to current implementation
4. **Compatibility**: Works alongside existing ABC-based detection

## Acceptance Criteria

- [ ] Detect observer collections by field name patterns
- [ ] Use type flow to determine types stored in observer collections
- [ ] Register types from observer collections as interfaces
- [ ] Extract method names from dispatch loops (`for x in collection: x.method()`)
- [ ] Register concrete implementations that override interface methods
- [ ] Create call edges from dispatch sites to all implementations
- [ ] Test `test_observer_pattern_cross_module` passes
- [ ] Unit tests verify interface discovery without ABC inheritance
- [ ] Integration test verifies cross-module observer pattern detection
- [ ] Performance benchmark shows <10% overhead

## Technical Details

### Implementation Approach

**Two-Phase Analysis**:

**Phase 1: Collect Observer Collections and Their Types**
```rust
// In first pass through class definitions
for class_def in classes {
    // 1. Find observer collections in __init__
    if let Some(init_method) = find_init_method(class_def) {
        for stmt in init_method.body {
            if let Assign { target: self.field_name, value: [] } = stmt {
                if is_observer_collection_name(field_name) {
                    // Record that this class has an observer collection
                    observer_collections.insert((class_name, field_name));
                }
            }
        }
    }

    // 2. Analyze methods that add to observer collections
    for method in class_def.methods {
        for stmt in method.body {
            if let Append { collection: self.field_name, item } = stmt {
                // Use type flow to get type of item
                if let Some(type_id) = type_flow.infer_type(item) {
                    collection_types.insert((class_name, field_name), type_id);
                }
            }
        }
    }
}
```

**Phase 2: Register Interfaces Based on Usage**
```rust
// After type flow analysis is complete
for ((class_name, field_name), type_ids) in collection_types {
    for type_id in type_ids {
        // Register this type as an observer interface
        observer_registry.register_interface(&type_id.name);

        // Find which methods are called on this collection
        for method in find_methods_calling_collection(class_name) {
            for dispatch in find_dispatch_loops(method, field_name) {
                // Register the method as part of the interface
                observer_registry.register_interface_method(
                    &type_id.name,
                    &dispatch.method_name
                );
            }
        }
    }
}
```

### Architecture Changes

**Modified**: `src/analysis/python_type_tracker.rs`

```rust
impl TwoPassExtractor {
    /// Phase 1.5: After type flow tracking, discover observer interfaces
    fn discover_observer_interfaces(&mut self, module: &ast::Module) {
        // 1. Find all observer collections
        let collections = self.find_observer_collections(module);

        // 2. For each collection, get types that flow into it
        for (class_name, field_name) in collections {
            let collection_path = format!("{}.{}", class_name, field_name);
            let types = self.type_flow.get_collection_types(&collection_path);

            // 3. Register these types as observer interfaces
            for type_info in types {
                self.register_observer_interface_from_usage(&type_info);
            }
        }

        // 4. Analyze dispatch loops to find interface methods
        self.analyze_dispatch_loops_for_methods(module);
    }

    /// Register a type as an observer interface based on usage
    fn register_observer_interface_from_usage(&mut self, type_info: &TypeInfo) {
        let registry = Arc::get_mut(&mut self.observer_registry).unwrap();

        // Register the type as an interface
        registry.register_interface(&type_info.type_id.name);

        // Also register base classes as interfaces
        for base_class in &type_info.base_classes {
            registry.register_interface(&base_class.name);
        }
    }

    /// Analyze for loops to discover which methods are part of interfaces
    fn analyze_dispatch_loops_for_methods(&mut self, module: &ast::Module) {
        for class_def in &module.body {
            if let ast::Stmt::ClassDef(class_def) = class_def {
                for method in &class_def.body {
                    if let ast::Stmt::FunctionDef(func_def) = method {
                        self.find_and_register_interface_methods(
                            &class_def.name,
                            func_def
                        );
                    }
                }
            }
        }
    }
}
```

**New Helper Functions**:
```rust
/// Detect if a for loop is an observer dispatch pattern
fn is_observer_dispatch_loop(for_stmt: &ast::StmtFor) -> Option<ObserverDispatchInfo> {
    // Check if iterating over self.field where field is observer collection
    let collection_name = extract_collection_name(&for_stmt.iter)?;

    if !is_observer_collection_name(&collection_name) {
        return None;
    }

    // Extract method calls on the iteration variable
    let target_var = extract_loop_variable(&for_stmt.target)?;
    let method_calls = extract_method_calls(&for_stmt.body, target_var);

    Some(ObserverDispatchInfo {
        collection: collection_name,
        methods: method_calls,
    })
}
```

### Data Structures

```rust
/// Information about an observer collection and its types
#[derive(Debug, Clone)]
pub struct ObserverCollectionInfo {
    /// Class containing the collection
    pub class_name: String,
    /// Field name of the collection
    pub field_name: String,
    /// Types that have been added to this collection
    pub member_types: HashSet<TypeId>,
}

/// Information about a dispatch loop
#[derive(Debug, Clone)]
pub struct ObserverDispatchInfo {
    /// Collection being iterated
    pub collection: String,
    /// Methods called on each item
    pub methods: Vec<String>,
}
```

### Key Algorithms

**Algorithm 1: Observer Collection Discovery**
```
Input: Class definition
Output: Set of (class_name, field_name) for observer collections

1. Find __init__ method in class
2. For each assignment in __init__:
   a. If target is self.field_name
   b. If field_name matches observer pattern (observers, listeners, etc.)
   c. Record (class_name, field_name)
3. Return recorded collections
```

**Algorithm 2: Type-to-Collection Mapping**
```
Input: Observer collection (class, field), Type flow tracker
Output: Set of types in collection

1. Normalize collection name: "Class.field"
2. Query type flow tracker for all types added to collection
3. For each type:
   a. Get TypeInfo including base classes
   b. Add type and all base classes to result set
4. Return result set
```

**Algorithm 3: Interface Method Discovery**
```
Input: Dispatch loop, Type flow tracker
Output: Interface method registrations

1. Extract collection being iterated (e.g., "self.observers")
2. Extract method calls on loop variable (e.g., "update")
3. Get types in collection from type flow tracker
4. For each type:
   a. For each method called:
      i. Register type.method as interface method
      ii. Find all implementations of type.method
      iii. Register implementations in observer registry
```

### APIs and Interfaces

**Enhanced ObserverRegistry API**:
```rust
impl ObserverRegistry {
    /// Register a class as an observer interface (usage-based)
    pub fn register_interface_from_usage(&mut self, interface: &str, evidence: UsageEvidence);

    /// Record evidence that a type is used as an interface
    pub fn add_usage_evidence(&mut self, type_name: &str, evidence: UsageEvidence);

    /// Get confidence score for a type being an interface
    pub fn get_interface_confidence(&self, type_name: &str) -> f32;
}

#[derive(Debug, Clone)]
pub enum UsageEvidence {
    /// Type stored in observer collection
    InObserverCollection { collection: String, class: String },

    /// Type used as base class
    UsedAsBaseClass { derived_class: String },

    /// Methods called in dispatch loop
    DispatchedInLoop { method: String, loop_location: Location },

    /// Inherits from ABC (highest confidence)
    InheritsFromABC,
}
```

## Dependencies

**Prerequisites**:
- **Spec 127**: Type Flow Tracking Infrastructure (required)

**Affected Components**:
- `src/analysis/python_type_tracker.rs` - Add interface discovery phase
- `src/analysis/python_call_graph/observer_registry.rs` - Add usage-based methods
- `src/analysis/python_call_graph/observer_dispatch.rs` - Use discovered interfaces

**External Dependencies**: None

## Testing Strategy

### Unit Tests

**File**: `src/analysis/python_type_tracker.rs`

```rust
#[cfg(test)]
mod observer_discovery_tests {
    #[test]
    fn test_discover_observer_without_abc() {
        let code = r#"
class Observer:
    def update(self, event):
        pass

class Subject:
    def __init__(self):
        self.observers = []

    def attach(self, observer):
        self.observers.append(observer)

    def notify(self, event):
        for obs in self.observers:
            obs.update(event)
"#;
        let mut extractor = TwoPassExtractor::new_with_source(
            PathBuf::from("test.py"),
            code
        );

        let module = parse(code).unwrap();
        extractor.extract(&module);

        // Observer should be registered as interface (no ABC!)
        let registry = extractor.observer_registry;
        assert!(registry.is_interface("Observer"));
    }

    #[test]
    fn test_register_interface_methods_from_dispatch() {
        let code = r#"
class EventHandler:
    def on_start(self): pass
    def on_stop(self): pass

class Manager:
    def __init__(self):
        self.handlers = []

    def trigger(self):
        for h in self.handlers:
            h.on_start()
            h.on_stop()
"#;
        let mut extractor = TwoPassExtractor::new_with_source(
            PathBuf::from("test.py"),
            code
        );

        let module = parse(code).unwrap();
        extractor.extract(&module);

        // EventHandler should be interface with on_start and on_stop methods
        let registry = extractor.observer_registry;
        assert!(registry.is_interface("EventHandler"));
        assert!(registry.has_interface_method("EventHandler", "on_start"));
        assert!(registry.has_interface_method("EventHandler", "on_stop"));
    }

    #[test]
    fn test_base_class_registered_as_interface() {
        let code = r#"
class BaseObserver:
    def notify(self): pass

class ConcreteObserver(BaseObserver):
    def notify(self): print("notified")

class Subject:
    def __init__(self):
        self.observers = []

    def add(self, obs):
        self.observers.append(obs)  # ConcreteObserver added

    def trigger(self):
        for o in self.observers:
            o.notify()
"#;
        let mut extractor = TwoPassExtractor::new_with_source(
            PathBuf::from("test.py"),
            code
        );

        let module = parse(code).unwrap();
        extractor.extract(&module);

        let registry = extractor.observer_registry;

        // BaseObserver should be registered (base of ConcreteObserver)
        assert!(registry.is_interface("BaseObserver"));
        assert!(registry.is_interface("ConcreteObserver"));
    }
}
```

### Integration Tests

**File**: `tests/python_cross_module_test.rs`

```rust
#[test]
fn test_observer_pattern_cross_module() {
    // This is the currently ignored test - should now pass!
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
```

### Performance Tests

```rust
#[test]
fn test_observer_discovery_performance() {
    // Generate code with 100 classes, 50 observer collections
    let code = generate_large_observer_pattern(100, 50);

    let start = Instant::now();
    let mut extractor = TwoPassExtractor::new_with_source(
        PathBuf::from("test.py"),
        &code
    );
    let module = parse(&code).unwrap();
    extractor.extract(&module);
    let duration = start.elapsed();

    // Should complete in reasonable time (baseline + <10%)
    assert!(duration < Duration::from_millis(500));
}
```

## Documentation Requirements

### Code Documentation

1. **Module-level docs** for observer discovery:
```rust
//! Observer Interface Discovery
//!
//! Discovers observer pattern interfaces through usage analysis rather than
//! requiring explicit ABC inheritance. The discovery process:
//!
//! 1. Identifies observer collections (self.observers, self.listeners, etc.)
//! 2. Tracks what types flow into these collections (via type flow tracker)
//! 3. Registers those types as observer interfaces
//! 4. Analyzes dispatch loops to determine interface methods
//!
//! # Example
//!
//! ```python
//! class Observer:  # No ABC inheritance!
//!     def update(self): pass
//!
//! class Subject:
//!     def __init__(self):
//!         self.observers = []  # Observer collection detected
//!
//!     def notify(self):
//!         for obs in self.observers:  # Dispatch loop detected
//!             obs.update()  # update() registered as interface method
//! ```
```

2. **Function documentation** with examples for:
   - `discover_observer_interfaces()`
   - `register_observer_interface_from_usage()`
   - `analyze_dispatch_loops_for_methods()`

### Architecture Updates

**Update**: `ARCHITECTURE.md`

Add section:
```markdown
### Observer Pattern Detection

Debtmap detects observer patterns through **usage-based analysis** rather than
requiring explicit interface declarations. The detection process:

1. **Collection Detection**: Identify fields with observer-like names
   (`observers`, `listeners`, `handlers`, etc.)

2. **Type Flow Analysis**: Use type flow tracker (spec 127) to determine
   what types are stored in observer collections

3. **Interface Registration**: Register types found in observer collections
   as observer interfaces, including their base classes

4. **Method Discovery**: Analyze dispatch loops (`for x in collection: x.method()`)
   to determine which methods are part of the observer interface

5. **Implementation Mapping**: Connect interface methods to concrete
   implementations across modules

This approach handles duck-typed interfaces common in Python without requiring
ABC inheritance, significantly reducing false positives in dead code detection.
```

## Implementation Notes

### Phased Integration

**Phase 1.5 Position**:
The observer interface discovery runs between existing phases:
1. Phase 1: Collect functions and basic type info
2. **Phase 1.5: Discover observer interfaces** ← NEW
3. Phase 2: Register observer implementations
4. Phase 3: Resolve observer dispatches

This ordering ensures type flow data is available before interface discovery.

### Confidence Scoring

Track evidence strength for interface detection:
```rust
fn calculate_interface_confidence(evidence: &[UsageEvidence]) -> f32 {
    let mut score = 0.0;

    for ev in evidence {
        score += match ev {
            UsageEvidence::InheritsFromABC => 1.0,  // Definitive
            UsageEvidence::InObserverCollection { .. } => 0.7,  // Strong
            UsageEvidence::DispatchedInLoop { .. } => 0.8,  // Very strong
            UsageEvidence::UsedAsBaseClass { .. } => 0.5,  // Moderate
        };
    }

    score.clamp(0.0, 1.0)
}
```

Use confidence to filter low-quality detections (threshold: 0.6).

### Edge Cases

1. **Empty collections**: `self.observers = []` with no appends → No types detected, no interface registered
2. **Multiple base classes**: Register all bases as potential interfaces
3. **Conditional dispatch**: `if condition: for x in observers: x.method()` → Still detect
4. **Method name collisions**: Different collections calling same method name → Track per-collection

### Performance Optimization

1. **Cache collection analysis**: Don't re-analyze same collection multiple times
2. **Lazy interface registration**: Only register when dispatch loop found
3. **Prune low-confidence interfaces**: Remove interfaces with confidence < 0.5

## Migration and Compatibility

**Breaking Changes**: None

**Compatibility**:
- Existing ABC-based detection continues unchanged
- New usage-based detection is additive
- Higher confidence for ABC-based interfaces
- Both detection methods contribute to same registry

**Migration Path**:
1. Spec 127 provides type flow infrastructure
2. This spec adds interface discovery
3. Spec 129 adds cross-module resolution
4. Remove `#[ignore]` from `test_observer_pattern_cross_module`

**Success Metrics**:
- Test `test_observer_pattern_cross_module` passes
- No regressions in existing observer pattern tests
- Performance overhead < 10%
- False positive rate < 10% for observer detection
