---
number: 117
title: Observer Pattern Call Graph Detection
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-20
---

# Specification 117: Observer Pattern Call Graph Detection

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently has partial support for observer pattern detection but fails to integrate pattern recognition with call graph construction. This causes false positives in dead code detection for observer pattern implementations.

**Real-World Impact from promptconstruct-frontend Analysis**:

```python
# conversation_manager.py
class ConversationObserver(ABC):
    @abstractmethod
    def on_message_added(self, message, index):
        pass

class ConversationManager:
    def add_message(self, message):
        self.conversation.add_message(message)
        # ❌ This loop creates no call edges
        for observer in self.observers:
            observer.on_message_added(message, index)

# conversation_panel.py
class ConversationPanel(wx.Panel, ConversationObserver):
    def on_message_added(self, message, index):
        # ⚠️ Flagged as "no callers detected - may be dead code"
        pass
```

**Current Behavior**:
- Pattern recognition correctly identifies `ConversationPanel` implements `ConversationObserver`
- Pattern recognition correctly identifies `on_message_added` as an observer method
- Call graph builder does NOT create edge: `ConversationManager.add_message → ConversationPanel.on_message_added`
- Result: Observer implementations flagged as dead code (false positive)

**What Works**:
- ✅ Event binding detection: `obj.Bind(wx.EVT_KEY_DOWN, self.on_key_down)`
- ✅ Observer pattern recognition: Identifies ABC interfaces and implementations
- ✅ Callback tracking: Handles decorators, lambdas, partial functions

**What's Missing**:
- ❌ Observer iteration loop call edge creation
- ❌ Integration between pattern recognition and call graph construction
- ❌ Abstract method dispatch tracking
- ❌ Observer registry tracking across class hierarchies

**Root Cause**:
Pattern recognition layer (`src/analysis/patterns/observer.rs`) and call graph construction layer (`src/analysis/python_call_graph/`) operate independently without communication. Observer dispatch happens through dynamic iteration patterns that aren't currently tracked.

## Objective

Integrate observer pattern recognition with call graph construction to detect observer dispatch patterns and create appropriate call edges, eliminating false positives for observer pattern implementations.

## Requirements

### Functional Requirements

**FR1: Observer Loop Detection**
- Detect `for observer in self.observers:` iteration patterns
- Identify method calls on iteration variables
- Support common observer collection names: `observers`, `listeners`, `callbacks`, `handlers`, `subscribers`, `watchers`
- Handle both direct iteration and indirect iteration (e.g., via helper methods)

**FR2: Observer Registry Tracking**
- Track observer collections at class level (e.g., `self.observers = []`)
- Maintain mapping of observer collections to their interface types
- Support observer registration methods (e.g., `register_observer`, `add_listener`)
- Track observer implementations across class hierarchies

**FR3: Abstract Method Dispatch**
- Detect calls to abstract interface methods
- Find all concrete implementations of abstract methods
- Create call edges from dispatch site to all implementations
- Distinguish between single implementation and multiple implementations

**FR4: Call Graph Integration**
- Add new `CallType::ObserverDispatch` variant
- Create call edges from observer notification loops to implementations
- Propagate confidence scores based on pattern strength
- Integrate with existing call graph construction pipeline

**FR5: Cross-File Observer Detection**
- Track observer registrations across module boundaries
- Handle cases where observer interface and implementations are in different files
- Support dynamic observer registration at runtime
- Maintain observer registry in cross-module context

### Non-Functional Requirements

**NFR1: Performance**
- Observer loop detection should add < 5% overhead to call graph construction
- Observer registry should use O(1) lookup for common operations
- Pattern detection should be lazy and cached where possible

**NFR2: Accuracy**
- Reduce false positive rate for observer patterns by > 80%
- Maintain existing true positive detection rate
- Provide confidence scores for observer dispatch edges (0.7-0.95 range)

