---
number: 111
title: Design Pattern Recognition
category: foundation
priority: high
status: draft
dependencies: [109]
created: 2025-10-16
---

# Specification 111: Design Pattern Recognition

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 109 (Cross-File Dependency Analysis)

## Context

Debtmap v0.2.8 fails to recognize common software design patterns, leading to false positives when pattern-based code appears unused within individual files but is actually invoked through pattern mechanisms.

**Real-World Impact from Bug Report**:
- **False Positive #5**: `ConversationPanel.on_message_added()` flagged as dead code
  - Implements `ConversationObserver` interface (lines 11-20)
  - Called via observer pattern by `ConversationManager` (lines 136-137)
  - Debtmap missed the `for observer in self.observers: observer.on_message_added()` pattern
- **False Positive #10**: `ConversationManager.add_message()` flagged as dead code
  - Used via singleton pattern (module-level `manager` instance)
  - Imported and called from other modules
- **Impact**: 20% of false positives caused by unrecognized design patterns

**Current Behavior**:
```python
# conversation_manager.py (lines 11-20)
class ConversationObserver(ABC):
    @abstractmethod
    def on_message_added(self, message, index):
        pass

# conversation_panel.py
class ConversationPanel(ConversationObserver):
    def on_message_added(self, message, index):  # ❌ Flagged as dead code
        """Handle new message added to conversation."""
        # Implementation...

# conversation_manager.py (lines 136-137)
for observer in self.observers:
    observer.on_message_added(message, index)  # ✅ Actually calls it!
```

**What Debtmap Currently Sees**:
- `on_message_added()` has no direct callers in same file → mark as dead code
- Misses that it implements abstract method from base class
- Misses that it's called polymorphically via `observer.on_message_added()`

**Why This is Critical**:
- Observer pattern is ubiquitous in event-driven systems
- Singleton pattern is common for managers and services
- Factory patterns create objects dynamically
- Strategy pattern injects behavior at runtime
- Removing pattern implementations breaks applications

## Objective

Implement design pattern recognition to detect when functions/methods are invoked through pattern mechanisms rather than direct calls, eliminating pattern-based false positives and reducing overall false positive rate from 20% to < 2%.

## Requirements

### Functional Requirements

1. **Observer Pattern Detection**
   - Identify abstract base classes defining observer interfaces
   - Detect concrete observer implementations
   - Track observer registration (`.add_observer()`, `.register()`)
   - Recognize observer invocation loops (`for obs in observers: obs.method()`)
   - Mark observer interface methods as "used via pattern"

2. **Singleton Pattern Detection**
   - Detect module-level class instantiation
   - Track singleton instance exports
   - Follow singleton usage across files (depends on Spec 109)
   - Recognize common singleton patterns (module-level, `__new__`, decorator)

3. **Factory Pattern Detection**
   - Identify factory methods/functions
   - Track class instantiation via factories
   - Detect factory registration dictionaries (`{"type": ClassA}`)
   - Recognize abstract factory implementations

4. **Strategy Pattern Detection**
   - Identify strategy interfaces/protocols
   - Detect strategy implementations
   - Track strategy injection (constructor, setter, decorator)
   - Mark strategy methods as "used via pattern"

5. **Dependency Injection Detection**
   - Detect constructor injection patterns
   - Identify injected dependencies
   - Track framework-based DI (if common decorators present)
   - Mark injected methods as "potentially used"

6. **Callback Pattern Detection**
   - Identify callback registration (`.on()`, `.subscribe()`, `.callback =`)
   - Track callback invocation patterns
   - Detect event handler decorators (`@app.route`, `@handler`)

7. **Template Method Pattern Detection**
   - Identify template method base classes
   - Detect overridden template methods
   - Mark template method implementations as "used via inheritance"

### Non-Functional Requirements

