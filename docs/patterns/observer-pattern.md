# Observer Pattern Detection Guide

## Overview

DebtMap automatically detects observer pattern implementations in Rust code to improve dead code detection accuracy. This guide explains which patterns are detected, how they work, and what to expect from the analysis.

## What is Observer Pattern Detection?

Observer pattern detection identifies code structures where:
1. **Observer Registry**: Collections that store callbacks, handlers, or event listeners
2. **Observer Dispatch**: Loops that iterate over registered observers and invoke them
3. **Dynamic Invocation**: Handlers are called indirectly through the collection

This helps DebtMap understand that callback functions are actually used, even though they're not directly called in the code.

## Supported Observer Patterns

### 1. Standard Observer Loop

The most common pattern: iterate over a collection of handlers and call a method on each.

```rust
pub struct EventBus {
    listeners: Vec<Box<dyn EventHandler>>,  // ← Registry detected
}

impl EventBus {
    pub fn notify(&self, event: &Event) {   // ← Dispatcher detected
        for listener in &self.listeners {    // ← Loop over registry
            listener.handle(event);          // ← Dispatch call
        }
    }
}
```

**Detection Behavior**:
- `listeners` field recognized as observer registry (type: `Vec<Box<dyn Trait>>`)
- `notify()` method marked as observer dispatcher
- Any function implementing `EventHandler` is considered reachable via dispatch

**Why This Matters**: Without detection, `handle()` implementations might be flagged as dead code if never called directly.

### 2. Inline Notification with Filter

Observers with filtering or transformation before dispatch.

```rust
pub struct SelectiveNotifier {
    callbacks: Vec<Box<dyn Fn(&Event) -> bool>>,  // ← Registry
}

impl SelectiveNotifier {
    pub fn notify_matching(&self, event: &Event, predicate: impl Fn(&Event) -> bool) {
        for callback in self.callbacks.iter().filter(|cb| predicate(event)) {  // ← Filtered loop
            callback(event);  // ← Dispatch
        }
    }
}
```

**Detection Behavior**:
- Handles iterator chains (`.iter()`, `.filter()`, etc.)
- Recognizes closure invocation as dispatch
- Works with inline transformations

### 3. HashMap-Based Event Dispatch

Multiple event types with different handlers for each.

```rust
pub struct EventDispatcher {
    handlers: HashMap<EventType, Vec<Box<dyn EventHandler>>>,  // ← Registry
}

impl EventDispatcher {
    pub fn dispatch(&self, event_type: EventType, event: &Event) {
        if let Some(handlers) = self.handlers.get(&event_type) {
            for handler in handlers {  // ← Nested loop detected
                handler.handle(event);  // ← Dispatch
            }
        }
    }
}
```

**Detection Behavior**:
- Recognizes `HashMap<K, Vec<Handler>>` pattern
- Detects nested loops (outer: hash lookup, inner: handler iteration)
- Works with conditional handler retrieval

### 4. Multiple Observer Implementations

Single dispatcher calling multiple types of observers.

```rust
pub struct MultiEventBus {
    on_start_handlers: Vec<Box<dyn StartHandler>>,   // ← Registry 1
    on_stop_handlers: Vec<Box<dyn StopHandler>>,     // ← Registry 2
}

impl MultiEventBus {
    pub fn notify_start(&self) {
        for handler in &self.on_start_handlers {  // ← Dispatcher 1
            handler.on_start();
        }
    }

    pub fn notify_stop(&self) {
        for handler in &self.on_stop_handlers {   // ← Dispatcher 2
            handler.on_stop();
        }
    }
}
```

**Detection Behavior**:
- Each field recognized as separate observer registry
- Multiple dispatch functions detected independently
- Handlers grouped by event type

### 5. Class Hierarchy with Inherited Registry

Observer registry in base class, dispatch in derived class.

```rust
pub struct BaseEventBus {
    listeners: Vec<Box<dyn Listener>>,  // ← Registry in base
}

pub struct DerivedEventBus {
    base: BaseEventBus,  // ← Inherited field
    additional_state: State,
}

impl DerivedEventBus {
    pub fn notify(&self) {
        for listener in &self.base.listeners {  // ← Detected via field access
            listener.on_event();
        }
    }
}
```

**Detection Behavior**:
- Tracks field access chains: `self.base.listeners`
- Matches against observer registries through indirection
- Supports multiple levels of nesting: `self.inner.dispatcher.handlers`

### 6. Method Reference Dispatch

Handlers stored as method references rather than trait objects.

