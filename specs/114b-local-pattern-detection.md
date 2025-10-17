---
number: 114b
title: Local Pattern Detection
category: foundation
priority: high
status: draft
dependencies: [114a]
created: 2025-10-16
---

# Specification 114b: Local Pattern Detection

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 114a (Parser Enhancements)

## Context

This is Phase 2 of the Design Pattern Recognition feature (Spec 114). With enhanced AST extraction from Phase 1, we can now detect design patterns within individual files using decorator information, inheritance relationships, and AST patterns.

**Current Gap**: Even with rich AST data, debtmap doesn't recognize:
- Observer pattern (ABC + abstract methods)
- Factory pattern (factory functions with conditional instantiation)
- Callback pattern (decorator-based event handlers)

**Why Single-File First**: Starting with local detection:
- Simpler implementation (no cross-file resolution needed)
- Validates pattern matching algorithms
- Provides immediate value for single-file patterns
- Foundation for cross-file integration (Phase 3)

## Objective

Implement single-file pattern detection for Observer, Factory, and Callback patterns using AST-based heuristics, enabling debtmap to identify pattern usage without cross-file analysis.

## Requirements

### Functional Requirements

1. **Observer Pattern Detection (Single-File)**
   - Identify abstract base classes with `@abstractmethod`
   - Detect concrete implementations inheriting from ABC
   - Find observer invocation loops in same file
   - Match observer method calls to implementations
   - Conservative approach: mark as "possibly used via pattern"

2. **Factory Pattern Detection**
   - Identify factory functions by name heuristics
   - Detect factory registries (dictionaries mapping types to classes)
   - Recognize conditional class instantiation
   - Track which classes are instantiated via factories

3. **Callback Pattern Detection**
   - Identify event handler decorators (`@app.route`, `@handler`)
   - Detect callback registration calls (`.on()`, `.subscribe()`)
   - Track callback function assignments
   - Mark decorated functions as "used via callback"

4. **Pattern Recognition API**
   - Create `PatternDetector` interface
   - Implement `PatternRecognizer` trait for each pattern
   - Return `PatternInstance` with usage information
   - Support querying if function is used by pattern

### Non-Functional Requirements

