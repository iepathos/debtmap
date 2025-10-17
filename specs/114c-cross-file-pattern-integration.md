---
number: 114c
title: Cross-File Pattern Integration
category: foundation
priority: high
status: draft
dependencies: [114a, 114b]
created: 2025-10-16
---

# Specification 114c: Cross-File Pattern Integration

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 114a (Parser Enhancements), Spec 114b (Local Pattern Detection)

## Context

This is Phase 3 of the Design Pattern Recognition feature (Spec 114). With local pattern detection working, we now need to extend it across files to detect patterns like:
- Singleton instances imported and used in other files
- Observer implementations in separate files from interfaces
- Polymorphic invocations calling methods across module boundaries

**Real-World Example from Bug Report**:
```python
# conversation_manager.py
class ConversationObserver(ABC):
    @abstractmethod
    def on_message_added(self, message, index):
        pass

# conversation_panel.py (DIFFERENT FILE)
class ConversationPanel(ConversationObserver):
    def on_message_added(self, message, index):  # ❌ Currently flagged as dead
        """Handle new message added to conversation."""
        # Implementation...

# conversation_manager.py
for observer in self.observers:
    observer.on_message_added(message, index)  # ✅ Polymorphic call across files
```

**Why This is Critical**: Most real-world patterns span multiple files. Without cross-file integration, pattern detection is incomplete and false positives remain.

## Objective

Integrate pattern detection with existing cross-file analysis infrastructure (`CrossModuleContext`, `TraitRegistry`) to detect patterns that span multiple files, particularly singleton usage and polymorphic observer invocations.

## Requirements

### Functional Requirements

1. **Cross-File Observer Pattern**
   - Track observer interfaces defined in one file
   - Find implementations in other files via inheritance tracking
   - Detect polymorphic invocations across module boundaries
   - Use `CrossModuleContext` for symbol resolution

2. **Singleton Pattern Detection**
   - Detect module-level class instantiation
   - Track singleton instance exports via `CrossModuleContext`
   - Follow singleton usage across files
   - Mark singleton instance methods as "used"

3. **Polymorphic Invocation Detection**
   - Identify method calls on interface types
   - Resolve to concrete implementations via call graph
   - Handle `for obs in observers: obs.method()` pattern
   - Support attribute access patterns

4. **Integration with CrossModuleContext**
   - Use symbol resolution for class lookup
   - Leverage namespace tracking for imports
   - Utilize dependency graph for file relationships
   - Share pattern information across modules

5. **Integration with TraitRegistry (Rust)**
   - Map Rust traits to observer pattern
   - Use trait method resolution for polymorphic calls
   - Leverage existing visitor pattern detection

### Non-Functional Requirements

1. **Accuracy**
   - Pattern detection precision > 90% (cross-file)
   - Reduce false positive rate from 20% to < 5%
   - No false negatives for common patterns

2. **Performance**
   - Cross-file pattern detection adds < 5% overhead
   - Efficient use of existing call graph
   - Cached pattern queries

3. **Compatibility**
   - Works with existing `CrossModuleContext`
   - Compatible with `TraitRegistry` for Rust
   - No breaking changes to existing APIs

## Acceptance Criteria

- [ ] Observer pattern detected across files
- [ ] Observer implementations in separate files tracked
- [ ] Polymorphic invocations resolved via call graph
- [ ] `ConversationPanel.on_message_added()` example no longer flagged
- [ ] Singleton pattern detected (module-level instances)
- [ ] Singleton instance method calls tracked across files
- [ ] `ConversationManager.add_message()` example no longer flagged
- [ ] Integration with `CrossModuleContext` working
- [ ] Integration with `TraitRegistry` for Rust patterns
- [ ] False positive rate < 5% on test suite
- [ ] Performance overhead < 5%
- [ ] Unit tests for cross-file resolution
- [ ] Integration tests with multi-file patterns

## Technical Details

### Enhanced PatternDetector with Cross-File Context