```rust
pub struct MethodDispatcher {
    handlers: Vec<fn(&Event)>,  // ← Function pointer registry
}

impl MethodDispatcher {
    pub fn dispatch(&self, event: &Event) {
        for handler in &self.handlers {
            handler(event);  // ← Function pointer call
        }
    }
}
```

**Detection Behavior**:
- Recognizes `Vec<fn(...)>` as observer registry
- Detects function pointer invocation
- Works with both free functions and method pointers

## Detection Criteria

### Observer Registry Requirements

A field is recognized as an observer registry if it meets **all** of these criteria:

1. **Collection Type**: Must be a collection (Vec, HashMap, HashSet, etc.)
2. **Element Type**: Contains one of:
   - Trait objects: `Box<dyn Trait>`
   - Function pointers: `fn(...) -> ...`
   - Closures: `Box<dyn Fn(...)>`
3. **Field Name Pattern** (optional boost): Matches common patterns:
   - `listeners`, `handlers`, `observers`, `callbacks`, `subscribers`
   - `on_*_handlers`, `*_callbacks`, `*_listeners`

### Observer Dispatcher Requirements

A function is recognized as an observer dispatcher if it meets **all** of these criteria:

1. **Loop Pattern**: Contains a `for` loop
2. **Registry Reference**: Loop iterates over a field that is an observer registry
3. **Dispatch Call**: Loop body contains method call or function invocation on loop variable
4. **Complete Iteration**: No early `break` (must notify all observers)

## Examples by Language Feature

### With Iterators

```rust
pub fn notify_all(&self) {
    self.listeners
        .iter()                    // ← Iterator chain
        .for_each(|l| l.handle()); // ← Dispatch via for_each
}
```

**Status**: Currently **not detected** (requires functional iterator pattern support)
**Planned**: Spec 117 follow-up for iterator method detection

### With Conditionals

```rust
pub fn notify_if(&self, condition: bool, event: &Event) {
    if condition {
        for listener in &self.listeners {  // ← Detected
            listener.handle(event);
        }
    }
}
```

**Status**: **Detected** - conditional wrapper around loop is supported

### With Async/Await

```rust
pub async fn notify_async(&self, event: Event) {
    for listener in &self.listeners {
        listener.handle_async(&event).await;  // ← Async dispatch
    }
}
```

**Status**: **Detected** - async context doesn't affect pattern recognition

## Impact on Analysis

### Dead Code Detection

**Before Observer Detection**:
```
WARNING: Function 'on_user_login' appears to be dead code (no direct callers)
```

**After Observer Detection**:
```
INFO: Function 'on_user_login' is reachable via observer dispatch in 'UserEventBus::notify'
```

### Call Graph Visualization

Detected dispatcher functions are marked in the call graph:

```
UserEventBus::notify [OBSERVER_DISPATCHER]
  ├─> UserLoginHandler::on_user_login (via dynamic dispatch)
  ├─> EmailNotifier::on_user_login (via dynamic dispatch)
  └─> AnalyticsTracker::on_user_login (via dynamic dispatch)
```

### Complexity Analysis