**NFR3: Maintainability**
- Observer detection logic should be modular and testable
- Clear separation between pattern recognition and call graph integration
- Well-documented APIs for extending observer pattern support

**NFR4: Compatibility**
- Maintain backward compatibility with existing call graph API
- Preserve existing event binding detection behavior
- Support existing callback tracking mechanisms

## Acceptance Criteria

- [ ] **AC1**: Detect `for observer in self.observers: observer.method()` patterns and create call edges
- [ ] **AC2**: Track observer collections at class level with type information
- [ ] **AC3**: Create call edges from notification loops to all concrete implementations
- [ ] **AC4**: Add `CallType::ObserverDispatch` with confidence scoring (0.7-0.95)
- [ ] **AC5**: Support standard observer collection names: observers, listeners, callbacks, handlers, subscribers, watchers
- [ ] **AC6**: Handle inline notification (not just dedicated `notify_all()` methods)
- [ ] **AC7**: Integrate with existing observer pattern recognition in `patterns/observer.rs`
- [ ] **AC8**: Pass test: `ConversationPanel.on_message_added` shows caller `ConversationManager.add_message`
- [ ] **AC9**: Reduce false positive rate for observer patterns by > 80% (measured on promptconstruct-frontend)
- [ ] **AC10**: Maintain 100% of existing true positive detections for event bindings
- [ ] **AC11**: Documentation: Update ARCHITECTURE.md with observer dispatch flow
- [ ] **AC12**: Documentation: Add examples to pattern detection guide

## Technical Details

### Implementation Approach

**Phase 1: Observer Registry Infrastructure** (Day 1)

1. **Create Observer Registry** (`src/analysis/python_call_graph/observer_registry.rs`):
   ```rust
   pub struct ObserverRegistry {
       // Class -> Collection name -> Observer interface type
       collections: HashMap<String, HashMap<String, String>>,
       // Observer interface -> List of implementations
       implementations: HashMap<String, Vec<FunctionId>>,
   }
   ```

2. **Track Observer Collections**:
   - Detect `self.observers = []` in `__init__` methods
   - Track `register_observer(self, observer)` method signatures
   - Store collection name and inferred interface type

3. **Integration with Pattern Recognition**:
   - Consume `PatternInstance` from `ObserverPatternRecognizer`
   - Build implementation mapping from pattern detection results
   - Store in observer registry for call graph use

**Phase 2: Observer Loop Detection** (Day 2)

1. **Create Observer Loop Detector** (`src/analysis/python_call_graph/observer_dispatch.rs`):
   ```rust
   pub struct ObserverDispatchDetector {
       registry: Arc<ObserverRegistry>,
   }

   impl ObserverDispatchDetector {
       pub fn detect_observer_dispatch(
           &self,
           for_stmt: &ast::StmtFor,
           current_class: Option<&str>,
       ) -> Vec<ObserverDispatch> {
           // 1. Check if iterating over known observer collection
           // 2. Find method calls on iteration variable
           // 3. Look up implementations from registry
           // 4. Return dispatch information
       }
   }
   ```

2. **Observer Collection Detection**:
   ```rust
   fn is_observer_collection(target: &ast::Expr, class: Option<&str>) -> bool {
       // Check self.observers, self.listeners, etc.
       // Verify against observer registry
   }
   ```

3. **Method Call Extraction**:
   ```rust
   fn extract_dispatch_calls(for_body: &[ast::Stmt]) -> Vec<MethodCall> {
       // Find all method calls on iteration variable
       // Handle both simple and nested cases
   }
   ```

**Phase 3: Call Graph Integration** (Day 3)

1. **Extend CallType**:
   ```rust
   pub enum CallType {
       Direct,
       Callback,
       ObserverDispatch, // NEW
       // ... existing variants
   }
   ```