```rust
// src/analysis/patterns/mod.rs
use crate::analysis::python_call_graph::cross_module::CrossModuleContext;
use crate::analysis::call_graph::trait_registry::TraitRegistry;
use std::sync::Arc;

pub struct PatternDetector {
    recognizers: Vec<Box<dyn PatternRecognizer>>,
    cross_module_context: Option<Arc<CrossModuleContext>>,
    trait_registry: Option<Arc<TraitRegistry>>,
}

impl PatternDetector {
    pub fn new() -> Self {
        Self {
            recognizers: vec![
                Box::new(ObserverPatternRecognizer::new()),
                Box::new(SingletonPatternRecognizer::new()),
                Box::new(FactoryPatternRecognizer::new()),
                Box::new(CallbackPatternRecognizer::new()),
            ],
            cross_module_context: None,
            trait_registry: None,
        }
    }

    pub fn with_cross_module_context(mut self, context: Arc<CrossModuleContext>) -> Self {
        self.cross_module_context = Some(context);
        self
    }

    pub fn with_trait_registry(mut self, registry: Arc<TraitRegistry>) -> Self {
        self.trait_registry = Some(registry);
        self
    }

    /// Detect patterns across all files in project
    pub fn detect_project_patterns(&self, project_metrics: &ProjectMetrics) -> Vec<PatternInstance> {
        let mut all_patterns = Vec::new();

        // Single-file patterns
        for file_metrics in &project_metrics.files {
            for recognizer in &self.recognizers {
                all_patterns.extend(recognizer.detect(file_metrics));
            }
        }

        // Cross-file patterns (requires context)
        if let Some(context) = &self.cross_module_context {
            all_patterns.extend(self.detect_cross_file_observer_patterns(project_metrics, context));
            all_patterns.extend(self.detect_singleton_patterns(project_metrics, context));
        }

        all_patterns
    }

    /// Detect observer patterns that span multiple files
    fn detect_cross_file_observer_patterns(
        &self,
        project_metrics: &ProjectMetrics,
        context: &CrossModuleContext,
    ) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        // Find all observer interfaces
        for file_metrics in &project_metrics.files {
            for class in &file_metrics.classes {
                if self.is_observer_interface(class) {
                    // Find implementations across all files
                    let implementations = self.find_cross_file_implementations(
                        class,
                        &file_metrics.path,
                        project_metrics,
                        context,
                    );

                    if !implementations.is_empty() {
                        patterns.push(PatternInstance {
                            pattern_type: PatternType::Observer,
                            confidence: 0.95,
                            base_class: Some(class.name.clone()),
                            implementations,
                            usage_sites: self.find_polymorphic_invocations(
                                &class.name,
                                project_metrics,
                            ),
                            reasoning: format!(
                                "Observer interface {} with cross-file implementations",
                                class.name
                            ),
                        });
                    }
                }
            }
        }

        patterns
    }

    /// Find implementations of interface across all files
    fn find_cross_file_implementations(
        &self,
        interface: &ClassDef,
        interface_file: &Path,
        project_metrics: &ProjectMetrics,
        context: &CrossModuleContext,
    ) -> Vec<Implementation> {
        let mut implementations = Vec::new();

        for file_metrics in &project_metrics.files {
            for class in &file_metrics.classes {
                // Check if class imports and inherits from interface
                if self.inherits_from_interface(
                    class,
                    interface,
                    &file_metrics.path,
                    interface_file,
                    context,
                ) {
                    // Find methods that override abstract methods
                    for method in &class.methods {
                        if interface.methods.iter().any(|m| m.name == method.name && m.is_abstract) {
                            implementations.push(Implementation {
                                file: file_metrics.path.clone(),
                                class_name: Some(class.name.clone()),
                                function_name: method.name.clone(),
                                line: method.line,
                            });
                        }
                    }
                }
            }
        }

        implementations
    }

    /// Check if class inherits from interface (possibly in different file)
    fn inherits_from_interface(
        &self,
        class: &ClassDef,
        interface: &ClassDef,
        class_file: &Path,
        interface_file: &Path,
        context: &CrossModuleContext,
    ) -> bool {
        // Direct inheritance (same file)
        if class.base_classes.contains(&interface.name) {
            return true;
        }

        // Cross-file inheritance via imports
        if let Some(imports) = context.imports.get(class_file) {
            for import in imports {
                // Check if interface is imported
                if import.symbol == interface.name {
                    // Resolve import to actual module
                    if let Some(resolved_path) = context.resolve_import(class_file, &import.module) {
                        if resolved_path == interface_file {
                            // Class imports interface and uses it as base
                            return class.base_classes.contains(&interface.name);
                        }
                    }
                }
            }
        }

        false
    }

    /// Find polymorphic invocations (for obs in observers: obs.method())
    fn find_polymorphic_invocations(
        &self,
        interface_name: &str,
        project_metrics: &ProjectMetrics,
    ) -> Vec<UsageSite> {
        let mut usage_sites = Vec::new();

        for file_metrics in &project_metrics.files {
            // Search AST for observer loop patterns
            if let Some(ast) = &file_metrics.ast {
                usage_sites.extend(self.find_observer_loops_in_ast(ast, interface_name));
            }
        }

        usage_sites
    }

    fn find_observer_loops_in_ast(&self, ast: &ast::Mod, interface_name: &str) -> Vec<UsageSite> {
        // Implementation similar to single-file detection
        // but searches for any method calls on observer collections
        Vec::new() // Placeholder
    }

    fn is_observer_interface(&self, class: &ClassDef) -> bool {
        let has_abc_base = class.base_classes.iter().any(|b| {
            b.contains("ABC") || b.contains("Protocol") || b.contains("Interface")
        });
        let has_abstract_methods = class.methods.iter().any(|m| m.is_abstract);
        has_abc_base && has_abstract_methods
    }
}
```