1. **Accuracy**
   - Pattern detection precision > 90%
   - False positive reduction: 20% → < 2%
   - No false negatives (don't miss pattern implementations)

2. **Performance**
   - Pattern recognition adds < 10% to analysis time
   - Efficient pattern matching using indexed data structures
   - Parallel pattern detection across files

3. **Extensibility**
   - Easy to add new pattern recognizers
   - Pluggable pattern detection system
   - Configuration for project-specific patterns

4. **Language Support**
   - Python (primary)
   - Rust (trait-based patterns)
   - JavaScript/TypeScript (prototype patterns)

## Acceptance Criteria

- [ ] Observer pattern detected (abstract base + implementations)
- [ ] Observer interface methods marked as "used via pattern"
- [ ] `ConversationPanel.on_message_added()` example no longer flagged
- [ ] Singleton pattern detected (module-level instances)
- [ ] Singleton instance method calls tracked across files
- [ ] `ConversationManager.add_message()` example no longer flagged
- [ ] Factory pattern detected (factory functions + class registries)
- [ ] Strategy pattern detected (protocol implementations)
- [ ] Callback pattern detected (event registration + handlers)
- [ ] Template method pattern detected (overridden methods)
- [ ] Pattern usage shown in output reasoning
- [ ] False positive rate < 2% on promptconstruct-frontend
- [ ] Performance overhead < 10%
- [ ] Configuration for custom patterns
- [ ] Documentation with pattern examples

## Technical Details

### Implementation Approach

**Phase 1: Observer Pattern**
1. Detect abstract base classes with `@abstractmethod`
2. Find concrete implementations inheriting from ABC
3. Track observer registration patterns
4. Detect polymorphic observer invocations

**Phase 2: Singleton Pattern**
1. Detect module-level class instantiation
2. Track singleton instance exports
3. Integrate with cross-file analysis (Spec 109)

**Phase 3: Additional Patterns**
1. Implement factory pattern detection
2. Add strategy pattern recognition
3. Implement callback pattern detection
4. Add template method pattern

**Phase 4: Integration and Configuration**
1. Integrate pattern detectors into dead code analysis
2. Add configuration for custom patterns
3. Implement pattern reasoning in output

### Architecture Changes

```rust
// src/analysis/patterns/mod.rs
pub mod observer;
pub mod singleton;
pub mod factory;
pub mod strategy;
pub mod callback;
pub mod template_method;

pub struct PatternDetector {
    detectors: Vec<Box<dyn PatternRecognizer>>,
    config: PatternConfig,
}

pub trait PatternRecognizer: Send + Sync {
    fn name(&self) -> &str;
    fn detect(&self, context: &ProjectContext) -> Vec<PatternInstance>;
    fn is_used_by_pattern(&self, function: &FunctionDef, context: &ProjectContext) -> bool;
}

#[derive(Debug, Clone)]
pub struct PatternInstance {
    pub pattern_type: PatternType,
    pub base_class: Option<String>,
    pub implementations: Vec<Implementation>,
    pub usage_sites: Vec<UsageSite>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    Observer,
    Singleton,
    Factory,
    Strategy,
    DependencyInjection,
    Callback,
    TemplateMethod,
}

#[derive(Debug, Clone)]
pub struct Implementation {
    pub file: PathBuf,
    pub class_name: String,
    pub method_name: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct UsageSite {
    pub file: PathBuf,
    pub line: usize,
    pub context: String,
}

impl PatternDetector {
    pub fn new(config: PatternConfig) -> Self;
    pub fn detect_all_patterns(&self, context: &ProjectContext) -> Vec<PatternInstance>;
    pub fn is_function_used_by_pattern(&self, function: &FunctionDef, context: &ProjectContext) -> Option<PatternInstance>;
}

// src/analysis/patterns/observer.rs
pub struct ObserverPatternRecognizer {
    config: ObserverConfig,
}

#[derive(Debug, Clone)]
pub struct ObserverConfig {
    pub interface_markers: Vec<String>, // ABC, Protocol, Interface
    pub registration_methods: Vec<String>, // add_observer, register, subscribe
    pub common_method_prefixes: Vec<String>, // on_, handle_, notify_
}

impl PatternRecognizer for ObserverPatternRecognizer {
    fn detect(&self, context: &ProjectContext) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        // Find abstract base classes
        for (file, analysis) in &context.file_analyses {
            for class in &analysis.classes {
                if self.is_observer_interface(class) {
                    let pattern = self.find_observer_implementations(class, context);
                    patterns.push(pattern);
                }
            }
        }

        patterns
    }

    fn is_used_by_pattern(&self, function: &FunctionDef, context: &ProjectContext) -> bool {
        // Check if function implements observer interface method
        if let Some(class) = &function.class_name {
            if let Some(base_class) = context.get_base_class(class) {
                if self.is_observer_interface(&base_class) {
                    // Check if there's a polymorphic invocation
                    return self.has_polymorphic_invocation(function, context);
                }
            }
        }
        false
    }
}

impl ObserverPatternRecognizer {
    fn is_observer_interface(&self, class: &ClassDef) -> bool {
        // Check if class is ABC with abstract methods
        class.base_classes.iter().any(|b| b == "ABC" || b == "Protocol")
            && class.methods.iter().any(|m| m.is_abstract)
    }

    fn has_polymorphic_invocation(&self, function: &FunctionDef, context: &ProjectContext) -> bool {
        // Look for patterns like: `for obs in observers: obs.method_name()`
        for (file, analysis) in &context.file_analyses {
            if self.find_observer_loops(file, &function.name, analysis) {
                return true;
            }
        }
        false
    }

    fn find_observer_loops(&self, file: &Path, method_name: &str, analysis: &FileAnalysis) -> bool {
        // Parse for patterns:
        // - for x in observers: x.method_name()
        // - [obs.method_name() for obs in self.observers]
        // - self.observers.forEach(obs => obs.method_name())
        analysis.contains_pattern(&format!("for .+ in .+observers:.+\\.{}\\(", method_name))
    }
}

// src/analysis/patterns/singleton.rs
pub struct SingletonPatternRecognizer;

impl PatternRecognizer for SingletonPatternRecognizer {
    fn detect(&self, context: &ProjectContext) -> Vec<PatternInstance> {
        let mut singletons = Vec::new();

        for (file, analysis) in &context.file_analyses {
            // Detect module-level class instantiation
            for assignment in &analysis.module_level_assignments {
                if self.is_singleton_assignment(assignment) {
                    singletons.push(self.create_singleton_pattern(assignment, file));
                }
            }
        }

        singletons
    }

    fn is_used_by_pattern(&self, function: &FunctionDef, context: &ProjectContext) -> bool {
        // Check if function is a method on a singleton instance
        if let Some(class) = &function.class_name {
            if context.symbol_resolver.is_singleton_class(class) {
                // Check for cross-file usage via singleton instance
                return context.call_graph.has_singleton_invocation(function);
            }
        }
        false
    }
}

impl SingletonPatternRecognizer {
    fn is_singleton_assignment(&self, assignment: &Assignment) -> bool {
        // Pattern: `instance = ClassName()`
        assignment.value.is_class_instantiation() && assignment.scope == Scope::Module
    }
}

// src/analysis/patterns/factory.rs
pub struct FactoryPatternRecognizer;

impl PatternRecognizer for FactoryPatternRecognizer {
    fn detect(&self, context: &ProjectContext) -> Vec<PatternInstance> {
        let mut factories = Vec::new();

        for (file, analysis) in &context.file_analyses {
            // Detect factory functions
            for function in &analysis.functions {
                if self.is_factory_function(function) {
                    factories.push(self.create_factory_pattern(function, file));
                }
            }

            // Detect factory registries: {"type_a": ClassA, "type_b": ClassB}
            for dict_var in &analysis.module_level_dicts {
                if self.is_factory_registry(dict_var) {
                    factories.push(self.create_registry_pattern(dict_var, file));
                }
            }
        }

        factories
    }

    fn is_used_by_pattern(&self, function: &FunctionDef, context: &ProjectContext) -> bool {
        // Check if class constructor is called via factory
        if function.name == "__init__" {
            return context.has_factory_instantiation(&function.class_name);
        }
        false
    }
}

impl FactoryPatternRecognizer {
    fn is_factory_function(&self, function: &FunctionDef) -> bool {
        // Heuristics:
        // - Name contains "create", "make", "build", "factory"
        // - Returns class instance
        // - Has conditional class instantiation
        let name_lower = function.name.to_lowercase();
        (name_lower.contains("create")
            || name_lower.contains("make")
            || name_lower.contains("build")
            || name_lower.contains("factory"))
            && function.returns_class_instance()
    }

    fn is_factory_registry(&self, dict_var: &DictVariable) -> bool {
        // Check if dict values are class references
        dict_var.values.iter().all(|v| v.is_class_reference())
    }
}
```

### Data Structures

```rust
#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub file_analyses: HashMap<PathBuf, FileAnalysis>,
    pub symbol_resolver: SymbolResolver,
    pub call_graph: CrossFileCallGraph,
    pub patterns: Vec<PatternInstance>,
}

#[derive(Debug, Clone)]
pub struct ClassDef {
    pub name: String,
    pub base_classes: Vec<String>,
    pub methods: Vec<MethodDef>,
    pub is_abstract: bool,
    pub decorators: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MethodDef {
    pub name: String,
    pub is_abstract: bool,
    pub decorators: Vec<String>,
    pub overrides_base: bool,
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub name: String,
    pub value: Expression,
    pub scope: Scope,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Scope {
    Module,
    Class,
    Function,
}

#[derive(Debug, Clone)]
pub enum Expression {
    ClassInstantiation { class_name: String },
    FunctionCall { function_name: String },
    ClassReference { class_name: String },
    Other,
}

impl Expression {
    pub fn is_class_instantiation(&self) -> bool;
    pub fn is_class_reference(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct DictVariable {
    pub name: String,
    pub keys: Vec<String>,
    pub values: Vec<Expression>,
}
```

### APIs and Interfaces

```rust
// Configuration in .debtmap.toml
[patterns]
enabled = true

[patterns.observer]
interface_markers = ["ABC", "Protocol", "Interface"]
registration_methods = ["add_observer", "register", "subscribe"]
method_prefixes = ["on_", "handle_", "notify_"]

[patterns.singleton]
detect_module_level = true
detect_new_override = true
detect_decorator = true

[patterns.factory]
detect_functions = true
detect_registries = true
name_patterns = ["create_", "make_", "build_", "_factory"]

[patterns.custom]
# User-defined pattern matching rules
[[patterns.custom.rule]]
name = "event_handler"
method_pattern = "^handle_.*_event$"
registration_pattern = "\.register_handler\\("

// CLI options
Commands::Analyze {
    /// Disable pattern recognition
    #[arg(long = "no-pattern-detection")]
    no_pattern_detection: bool,

    /// Enable specific patterns only
    #[arg(long = "patterns")]
    patterns: Option<Vec<String>>, // observer,singleton,factory
}

// Integration with dead code detection
impl DeadCodeDetector {
    pub fn detect_with_pattern_analysis(
        &self,
        function: &FunctionDef,
        context: &ProjectContext,
    ) -> Option<DeadCodeFinding> {
        // Check if function is used via design pattern
        if let Some(pattern) = self.pattern_detector.is_function_used_by_pattern(function, context) {
            return None; // Used via pattern, not dead code
        }

        // Continue with normal dead code detection
        self.detect_without_patterns(function, context)
    }
}
```

### Integration Points

1. **Symbol Resolver** (Spec 109)
   - Track class inheritance relationships
   - Provide base class lookup
   - Identify abstract methods

2. **Cross-File Call Graph** (Spec 109)
   - Query polymorphic invocations
   - Track singleton instance usage
   - Identify factory-created objects

3. **Dead Code Detector**
   - Query pattern detector before marking as dead
   - Include pattern usage in findings

4. **Output Formatters**
   - Show pattern detection reasoning
   - Explain why function is used via pattern

## Dependencies

- **Prerequisites**:
  - Spec 109 (Cross-File Dependency Analysis) - Provides call graph and symbol resolution
- **Affected Components**:
  - `src/analysis/patterns/` - New module for pattern recognition
  - `src/debt/dead_code.rs` - Integrate pattern detection
  - `src/analyzers/python/` - Extract class inheritance and decorators
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observer_interface_detection() {
        let code = r#"
from abc import ABC, abstractmethod

class Observer(ABC):
    @abstractmethod
    def on_event(self, data):
        pass
        "#;

        let analysis = analyze_python_code(code, "observer.py");
        let recognizer = ObserverPatternRecognizer::new(ObserverConfig::default());

        let classes = &analysis.classes;
        assert!(recognizer.is_observer_interface(&classes[0]));
    }

    #[test]
    fn test_observer_implementation_detection() {
        let interface_code = r#"
class Observer(ABC):
    @abstractmethod
    def on_event(self):
        pass
        "#;

        let impl_code = r#"
class ConcreteObserver(Observer):
    def on_event(self):
        print("Event received")
        "#;

        let context = create_test_context(vec![
            ("observer.py", interface_code),
            ("concrete.py", impl_code),
        ]);

        let recognizer = ObserverPatternRecognizer::new(ObserverConfig::default());
        let patterns = recognizer.detect(&context);

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, PatternType::Observer);
        assert_eq!(patterns[0].implementations.len(), 1);
    }

    #[test]
    fn test_polymorphic_invocation_detection() {
        let code = r#"
for observer in self.observers:
    observer.on_event(data)
        "#;

        let analysis = analyze_python_code(code, "manager.py");
        let recognizer = ObserverPatternRecognizer::new(ObserverConfig::default());

        assert!(recognizer.find_observer_loops(
            Path::new("manager.py"),
            "on_event",
            &analysis
        ));
    }

    #[test]
    fn test_singleton_detection() {
        let code = r#"
class Manager:
    def process(self):
        pass

manager = Manager()  # Module-level singleton
        "#;

        let analysis = analyze_python_code(code, "manager.py");
        let recognizer = SingletonPatternRecognizer;

        let assignments = &analysis.module_level_assignments;
        assert!(recognizer.is_singleton_assignment(&assignments[0]));
    }

    #[test]
    fn test_factory_function_detection() {
        let code = r#"
def create_handler(handler_type):
    if handler_type == "a":
        return HandlerA()
    else:
        return HandlerB()
        "#;

        let analysis = analyze_python_code(code, "factory.py");
        let recognizer = FactoryPatternRecognizer;

        let function = &analysis.functions[0];
        assert!(recognizer.is_factory_function(function));
    }

    #[test]
    fn test_factory_registry_detection() {
        let code = r#"
HANDLERS = {
    "type_a": HandlerA,
    "type_b": HandlerB,
}
        "#;

        let analysis = analyze_python_code(code, "registry.py");
        let recognizer = FactoryPatternRecognizer;

        let dict_var = &analysis.module_level_dicts[0];
        assert!(recognizer.is_factory_registry(dict_var));
    }
}
```

### Integration Tests

**Test Case 1: Observer Pattern**
```python
# tests/fixtures/patterns/observer_base.py
from abc import ABC, abstractmethod