2. **Create Observer Dispatch Edges**:
   ```rust
   fn create_observer_edges(
       dispatch: &ObserverDispatch,
       registry: &ObserverRegistry,
       call_graph: &mut CallGraph,
   ) {
       let implementations = registry.get_implementations(&dispatch.method_name);
       for impl_id in implementations {
           call_graph.add_call(FunctionCall {
               caller: dispatch.caller_id.clone(),
               callee: impl_id.clone(),
               call_type: CallType::ObserverDispatch,
           });
       }
   }
   ```

3. **Integrate with TwoPassExtractor**:
   - Add observer registry to `TwoPassExtractor`
   - Populate registry during first pass
   - Detect observer dispatch during second pass
   - Create call edges during graph construction

**Phase 4: Testing and Validation** (Day 4)

1. **Unit Tests**:
   - Observer registry operations
   - Observer loop detection
   - Call edge creation
   - Confidence scoring

2. **Integration Tests**:
   - Full observer pattern with multiple implementations
   - Cross-file observer detection
   - Mixed patterns (observers + event bindings)
   - Real-world code from promptconstruct-frontend

3. **Regression Tests**:
   - Existing event binding tests still pass
   - No degradation in callback tracking
   - Performance benchmarks within limits

### Architecture Changes

**New Modules**:
- `src/analysis/python_call_graph/observer_registry.rs` - Observer tracking
- `src/analysis/python_call_graph/observer_dispatch.rs` - Loop detection
- `tests/python_observer_pattern_integration_test.rs` - Integration tests

**Modified Modules**:
- `src/analysis/python_type_tracker.rs` - Add observer registry
- `src/analysis/python_call_graph/call_analysis.rs` - Detect observer loops
- `src/priority/call_graph/mod.rs` - Add `CallType::ObserverDispatch`
- `src/analysis/patterns/observer.rs` - Export implementation mappings

**Integration Points**:
```
Pattern Recognition (observer.rs)
        ↓
    PatternInstance
        ↓
Observer Registry (observer_registry.rs)
        ↓
Two-Pass Extractor (python_type_tracker.rs)
        ↓
Observer Dispatch Detector (observer_dispatch.rs)
        ↓
Call Graph Builder (call_analysis.rs)
        ↓
    CallGraph with ObserverDispatch edges
```

### Data Structures

**ObserverRegistry**:
```rust
pub struct ObserverRegistry {
    // Class name -> Collection field name -> Observer interface type
    collections: HashMap<String, HashMap<String, String>>,

    // Observer interface -> List of implementation function IDs
    implementations: HashMap<String, Vec<FunctionId>>,

    // Helper lookups for fast access
    class_to_interface: HashMap<String, String>, // Implementation class -> Interface
}
```

**ObserverDispatch**:
```rust
pub struct ObserverDispatch {
    pub caller_id: FunctionId,
    pub method_name: String,
    pub observer_interface: Option<String>,
    pub collection_expr: String,
    pub confidence: f32,
}
```

**CallType Extension**:
```rust
pub enum CallType {
    Direct,
    Callback,
    ObserverDispatch, // NEW: Dynamic dispatch through observer pattern
    // ... existing variants
}
```

### APIs and Interfaces

**Observer Registry API**:
```rust
impl ObserverRegistry {
    pub fn new() -> Self;
    pub fn register_collection(&mut self, class: &str, field: &str, interface: &str);
    pub fn register_implementation(&mut self, interface: &str, impl_id: FunctionId);
    pub fn get_implementations(&self, method_name: &str) -> Vec<&FunctionId>;
    pub fn is_observer_collection(&self, class: &str, field: &str) -> bool;
}
```

**Observer Dispatch Detector API**:
```rust
impl ObserverDispatchDetector {
    pub fn new(registry: Arc<ObserverRegistry>) -> Self;
    pub fn detect_in_for_loop(&self, for_stmt: &ast::StmtFor, context: &AnalysisContext)
        -> Vec<ObserverDispatch>;
    pub fn calculate_confidence(&self, dispatch: &ObserverDispatch) -> f32;
}
```