### Singleton Pattern Recognizer (Cross-File)

```rust
// src/analysis/patterns/singleton.rs
use super::{PatternInstance, PatternRecognizer, PatternType, Implementation, UsageSite};
use crate::analysis::python_call_graph::cross_module::CrossModuleContext;
use crate::core::{FileMetrics, FunctionDef, ModuleScopeAnalysis};
use std::sync::Arc;

pub struct SingletonPatternRecognizer {
    cross_module_context: Option<Arc<CrossModuleContext>>,
}

impl SingletonPatternRecognizer {
    pub fn new() -> Self {
        Self {
            cross_module_context: None,
        }
    }

    pub fn with_context(mut self, context: Arc<CrossModuleContext>) -> Self {
        self.cross_module_context = Some(context);
        self
    }

    /// Detect module-level singleton instances
    fn is_singleton_assignment(&self, assignment: &Assignment) -> bool {
        matches!(assignment.value, Expression::ClassInstantiation { .. })
            && assignment.scope == Scope::Module
    }

    /// Find all usages of singleton instance across files
    fn find_singleton_usages(
        &self,
        singleton_name: &str,
        singleton_file: &Path,
        project_metrics: &ProjectMetrics,
        context: &CrossModuleContext,
    ) -> Vec<UsageSite> {
        let mut usage_sites = Vec::new();

        // Find files that import this singleton
        for (file_path, imports) in &context.imports {
            for import in imports {
                if import.symbol == singleton_name {
                    // Check if import is from singleton's module
                    if let Some(resolved) = context.resolve_import(file_path, &import.module) {
                        if resolved == singleton_file {
                            // Found an import! Now find usages in this file
                            if let Some(file_metrics) = project_metrics.files.iter().find(|f| &f.path == file_path) {
                                usage_sites.extend(self.find_singleton_usages_in_file(
                                    singleton_name,
                                    file_metrics,
                                ));
                            }
                        }
                    }
                }
            }
        }

        usage_sites
    }

    fn find_singleton_usages_in_file(
        &self,
        singleton_name: &str,
        file_metrics: &FileMetrics,
    ) -> Vec<UsageSite> {
        let mut usage_sites = Vec::new();

        // Search AST for attribute access on singleton (singleton.method())
        if let Some(ast) = &file_metrics.ast {
            usage_sites.extend(self.find_attribute_calls(ast, singleton_name, &file_metrics.path));
        }

        usage_sites
    }

    fn find_attribute_calls(&self, ast: &ast::Mod, name: &str, file_path: &Path) -> Vec<UsageSite> {
        let mut usage_sites = Vec::new();

        if let ast::Mod::Module(module) = ast {
            for stmt in &module.body {
                self.traverse_for_attribute_calls(stmt, name, file_path, &mut usage_sites);
            }
        }

        usage_sites
    }

    fn traverse_for_attribute_calls(
        &self,
        stmt: &ast::Stmt,
        name: &str,
        file_path: &Path,
        usage_sites: &mut Vec<UsageSite>,
    ) {
        match stmt {
            ast::Stmt::Expr(expr_stmt) => {
                if let ast::Expr::Call(call) = &expr_stmt.value {
                    if let ast::Expr::Attribute(attr) = &*call.func {
                        if let ast::Expr::Name(n) = &*attr.value {
                            if n.id == name {
                                usage_sites.push(UsageSite {
                                    file: file_path.to_path_buf(),
                                    line: call.range.start.to_usize(),
                                    context: format!("{}.{}()", name, attr.attr),
                                });
                            }
                        }
                    }
                }
            }
            ast::Stmt::FunctionDef(func) => {
                for stmt in &func.body {
                    self.traverse_for_attribute_calls(stmt, name, file_path, usage_sites);
                }
            }
            _ => {}
        }
    }
}

impl PatternRecognizer for SingletonPatternRecognizer {
    fn name(&self) -> &str {
        "Singleton"
    }

    fn detect(&self, file_metrics: &FileMetrics) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        if let Some(module_scope) = &file_metrics.module_scope {
            for singleton in &module_scope.singleton_instances {
                patterns.push(PatternInstance {
                    pattern_type: PatternType::Singleton,
                    confidence: 0.9,
                    base_class: Some(singleton.class_name.clone()),
                    implementations: vec![Implementation {
                        file: file_metrics.path.clone(),
                        class_name: Some(singleton.class_name.clone()),
                        function_name: singleton.variable_name.clone(),
                        line: singleton.line,
                    }],
                    usage_sites: Vec::new(), // Cross-file usage requires ProjectMetrics
                    reasoning: format!(
                        "Module-level singleton: {} = {}()",
                        singleton.variable_name, singleton.class_name
                    ),
                });
            }
        }

        patterns
    }

    fn is_function_used_by_pattern(
        &self,
        function: &FunctionDef,
        file_metrics: &FileMetrics,
    ) -> Option<PatternInstance> {
        // Check if function is a method on a singleton class
        let class_name = function.class_name.as_ref()?;

        if let Some(module_scope) = &file_metrics.module_scope {
            // Check if class has a singleton instance
            if module_scope.singleton_instances.iter().any(|s| &s.class_name == class_name) {
                return Some(PatternInstance {
                    pattern_type: PatternType::Singleton,
                    confidence: 0.85,
                    base_class: Some(class_name.clone()),
                    implementations: vec![Implementation {
                        file: file_metrics.path.clone(),
                        class_name: Some(class_name.clone()),
                        function_name: function.name.clone(),
                        line: function.start_line,
                    }],
                    usage_sites: Vec::new(),
                    reasoning: format!("Method on singleton class {}", class_name),
                });
            }
        }

        None
    }
}
```