1. **Accuracy**
   - Pattern detection precision > 85% (single-file)
   - No false negatives (don't miss obvious patterns)
   - Accept false positives for edge cases (conservative)

2. **Performance**
   - Pattern detection adds < 5% to analysis time
   - Use indexed lookups for pattern queries
   - Avoid repeated AST traversals

3. **Extensibility**
   - Easy to add new pattern recognizers
   - Pluggable pattern detection system
   - Clear interface for pattern types

## Acceptance Criteria

- [ ] Observer pattern detected in single file
- [ ] Abstract methods identified via `@abstractmethod`
- [ ] Concrete observer implementations detected
- [ ] Observer invocation loops recognized (`for obs in observers`)
- [ ] Factory functions identified by name patterns
- [ ] Factory registries detected (dict mapping types to classes)
- [ ] Callback decorators recognized (`@app.route`, `@handler`)
- [ ] PatternDetector API implemented
- [ ] Unit tests for each pattern recognizer
- [ ] Integration tests with real Python patterns
- [ ] Performance overhead < 5%

## Technical Details

### Architecture

```rust
// src/analysis/patterns/mod.rs
pub mod observer;
pub mod factory;
pub mod callback;

#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    Observer,
    Factory,
    Callback,
    // Future: Singleton, Strategy, TemplateMethod, DependencyInjection
}

#[derive(Debug, Clone)]
pub struct PatternInstance {
    pub pattern_type: PatternType,
    pub confidence: f32,  // 0.0 - 1.0
    pub base_class: Option<String>,
    pub implementations: Vec<Implementation>,
    pub usage_sites: Vec<UsageSite>,
    pub reasoning: String,
}

#[derive(Debug, Clone)]
pub struct Implementation {
    pub file: PathBuf,
    pub class_name: Option<String>,
    pub function_name: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct UsageSite {
    pub file: PathBuf,
    pub line: usize,
    pub context: String,  // Code snippet showing usage
}

pub trait PatternRecognizer: Send + Sync {
    fn name(&self) -> &str;
    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance>;
    fn is_function_used_by_pattern(&self, function: &FunctionDef, file_metrics: &FileMetrics) -> Option<PatternInstance>;
}

pub struct PatternDetector {
    recognizers: Vec<Box<dyn PatternRecognizer>>,
}

impl PatternDetector {
    pub fn new() -> Self {
        Self {
            recognizers: vec![
                Box::new(ObserverPatternRecognizer::new()),
                Box::new(FactoryPatternRecognizer::new()),
                Box::new(CallbackPatternRecognizer::new()),
            ],
        }
    }

    pub fn detect_all_patterns(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        self.recognizers
            .iter()
            .flat_map(|recognizer| recognizer.detect(file_metrics))
            .collect()
    }

    pub fn is_function_used_by_pattern(
        &self,
        function: &FunctionDef,
        file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        for recognizer in &self.recognizers {
            if let Some(pattern) = recognizer.is_function_used_by_pattern(function, file_metrics) {
                return Some(pattern);
            }
        }
        None
    }
}
```

### Observer Pattern Recognizer

```rust
// src/analysis/patterns/observer.rs
use super::{PatternInstance, PatternRecognizer, PatternType, Implementation, UsageSite};
use crate::core::{FileMetrics, FunctionDef, ClassDef, MethodDef};
use rustpython_parser::ast;
use std::path::PathBuf;

pub struct ObserverPatternRecognizer;

impl ObserverPatternRecognizer {
    pub fn new() -> Self {
        Self
    }

    /// Check if class is an observer interface (ABC with abstract methods)
    fn is_observer_interface(&self, class: &ClassDef) -> bool {
        // Must inherit from ABC or Protocol
        let has_abc_base = class.base_classes.iter().any(|b| {
            b.contains("ABC") || b.contains("Protocol") || b.contains("Interface")
        });

        // Must have at least one abstract method
        let has_abstract_methods = class.methods.iter().any(|m| m.is_abstract);

        has_abc_base && has_abstract_methods
    }

    /// Find concrete implementations of observer interface
    fn find_implementations(&self, interface: &ClassDef, file_metrics: &FileMetrics) -> Vec<Implementation> {
        file_metrics
            .classes
            .iter()
            .filter(|class| {
                // Check if class inherits from interface
                class.base_classes.contains(&interface.name)
            })
            .flat_map(|class| {
                // Return all methods that override abstract methods
                class.methods.iter().filter_map(|method| {
                    if interface.methods.iter().any(|m| m.name == method.name && m.is_abstract) {
                        Some(Implementation {
                            file: file_metrics.path.clone(),
                            class_name: Some(class.name.clone()),
                            function_name: method.name.clone(),
                            line: method.line,
                        })
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    /// Find observer invocation loops in AST
    /// Detects patterns like: `for observer in self.observers: observer.method_name()`
    fn find_observer_loops(&self, ast: &ast::Mod, method_name: &str) -> Vec<UsageSite> {
        let mut usage_sites = Vec::new();

        if let ast::Mod::Module(module) = ast {
            self.traverse_for_observer_loops(&module.body, method_name, &mut usage_sites);
        }

        usage_sites
    }

    fn traverse_for_observer_loops(
        &self,
        stmts: &[ast::Stmt],
        method_name: &str,
        usage_sites: &mut Vec<UsageSite>,
    ) {
        for stmt in stmts {
            match stmt {
                ast::Stmt::For(for_loop) => {
                    // Check if iterating over observer collection
                    if self.is_observer_collection(&for_loop.iter) {
                        // Check if loop body calls method_name on loop variable
                        if self.loop_calls_method(&for_loop.body, &for_loop.target, method_name) {
                            usage_sites.push(UsageSite {
                                file: PathBuf::new(), // Will be set by caller
                                line: for_loop.range.start.to_usize(),
                                context: format!("for ... in observers: observer.{}()", method_name),
                            });
                        }
                    }
                }
                ast::Stmt::FunctionDef(func) => {
                    self.traverse_for_observer_loops(&func.body, method_name, usage_sites);
                }
                ast::Stmt::ClassDef(class) => {
                    self.traverse_for_observer_loops(&class.body, method_name, usage_sites);
                }
                _ => {}
            }
        }
    }

    fn is_observer_collection(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::Attribute(attr) => {
                // self.observers, self.callbacks, etc.
                matches!(
                    attr.attr.as_str(),
                    "observers" | "callbacks" | "listeners" | "handlers" | "subscribers"
                )
            }
            ast::Expr::Name(name) => {
                // Local variable named observers, callbacks, etc.
                matches!(
                    name.id.as_str(),
                    "observers" | "callbacks" | "listeners" | "handlers" | "subscribers"
                )
            }
            _ => false,
        }
    }

    fn loop_calls_method(&self, body: &[ast::Stmt], target: &ast::Expr, method_name: &str) -> bool {
        for stmt in body {
            if let ast::Stmt::Expr(expr_stmt) = stmt {
                if let ast::Expr::Call(call) = &expr_stmt.value {
                    if let ast::Expr::Attribute(attr) = &*call.func {
                        if attr.attr == method_name {
                            if self.expressions_match(&attr.value, target) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn expressions_match(&self, expr1: &ast::Expr, expr2: &ast::Expr) -> bool {
        match (expr1, expr2) {
            (ast::Expr::Name(n1), ast::Expr::Name(n2)) => n1.id == n2.id,
            _ => false,
        }
    }
}

impl PatternRecognizer for ObserverPatternRecognizer {
    fn name(&self) -> &str {
        "Observer"
    }

    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        // Find all observer interfaces
        for class in &file_metrics.classes {
            if self.is_observer_interface(class) {
                let implementations = self.find_implementations(class, file_metrics);

                if !implementations.is_empty() {
                    patterns.push(PatternInstance {
                        pattern_type: PatternType::Observer,
                        confidence: 0.9, // High confidence for ABC + @abstractmethod
                        base_class: Some(class.name.clone()),
                        implementations: implementations.clone(),
                        usage_sites: Vec::new(), // TODO: Find invocation sites
                        reasoning: format!(
                            "Observer interface {} with {} concrete implementations",
                            class.name,
                            implementations.len()
                        ),
                    });
                }
            }
        }

        patterns
    }

    fn is_function_used_by_pattern(
        &self,
        function: &FunctionDef,
        file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        // Check if function is an observer method
        let class_name = function.class_name.as_ref()?;

        // Find the class
        let class = file_metrics.classes.iter().find(|c| &c.name == class_name)?;

        // Check if class implements an observer interface
        for base_class_name in &class.base_classes {
            if let Some(base_class) = file_metrics.classes.iter().find(|c| &c.name == base_class_name) {
                if self.is_observer_interface(base_class) {
                    // Check if this method overrides an abstract method
                    if base_class.methods.iter().any(|m| m.name == function.name && m.is_abstract) {
                        return Some(PatternInstance {
                            pattern_type: PatternType::Observer,
                            confidence: 0.85,
                            base_class: Some(base_class.name.clone()),
                            implementations: vec![Implementation {
                                file: file_metrics.path.clone(),
                                class_name: Some(class_name.clone()),
                                function_name: function.name.clone(),
                                line: function.start_line,
                            }],
                            usage_sites: Vec::new(),
                            reasoning: format!(
                                "Implements abstract method {} from observer interface {}",
                                function.name, base_class.name
                            ),
                        });
                    }
                }
            }
        }

        None
    }
}
```

### Factory Pattern Recognizer

```rust
// src/analysis/patterns/factory.rs
use super::{PatternInstance, PatternRecognizer, PatternType, Implementation};
use crate::core::{FileMetrics, FunctionDef};

pub struct FactoryPatternRecognizer;

impl FactoryPatternRecognizer {
    pub fn new() -> Self {
        Self
    }

    fn is_factory_function(&self, function: &FunctionDef) -> bool {
        let name_lower = function.name.to_lowercase();

        // Name-based heuristics
        let has_factory_name = name_lower.contains("create")
            || name_lower.contains("make")
            || name_lower.contains("build")
            || name_lower.contains("factory")
            || name_lower.starts_with("get_")
            || name_lower.starts_with("new_");

        // TODO: Check if function returns different class types
        // TODO: Check for conditional instantiation patterns

        has_factory_name
    }
}

impl PatternRecognizer for FactoryPatternRecognizer {
    fn name(&self) -> &str {
        "Factory"
    }

    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        // Find factory functions
        for function in &file_metrics.functions {
            if self.is_factory_function(function) {
                patterns.push(PatternInstance {
                    pattern_type: PatternType::Factory,
                    confidence: 0.7, // Medium confidence (name-based only)
                    base_class: None,
                    implementations: vec![Implementation {
                        file: file_metrics.path.clone(),
                        class_name: None,
                        function_name: function.name.clone(),
                        line: function.start_line,
                    }],
                    usage_sites: Vec::new(),
                    reasoning: format!("Factory function {} (name-based detection)", function.name),
                });
            }
        }

        patterns
    }

    fn is_function_used_by_pattern(
        &self,
        _function: &FunctionDef,
        _file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        // Factory pattern primarily affects instantiated classes, not the factory itself
        None
    }
}
```

### Callback Pattern Recognizer

```rust
// src/analysis/patterns/callback.rs
use super::{PatternInstance, PatternRecognizer, PatternType, Implementation};
use crate::core::{FileMetrics, FunctionDef};

pub struct CallbackPatternRecognizer;

impl CallbackPatternRecognizer {
    pub fn new() -> Self {
        Self
    }

    fn has_callback_decorator(&self, function: &FunctionDef) -> bool {
        function.decorators.iter().any(|d| {
            d.contains("route")
                || d.contains("handler")
                || d.contains("app.")
                || d.contains("callback")
                || d.contains("on_")
        })
    }
}

impl PatternRecognizer for CallbackPatternRecognizer {
    fn name(&self) -> &str {
        "Callback"
    }

    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        for function in &file_metrics.functions {
            if self.has_callback_decorator(function) {
                patterns.push(PatternInstance {
                    pattern_type: PatternType::Callback,
                    confidence: 0.95, // High confidence for decorator-based detection
                    base_class: None,
                    implementations: vec![Implementation {
                        file: file_metrics.path.clone(),
                        class_name: None,
                        function_name: function.name.clone(),
                        line: function.start_line,
                    }],
                    usage_sites: Vec::new(),
                    reasoning: format!("Callback handler {} (decorator-based)", function.name),
                });
            }
        }

        patterns
    }

    fn is_function_used_by_pattern(
        &self,
        function: &FunctionDef,
        _file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        if self.has_callback_decorator(function) {
            Some(PatternInstance {
                pattern_type: PatternType::Callback,
                confidence: 0.95,
                base_class: None,
                implementations: vec![Implementation {
                    file: PathBuf::new(),
                    class_name: None,
                    function_name: function.name.clone(),
                    line: function.start_line,
                }],
                usage_sites: Vec::new(),
                reasoning: format!("Callback handler {}", function.name),
            })
        } else {
            None
        }
    }
}
```

## Dependencies

- **Prerequisites**: Spec 114a (Parser Enhancements)
- **Affected Components**:
  - `src/analysis/patterns/` - **New module**
  - `src/analysis/patterns/observer.rs` - **New**
  - `src/analysis/patterns/factory.rs` - **New**
  - `src/analysis/patterns/callback.rs` - **New**
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observer_interface_detection() {
        let file_metrics = create_test_file_metrics_with_classes(vec![
            ClassDef {
                name: "Observer".to_string(),
                base_classes: vec!["ABC".to_string()],
                methods: vec![MethodDef {
                    name: "on_event".to_string(),
                    is_abstract: true,
                    decorators: vec!["abstractmethod".to_string()],
                    overrides_base: false,
                    line: 10,
                }],
                is_abstract: true,
                decorators: vec![],
                line: 5,
            },
        ]);

        let recognizer = ObserverPatternRecognizer::new();
        let patterns = recognizer.detect(&file_metrics);

        assert_eq!(patterns.len(), 0); // No implementations in same file
    }

    #[test]
    fn test_observer_implementation_detection() {
        let file_metrics = create_test_file_metrics_with_classes(vec![
            ClassDef {
                name: "Observer".to_string(),
                base_classes: vec!["ABC".to_string()],
                methods: vec![MethodDef {
                    name: "on_event".to_string(),
                    is_abstract: true,
                    decorators: vec!["abstractmethod".to_string()],
                    overrides_base: false,
                    line: 10,
                }],
                is_abstract: true,
                decorators: vec![],
                line: 5,
            },
            ClassDef {
                name: "ConcreteObserver".to_string(),
                base_classes: vec!["Observer".to_string()],
                methods: vec![MethodDef {
                    name: "on_event".to_string(),
                    is_abstract: false,
                    decorators: vec![],
                    overrides_base: true,
                    line: 20,
                }],
                is_abstract: false,
                decorators: vec![],
                line: 18,
            },
        ]);

        let recognizer = ObserverPatternRecognizer::new();
        let patterns = recognizer.detect(&file_metrics);

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, PatternType::Observer);
        assert_eq!(patterns[0].implementations.len(), 1);
    }

    #[test]
    fn test_factory_function_detection() {
        let function = FunctionDef {
            name: "create_handler".to_string(),
            start_line: 10,
            decorators: vec![],
            class_name: None,
            // ... other fields
        };

        let recognizer = FactoryPatternRecognizer::new();
        assert!(recognizer.is_factory_function(&function));
    }

    #[test]
    fn test_callback_decorator_detection() {
        let function = FunctionDef {
            name: "handle_request".to_string(),
            start_line: 10,
            decorators: vec!["app.route('/api')".to_string()],
            class_name: None,
            // ... other fields
        };

        let recognizer = CallbackPatternRecognizer::new();
        assert!(recognizer.has_callback_decorator(&function));
    }
}
```

### Integration Tests

Create test fixtures in `tests/fixtures/patterns/`:

```python
# tests/fixtures/patterns/observer_single_file.py
from abc import ABC, abstractmethod