**Integration with TwoPassExtractor**:
```rust
impl TwoPassExtractor {
    pub fn new_with_observer_registry(
        file_path: PathBuf,
        source: &str,
        registry: Arc<ObserverRegistry>,
    ) -> Self;
}
```

## Dependencies

**Prerequisites**: None (standalone feature)

**Affected Components**:
- `src/analysis/patterns/observer.rs` - Export implementation data
- `src/analysis/python_type_tracker.rs` - Integrate observer registry
- `src/analysis/python_call_graph/call_analysis.rs` - Detect observer loops
- `src/priority/call_graph/mod.rs` - Add new call type
- `src/builders/call_graph.rs` - Handle observer dispatch edges

**External Dependencies**: None (uses existing AST parsing and pattern recognition)

## Testing Strategy

### Unit Tests

**Observer Registry Tests** (`src/analysis/python_call_graph/observer_registry.rs`):
```rust
#[test]
fn test_register_and_lookup_collection() {
    let mut registry = ObserverRegistry::new();
    registry.register_collection("Manager", "observers", "Observer");
    assert!(registry.is_observer_collection("Manager", "observers"));
}

#[test]
fn test_register_and_get_implementations() {
    let mut registry = ObserverRegistry::new();
    let func_id = FunctionId { name: "ConcreteObserver.on_event", ... };
    registry.register_implementation("Observer", func_id);

    let impls = registry.get_implementations("on_event");
    assert_eq!(impls.len(), 1);
}
```

**Observer Loop Detection Tests** (`src/analysis/python_call_graph/observer_dispatch.rs`):
```rust
#[test]
fn test_detect_simple_observer_loop() {
    let python_code = r#"
for observer in self.observers:
    observer.on_event()
"#;
    // Parse and detect observer dispatch
    assert!(dispatch_detected);
}

#[test]
fn test_detect_inline_notification() {
    let python_code = r#"
def add_item(self, item):
    self.items.append(item)
    for listener in self.listeners:
        listener.on_item_added(item)
"#;
    // Should detect inline observer notification
    assert!(dispatch_detected);
}
```

### Integration Tests

**Full Observer Pattern Test** (`tests/python_observer_pattern_integration_test.rs`):
```rust
#[test]
fn test_full_observer_pattern_call_graph() {
    let python_code = r#"
from abc import ABC, abstractmethod

class Observer(ABC):
    @abstractmethod
    def on_event(self):
        pass

class Subject:
    def __init__(self):
        self.observers = []

    def notify(self):
        for observer in self.observers:
            observer.on_event()

class ConcreteObserver(Observer):
    def on_event(self):
        print("Event received")
"#;

    let call_graph = analyze_python_code(python_code);

    // Verify call edge exists
    let on_event = call_graph.find_function("ConcreteObserver.on_event");
    let callers = call_graph.get_callers(&on_event);

    assert_eq!(callers.len(), 1);
    assert_eq!(callers[0].name, "Subject.notify");
}
```

**promptconstruct-frontend Validation** (`tests/python_observer_false_positive_validation.rs`):
```rust
#[test]
fn test_conversation_observer_pattern() {
    // Use actual code from promptconstruct-frontend
    let conversation_manager = read_file("../promptconstruct-frontend/promptconstruct/client/conversation_manager.py");
    let conversation_panel = read_file("../promptconstruct-frontend/promptconstruct/client/conversation_panel.py");

    let call_graph = analyze_python_project(&[conversation_manager, conversation_panel]);

    // Verify ConversationPanel.on_message_added has callers
    let on_message_added = call_graph.find_function("ConversationPanel.on_message_added");
    assert!(!call_graph.get_callers(&on_message_added).is_empty());
}
```

### Performance Tests