class Observer(ABC):
    @abstractmethod
    def on_message_added(self, message, index):
        pass

# tests/fixtures/patterns/observer_impl.py
class ConversationPanel(Observer):
    def on_message_added(self, message, index):
        print(f"Message added: {message}")

# tests/fixtures/patterns/observer_manager.py
class Manager:
    def __init__(self):
        self.observers = []

    def add_observer(self, observer):
        self.observers.append(observer)

    def notify(self, message, index):
        for observer in self.observers:
            observer.on_message_added(message, index)
```

Expected: `ConversationPanel.on_message_added()` NOT flagged as dead code.

**Test Case 2: Singleton Pattern**
```python
# tests/fixtures/patterns/singleton_class.py
class Service:
    def do_work(self):
        pass

service = Service()

# tests/fixtures/patterns/singleton_client.py
from singleton_class import service
service.do_work()
```

Expected: `Service.do_work()` NOT flagged as dead code.

**Test Case 3: Factory Pattern**
```python
# tests/fixtures/patterns/factory.py
class HandlerA:
    def handle(self):
        pass

class HandlerB:
    def handle(self):
        pass

def create_handler(handler_type):
    if handler_type == "a":
        return HandlerA()
    return HandlerB()

# tests/fixtures/patterns/factory_client.py
from factory import create_handler
handler = create_handler("a")
handler.handle()
```

Expected: `HandlerA.handle()` NOT flagged as dead code (created via factory).

### Performance Tests

```rust
#[test]
fn test_pattern_detection_performance() {
    let temp_dir = create_large_python_project(1000); // 1000 files
    let files = discover_python_files(&temp_dir);

    let start = Instant::now();
    let context = analyze_project_with_cross_file_context(files, &Config::default()).unwrap();
    let patterns = PatternDetector::new(PatternConfig::default()).detect_all_patterns(&context);
    let duration = start.elapsed();

    let analysis_time = Duration::from_secs(2); // Baseline from Spec 109
    let overhead_pct = (duration.as_secs_f32() - analysis_time.as_secs_f32()) / analysis_time.as_secs_f32() * 100.0;

    assert!(overhead_pct < 10.0, "Pattern detection overhead: {:.1}%", overhead_pct);
}
```

## Documentation Requirements

### Code Documentation

- Document each pattern recognizer's algorithm
- Explain pattern matching heuristics
- Provide examples for each pattern type

### User Documentation

Add to user guide:

```markdown
## Design Pattern Recognition