### Integration with Dead Code Detector

```rust
// src/debt/dead_code.rs
use crate::analysis::patterns::PatternDetector;

pub struct DeadCodeDetector {
    pattern_detector: PatternDetector,
}

impl DeadCodeDetector {
    pub fn new(pattern_detector: PatternDetector) -> Self {
        Self { pattern_detector }
    }

    pub fn detect_dead_code(
        &self,
        function: &FunctionDef,
        file_metrics: &FileMetrics,
        call_graph: &CallGraph,
    ) -> Option<DeadCodeFinding> {
        // Fast path: If function has direct callers, it's not dead
        if call_graph.has_callers(&function.id) {
            return None;
        }

        // Check if function is used via design pattern
        if let Some(pattern) = self.pattern_detector.is_function_used_by_pattern(function, file_metrics) {
            // Function is used via pattern - NOT dead code
            return None;
        }

        // No direct callers and no pattern usage - likely dead code
        Some(DeadCodeFinding {
            function: function.clone(),
            confidence: 0.8,
            reasoning: "No direct callers and no pattern usage detected".to_string(),
        })
    }
}
```

### Rust Trait Pattern Integration

```rust
// src/analysis/patterns/rust_traits.rs
use crate::analysis::call_graph::trait_registry::TraitRegistry;
use super::{PatternInstance, PatternType, Implementation};

pub struct RustTraitPatternRecognizer {
    trait_registry: Arc<TraitRegistry>,
}

impl RustTraitPatternRecognizer {
    pub fn new(trait_registry: Arc<TraitRegistry>) -> Self {
        Self { trait_registry }
    }

    /// Detect trait-based observer pattern in Rust
    pub fn detect_trait_observer_patterns(&self) -> Vec<PatternInstance> {
        let mut patterns = Vec::new();

        // Iterate over all traits
        for (trait_name, trait_methods) in self.trait_registry.trait_definitions.iter() {
            // Find implementations of this trait
            if let Some(implementations) = self.trait_registry.find_implementations(trait_name) {
                if !implementations.is_empty() {
                    patterns.push(PatternInstance {
                        pattern_type: PatternType::Observer,
                        confidence: 0.95,
                        base_class: Some(trait_name.clone()),
                        implementations: implementations.iter().map(|impl_info| {
                            Implementation {
                                file: impl_info.file.clone(),
                                class_name: Some(impl_info.type_name.clone()),
                                function_name: trait_name.clone(),
                                line: impl_info.line,
                            }
                        }).collect(),
                        usage_sites: self.find_trait_method_calls(trait_name),
                        reasoning: format!("Rust trait {} with {} implementations", trait_name, implementations.len()),
                    });
                }
            }
        }

        patterns
    }

    fn find_trait_method_calls(&self, trait_name: &str) -> Vec<UsageSite> {
        // Use TraitRegistry to find polymorphic trait method calls
        self.trait_registry
            .unresolved_calls
            .iter()
            .filter(|call| call.trait_name == trait_name)
            .map(|call| UsageSite {
                file: call.file.clone(),
                line: call.line,
                context: format!("Trait method call: {}", call.method_name),
            })
            .collect()
    }
}
```