**Benchmarks** (`benches/observer_pattern_bench.rs`):
```rust
#[bench]
fn bench_observer_registry_lookup(b: &mut Bencher) {
    // Measure registry lookup performance
}

#[bench]
fn bench_observer_loop_detection(b: &mut Bencher) {
    // Measure observer loop detection overhead
}

#[bench]
fn bench_call_graph_with_observer_dispatch(b: &mut Bencher) {
    // Compare call graph construction with/without observer dispatch
}
```

### User Acceptance

**Success Metrics**:
1. Run debtmap on promptconstruct-frontend before and after implementation
2. Count observer pattern false positives before: ~5-10 items
3. Count observer pattern false positives after: < 2 items
4. Verify no new false negatives introduced
5. Check runtime performance impact < 5%

**Validation Commands**:
```bash
# Before
cargo run -- analyze ../promptconstruct-frontend --output before.json

# After (with observer dispatch detection)
cargo run -- analyze ../promptconstruct-frontend --output after.json

# Compare
cargo run -- compare before.json after.json
```

## Documentation Requirements

### Code Documentation

**Module Documentation**:
- `observer_registry.rs`: Comprehensive module docs with usage examples
- `observer_dispatch.rs`: Explain detection algorithm and patterns supported
- Add inline documentation for all public APIs
- Document confidence scoring formula

**API Documentation**:
```rust
/// Observer registry for tracking observer collections and implementations.
///
/// The registry maintains mappings between:
/// - Classes and their observer collection fields
/// - Observer interfaces and concrete implementations
/// - Collection names and their interface types
///
/// # Example
/// ```
/// let mut registry = ObserverRegistry::new();
/// registry.register_collection("Subject", "observers", "Observer");
/// registry.register_implementation("Observer", observer_impl_id);
/// ```
pub struct ObserverRegistry { ... }
```

### User Documentation

**Pattern Detection Guide** (`docs/patterns/observer-pattern.md`):
```markdown
# Observer Pattern Detection

Debtmap detects observer pattern implementations and creates call graph edges
for observer dispatch. This reduces false positives in dead code detection.

## Supported Patterns

### Standard Observer Loop
```python
for observer in self.observers:
    observer.on_event()
```

### Inline Notification
```python
def add_item(self, item):
    self.items.append(item)
    for listener in self.listeners:
        listener.on_item_added(item)
```

### Multiple Implementations
Debtmap tracks all implementations of observer interfaces and creates
call edges to each concrete implementation.
```

**CHANGELOG.md Entry**:
```markdown
## [0.2.9] - 2025-10-20

### Added
- Observer pattern call graph detection for Python code
- New `CallType::ObserverDispatch` for dynamic observer dispatch
- Observer registry for tracking observer collections and implementations
- Support for standard observer collection names: observers, listeners, callbacks, handlers

### Fixed
- False positives for observer pattern implementations (80% reduction)
- Missing call edges for observer notification loops
- Dead code detection for abstract method implementations
```

### Architecture Updates

**ARCHITECTURE.md Section**:
```markdown
## Observer Pattern Detection

### Overview
Debtmap integrates observer pattern recognition with call graph construction
to detect observer dispatch patterns and create appropriate call edges.

### Components
1. **Observer Registry** (`observer_registry.rs`)
   - Tracks observer collections at class level
   - Maintains interface to implementation mappings
   - Provides fast lookup for dispatch detection

2. **Observer Dispatch Detector** (`observer_dispatch.rs`)
   - Detects `for observer in collection:` patterns
   - Identifies method calls on iteration variables
   - Creates observer dispatch information

3. **Call Graph Integration** (`call_analysis.rs`)
   - Consumes observer dispatch information
   - Creates call edges with `CallType::ObserverDispatch`
   - Integrates with existing call graph construction