Debtmap detects common design patterns to avoid false positives:

### Supported Patterns

1. **Observer Pattern**
   - Abstract base classes with `@abstractmethod`
   - Concrete implementations inheriting from ABC
   - Polymorphic invocations: `for obs in observers: obs.method()`

2. **Singleton Pattern**
   - Module-level class instantiation: `manager = Manager()`
   - Tracked across file imports

3. **Factory Pattern**
   - Factory functions: `create_*`, `make_*`, `build_*`
   - Factory registries: `{"type": HandlerClass}`

4. **Strategy Pattern**
   - Strategy interfaces (Protocol, ABC)
   - Strategy implementations and injection

5. **Callback Pattern**
   - Callback registration: `.on()`, `.subscribe()`
   - Event handler decorators: `@app.route`, `@handler`

6. **Template Method Pattern**
   - Base class template methods
   - Overridden implementations

### Configuration

Enable/disable patterns:

```toml
# .debtmap.toml
[patterns]
enabled = true

[patterns.observer]
interface_markers = ["ABC", "Protocol", "Interface"]
registration_methods = ["add_observer", "register", "subscribe"]
method_prefixes = ["on_", "handle_", "notify_"]

[patterns.custom]
[[patterns.custom.rule]]
name = "event_handler"
method_pattern = "^handle_.*_event$"
registration_pattern = "\\.register_handler\\("
```