Observer dispatch loops are recognized as coordination logic and receive adjusted complexity scoring:
- Loop itself: Low complexity (it's just iteration)
- Individual handlers: Analyzed independently

## Configuration

### Customizing Detection

Create or modify `.debtmap.toml`:

```toml
[observer_detection]
# Enable or disable observer pattern detection
enabled = true

# Custom field name patterns to recognize as observer registries
registry_field_patterns = [
    "listeners",
    "handlers",
    "observers",
    "callbacks",
    "subscribers",
    "delegates"
]

# Minimum confidence threshold (0.0-1.0)
min_confidence = 0.8
```

### Verbose Output

Run with `--verbose` to see detection details:

```bash
debtmap analyze --verbose
```

**Output**:
```
OBSERVER PATTERN DETECTION:
  Registry detected: EventBus.listeners (confidence: 1.0)
    Type: Vec<Box<dyn EventHandler>>
    Field name pattern: 'listeners' (matched)

  Dispatcher detected: EventBus::notify (confidence: 1.0)
    Iterates over: self.listeners
    Dispatch call: listener.handle(event)
    Call graph updated: 3 handlers marked as reachable
```

## Limitations

### Current Limitations

1. **Iterator Methods Not Detected**: Patterns using `.for_each()`, `.map()` are not yet recognized
2. **Rust-Only**: Python and TypeScript detection planned but not implemented
3. **Single Module**: Cross-module observer registration tracking not supported
4. **No Confidence Scoring**: All detections are binary (yes/no), not probabilistic

### False Negatives (Patterns We Miss)

**Functional Style**:
```rust
// NOT DETECTED (yet)
self.listeners.iter().for_each(|l| l.handle());
```

**Trait-Based Dispatch**:
```rust
// NOT DETECTED (requires type system analysis)
fn notify<T: EventHandler>(handlers: &[T]) {
    for handler in handlers {
        handler.handle();
    }
}
```

**Macro-Generated Observers**:
```rust
// NOT DETECTED (macros expanded before analysis)
dispatch_all!(self.listeners, handle, event);
```

### False Positives (Rare)

**Generic Iteration Over Non-Observers**:
```rust
// Might be incorrectly detected if field name matches pattern
pub struct DataProcessor {
    handlers: Vec<DataTransform>,  // Not really observers
}

impl DataProcessor {
    pub fn process(&self, data: Data) -> Data {
        for handler in &self.handlers {
            data = handler.transform(data);  // Sequential processing, not dispatch
        }
    }
}
```

**Mitigation**: Use semantic field names (`transforms`, `processors`) or disable detection for specific modules.

## Best Practices

### For Detection Accuracy

1. **Use Conventional Names**: Name observer collections with standard patterns:
   - `listeners`, `handlers`, `observers`, `callbacks`

2. **Explicit Types**: Prefer explicit trait objects over generic bounds:
   - Good: `Vec<Box<dyn EventHandler>>`
   - Less detectable: `Vec<T> where T: EventHandler`

3. **Simple Loops**: Use straightforward `for` loops for dispatch:
   - Good: `for handler in &self.listeners { ... }`
   - Less detectable: `self.listeners.iter().for_each(...)`

### For Maintainability

1. **Document Observer Intent**: Add comments to clarify observer patterns:
   ```rust
   /// Observers registered for user events
   pub listeners: Vec<Box<dyn UserEventListener>>,
   ```

2. **Single Responsibility**: Keep dispatch functions focused:
   ```rust
   // Good: One loop, one purpose
   pub fn notify(&self, event: &Event) {
       for listener in &self.listeners {
           listener.handle(event);
       }
   }
   ```

3. **Test Coverage**: Write tests that exercise dispatch paths:
   ```rust
   #[test]
   fn test_notify_calls_all_listeners() {
       let bus = EventBus::new();
       bus.add_listener(Box::new(MockListener::new()));
       bus.notify(&Event::new());
       // Assert listener was called
   }
   ```

## Troubleshooting

### Pattern Not Detected

**Check**:
1. Is the collection field using a standard name? (`listeners`, `handlers`, etc.)
2. Is the element type a trait object or function pointer?
3. Does the dispatch function use a `for` loop (not `.for_each()`)?
4. Run with `--verbose` to see detection output

**Workarounds**:
1. Rename field to match standard patterns
2. Add explicit type annotations
3. Refactor to use `for` loop instead of iterator methods

### False Positive Detection

**Check**:
1. Is the function really dispatching to observers?
2. Is the collection semantically a registry of handlers?

**Workarounds**:
1. Rename field to avoid observer patterns (`transforms` vs `handlers`)
2. Disable observer detection for specific modules in `.debtmap.toml`
3. File an issue with the false positive example

## Future Enhancements

Planned improvements for observer pattern detection:

1. **Functional Iterator Support**: Detect `.for_each()`, `.map()` dispatch patterns
2. **Multi-Language**: Extend to Python (decorators) and TypeScript (event emitters)
3. **Cross-Module Tracking**: Follow observer registration across module boundaries
4. **Confidence Scoring**: Probabilistic detection with adjustable thresholds
5. **Generic Trait Bounds**: Support `Vec<T> where T: Handler` patterns
6. **Macro Expansion**: Analyze macros that generate observer patterns

## Related Documentation

- [ARCHITECTURE.md](../../ARCHITECTURE.md#observer-pattern-detection) - Observer pattern architecture
- [Spec 117](../../specs/117-observer-pattern-detection.md) - Original specification
- [Call Graph Analysis](../analysis/call-graph.md) - How detection integrates with call graphs
- [Dead Code Detection](../analysis/dead-code.md) - Impact on dead code analysis

## Examples Repository

See the test suite for comprehensive examples:
- `tests/observer_dispatch_detection.rs` - Standard patterns
- `tests/observer_class_hierarchy.rs` - Inheritance scenarios
- `tests/integration/observer_full_pipeline.rs` - End-to-end examples

## Questions or Issues?

If you encounter unexpected behavior with observer pattern detection:

1. Run with `--verbose` to see what was detected
2. Check if your pattern matches the supported patterns above
3. File an issue with a minimal reproduction case
4. Consider workarounds from the Troubleshooting section

The detection system is actively developed and your feedback helps improve accuracy!