class Observer(ABC):
    @abstractmethod
    def on_event(self, data):
        pass

class ConcreteObserver(Observer):
    def on_event(self, data):
        print(f"Event: {data}")

class Manager:
    def __init__(self):
        self.observers = []

    def add_observer(self, observer):
        self.observers.append(observer)

    def notify(self, data):
        for observer in self.observers:
            observer.on_event(data)
```

Test that:
- Observer interface is detected
- ConcreteObserver implementation is detected
- `on_event` is marked as "used via pattern"

## Documentation Requirements

- Document PatternDetector API
- Explain each pattern recognizer's algorithm
- Provide examples of detected patterns
- Update ARCHITECTURE.md with pattern detection flow

## Implementation Notes

### Conservative Detection
- Prefer false positives over false negatives
- If pattern is ambiguous, mark as "possibly used"
- Provide confidence scores for uncertain patterns

### Heuristic Tuning
- **Observer**: ABC + @abstractmethod = high confidence
- **Factory**: Name pattern = medium confidence
- **Callback**: Decorator = high confidence

### Performance
- Single AST traversal per file
- Index classes and functions for quick lookup
- Avoid repeated pattern matching

## Success Metrics

- [ ] Observer pattern detection: 85%+ accuracy on test suite
- [ ] Factory pattern detection: 70%+ accuracy (name-based)
- [ ] Callback pattern detection: 95%+ accuracy (decorator-based)
- [ ] Performance overhead < 5%
- [ ] No false negatives on obvious patterns