### Data Flow
```
Pattern Recognition → Observer Registry → Observer Dispatch Detector → Call Graph
```
```

## Implementation Notes

### Edge Cases to Handle

1. **Dynamic Observer Registration**:
   ```python
   observers = [Observer1(), Observer2()]
   for obs in observers:
       subject.register_observer(obs)
   ```
   - Track observer registrations dynamically
   - Update registry during analysis

2. **Conditional Notification**:
   ```python
   for observer in self.observers:
       if condition:
           observer.on_event()
   ```
   - Still create call edges (conservative approach)
   - May reduce confidence score

3. **Nested Loops**:
   ```python
   for observer in self.primary_observers:
       for event in events:
           observer.on_event(event)
   ```
   - Detect observer iteration at any nesting level
   - Focus on outermost observer loop

4. **Method Chaining**:
   ```python
   for observer in self.get_observers():
       observer.on_event()
   ```
   - Resolve method calls to identify collection
   - Track return types for collection inference

### Performance Considerations

**Optimization Strategies**:
1. Lazy initialization of observer registry
2. Cache observer collection lookups
3. Prune registry for out-of-scope classes
4. Use efficient data structures (HashMap, BTreeSet)

**Expected Overhead**:
- Observer registry: O(1) lookup, O(n) initialization
- Loop detection: O(AST nodes) - same as existing traversal
- Call edge creation: O(implementations) per dispatch
- Total: < 5% overhead on overall analysis time

### Confidence Scoring Formula

```rust
fn calculate_observer_dispatch_confidence(dispatch: &ObserverDispatch) -> f32 {
    let mut confidence = 0.85; // Base confidence

    // Higher confidence for known observer collection names
    if is_standard_collection_name(&dispatch.collection_expr) {
        confidence += 0.05;
    }

    // Higher confidence if interface explicitly identified
    if dispatch.observer_interface.is_some() {
        confidence += 0.05;
    }

    // Lower confidence for complex expressions
    if is_complex_collection_expression(&dispatch.collection_expr) {
        confidence -= 0.10;
    }

    confidence.clamp(0.70, 0.95)
}
```

### Potential Pitfalls

1. **Over-eagerness**: Creating edges for non-observer loops
   - Mitigation: Require collection name match or interface type confirmation

2. **Cross-file complexity**: Tracking observers across modules
   - Mitigation: Use cross-module context (already implemented)

3. **Performance regression**: Observer detection slowing analysis
   - Mitigation: Benchmark and optimize hot paths

4. **False negatives**: Missing unconventional observer patterns
   - Mitigation: Start conservative, expand support based on real-world usage

## Migration and Compatibility

### Breaking Changes

None. This is a purely additive feature.

### Backward Compatibility

- Existing call graph API remains unchanged
- New `CallType::ObserverDispatch` is additive
- Existing tests continue to pass
- Configuration options remain compatible

### Migration Path

No migration required. Feature activates automatically on next analysis run.

### Compatibility Considerations

**Python Version Support**:
- Python 2.7: Supported (abstract base classes available)
- Python 3.x: Full support for all features
- Type hints: Optional, used if available for better accuracy

**Framework Support**:
- wxPython: Full support (existing event binding + new observer dispatch)
- PyQt/PySide: Full support (signal/slot + observer patterns)
- Django: Supported (signal system uses observer pattern)
- Flask: Supported (callback patterns)
- Generic Python: Full support for ABC-based observers

### Rollback Plan

If issues arise, observer dispatch detection can be disabled via configuration:

```toml
[analysis]
enable_observer_dispatch_detection = false
```

This would revert behavior to v0.2.8 without code changes.

## Success Criteria Summary

**Primary Goals**:
- ✅ Reduce false positive rate for observer patterns by > 80%
- ✅ Create call edges from observer loops to implementations
- ✅ Maintain existing true positive detection rate
- ✅ Add < 5% performance overhead

**Validation**:
- Run on promptconstruct-frontend and measure false positive reduction
- Run existing test suite (all tests pass)
- Run performance benchmarks (< 5% overhead)
- Manual code review of generated call graph edges

**Timeline**: 4 days
- Day 1: Observer registry infrastructure
- Day 2: Observer loop detection
- Day 3: Call graph integration
- Day 4: Testing and validation