### CLI Usage

```bash
# Disable pattern detection
debtmap analyze src --no-pattern-detection

# Enable specific patterns only
debtmap analyze src --patterns observer,singleton
```

### Output

Pattern usage shown in results:

```
#5 ConversationPanel.on_message_added [USED VIA PATTERN]
  Location: conversation_panel.py:583
  Pattern: Observer (implements ConversationObserver)
  Invocation: conversation_manager.py:137
    for observer in self.observers:
        observer.on_message_added(message, index)
```
```

### Architecture Documentation

Update ARCHITECTURE.md with pattern detection pipeline.

## Implementation Notes

### Pattern Detection Strategy

1. **Build pattern index**: Scan for pattern definitions (ABCs, singletons, etc.)
2. **Find implementations**: Match concrete classes to pattern interfaces
3. **Detect invocations**: Find polymorphic calls, factory usage, etc.
4. **Mark as used**: Exclude pattern implementations from dead code

### Heuristic Tuning

- **Observer**: Require both ABC inheritance AND polymorphic loop
- **Singleton**: Module-level assignment + cross-file import
- **Factory**: Name pattern OR returns multiple class types
- **Strategy**: Protocol implementation + injection detection

### Edge Cases

1. **Multiple pattern implementations**: Class implements multiple patterns
2. **Nested patterns**: Observer that's also a singleton
3. **Partial implementations**: Class inherits ABC but doesn't implement all methods
4. **Dynamic patterns**: Runtime pattern construction (e.g., `type()` usage)

### Performance Optimization

- **Index patterns once**: Build pattern index upfront
- **Lazy evaluation**: Only check patterns for functions without direct calls
- **Cache pattern queries**: Memoize pattern membership checks
- **Parallel detection**: Run pattern recognizers in parallel

## Migration and Compatibility

### Backward Compatibility

- **No breaking changes**: Reduces false positives only
- **Opt-out option**: `--no-pattern-detection` flag
- **Gradual rollout**: Enable patterns incrementally

### Migration Path

For existing users:
1. **Automatic activation**: Pattern detection runs by default
2. **Review flagged items**: Check if previously detected dead code is now marked as pattern usage
3. **Tune configuration**: Adjust pattern settings for project conventions

## Future Enhancements

1. **Additional patterns**: Command, Mediator, Proxy, Decorator patterns
2. **Framework-specific patterns**: Django views, Flask routes, FastAPI endpoints
3. **Machine learning**: Train model on labeled pattern examples
4. **Pattern visualization**: Generate diagrams showing pattern relationships
5. **Pattern suggestions**: Recommend design patterns for code structure
6. **Anti-pattern detection**: Identify pattern misuse

## Success Metrics

- **False positive reduction**: 20% → < 2%
- **Pattern coverage**: Detect 95% of common pattern implementations
- **Performance**: < 10% overhead
- **User adoption**: 50% of users benefit from pattern detection
- **Bug reports**: Zero complaints about "removed my observer implementation"

## Related Specifications

- Spec 109: Cross-File Dependency Analysis (provides call graph and symbol resolution)
- Spec 110: Public API Detection (complementary false positive reduction)
- Spec 113: Confidence Scoring (uses pattern detection for confidence)