## Dependencies

- **Prerequisites**: Spec 114a (Parser Enhancements), Spec 114b (Local Pattern Detection)
- **Affected Components**:
  - `src/analysis/patterns/mod.rs` - Enhanced PatternDetector
  - `src/analysis/patterns/singleton.rs` - Cross-file singleton detection
  - `src/analysis/patterns/rust_traits.rs` - **New** Rust trait pattern integration
  - `src/debt/dead_code.rs` - Integrate pattern detection
- **External Dependencies**: None (uses existing infrastructure)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_file_observer_detection() {
        let project_metrics = create_test_project(vec![
            ("observer.py", "class Observer(ABC): @abstractmethod\n def on_event(self): pass"),
            ("impl.py", "from observer import Observer\nclass ConcreteObserver(Observer):\n def on_event(self): print('event')"),
        ]);

        let detector = PatternDetector::new()
            .with_cross_module_context(Arc::new(create_test_context()));

        let patterns = detector.detect_project_patterns(&project_metrics);

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, PatternType::Observer);
        assert_eq!(patterns[0].implementations.len(), 1);
    }

    #[test]
    fn test_singleton_cross_file_usage() {
        let project_metrics = create_test_project(vec![
            ("manager.py", "class Manager:\n def process(self): pass\nmanager = Manager()"),
            ("client.py", "from manager import manager\nmanager.process()"),
        ]);

        let detector = PatternDetector::new()
            .with_cross_module_context(Arc::new(create_test_context()));

        let patterns = detector.detect_project_patterns(&project_metrics);

        let singleton_patterns: Vec<_> = patterns.iter()
            .filter(|p| p.pattern_type == PatternType::Singleton)
            .collect();

        assert_eq!(singleton_patterns.len(), 1);
        assert!(!singleton_patterns[0].usage_sites.is_empty());
    }
}
```

### Integration Tests

Create multi-file test fixtures:

```python
# tests/fixtures/patterns/cross_file_observer/observer.py
from abc import ABC, abstractmethod

class Observer(ABC):
    @abstractmethod
    def on_event(self, data):
        pass

# tests/fixtures/patterns/cross_file_observer/impl.py
from observer import Observer

class ConcreteObserver(Observer):
    def on_event(self, data):
        print(f"Event: {data}")

# tests/fixtures/patterns/cross_file_observer/manager.py
from observer import Observer

class Manager:
    def __init__(self):
        self.observers = []

    def add_observer(self, observer: Observer):
        self.observers.append(observer)

    def notify(self, data):
        for observer in self.observers:
            observer.on_event(data)
```

Expected: `ConcreteObserver.on_event()` NOT flagged as dead code.

## Documentation Requirements

- Document cross-file pattern detection algorithm
- Explain integration with CrossModuleContext
- Explain integration with TraitRegistry
- Update ARCHITECTURE.md with cross-file pattern flow
- Provide examples of cross-file pattern detection

## Implementation Notes

### Cross-File Resolution
- Use `CrossModuleContext.resolve_import()` for symbol resolution
- Leverage `CrossModuleContext.namespaces` for import tracking
- Cache resolved patterns to avoid repeated lookups

### Performance Optimization
- Build pattern index once per project analysis
- Use existing call graph for invocation detection
- Avoid redundant AST traversals

### Error Handling
- Handle missing imports gracefully
- Fallback to conservative detection on resolution errors
- Log warnings for unresolved patterns

## Success Metrics

- [ ] Cross-file observer pattern detection: 90%+ accuracy
- [ ] Singleton cross-file usage tracking: 85%+ accuracy
- [ ] False positive rate < 5% on promptconstruct-frontend
- [ ] Performance overhead < 5%
- [ ] Real-world bug report examples resolved
