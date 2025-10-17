---
number: 114a
title: Pattern Parser Enhancements
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-16
---

# Specification 114a: Pattern Parser Enhancements

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None (prerequisite for 114b, 114c, 114d)

## Context

This is Phase 1 of the Design Pattern Recognition feature (Spec 114). Before debtmap can detect design patterns, language analyzers must extract additional AST information about decorators, inheritance, module-level assignments, and callback registrations.

**Current Gap**: The Python analyzer extracts basic function and class definitions but doesn't capture:
- Class decorators (`@dataclass`, `@abstractmethod`)
- Base class relationships (inheritance)
- Method decorators
- Module-level variable assignments
- Callback registration patterns

**Why This is Critical**: Pattern detection requires rich AST metadata. Without decorator and inheritance information, we cannot identify observer interfaces, singleton patterns, or factory methods.

## Objective

Enhance Python, Rust, and JavaScript/TypeScript analyzers to extract pattern-relevant AST information, creating the foundation for pattern recognition in subsequent phases.

## Requirements

### Functional Requirements

#### Python Analyzer Enhancements

1. **Class Decorators Extraction**
   - Extract decorators from class definitions
   - Store as `Vec<String>` on class metadata
   - Support: `@dataclass`, `@abstractmethod`, custom decorators
   - Preserve decorator ordering

2. **Base Classes Tracking**
   - Extract from `ClassDef.bases` AST node
   - Store qualified names (e.g., `abc.ABC`, `Protocol`)
   - Resolve import aliases for base classes
   - Track multiple inheritance chains

3. **Method Decorators Extraction**
   - Extract decorators from method definitions
   - Support: `@abstractmethod`, `@property`, `@staticmethod`, `@classmethod`
   - Framework decorators: `@app.route`, `@handler`, `@cached`
   - Store on method metadata

4. **Module-Level Assignments**
   - Track top-level variable assignments
   - Detect pattern: `instance = ClassName(args)`
   - Distinguish class instantiation from other assignments
   - Store in `ModuleScopeAnalysis` struct
   - Track assignment line numbers

5. **Callback Registrations**
   - Identify AST patterns: `obj.method_name = func`
   - Detect method calls: `.on()`, `.subscribe()`, `.register()`
   - Extract receiver, method, and callback function
   - Track event handler decorators

#### Rust Analyzer Enhancements

**Already Implemented** via `TraitRegistry`:
- ✅ Trait definitions and implementations
- ✅ Trait method calls (polymorphic invocations)
- ✅ Visitor pattern detection (syn::visit::Visit)
- ✅ Cross-module trait resolution

**Additional Requirements**:
- Track `Arc<Mutex<T>>` patterns for singleton detection
- Identify builder patterns (type-state builders)
- Detect framework-specific patterns (Actix handlers, etc.)

#### JavaScript/TypeScript Analyzer Enhancements

**Required Extractions**:
1. **Prototype Chain**: Track `prototype` assignments and `extends` clauses
2. **Class Decorators**: Extract TypeScript decorators (`@Component`, `@Injectable`)
3. **Callback Patterns**: Identify `.on()`, `.addEventListener()`, `.subscribe()`
4. **Factory Functions**: Detect functions returning different class instances

### Non-Functional Requirements

1. **Performance**
   - AST extraction adds < 5% to parse time
   - Use efficient data structures (Vec, HashMap)
   - Avoid unnecessary allocations

2. **Correctness**
   - 100% accuracy in decorator extraction
   - Proper handling of complex inheritance
   - Correct resolution of import aliases

3. **Maintainability**
   - Clean separation of AST extraction logic
   - Reusable data structures for pattern detection
   - Well-documented AST traversal code

## Acceptance Criteria

### Python Analyzer
- [ ] Extract class decorators into `Vec<String>` field on `ClassDef`
- [ ] Extract base classes with qualified names
- [ ] Extract method decorators into `Vec<String>` field on `MethodDef`
- [ ] Detect module-level class instantiations
- [ ] Store module-level assignments in dedicated struct
- [ ] Identify callback registration patterns
- [ ] Unit tests for each extraction type
- [ ] Integration test with real Python code

### Rust Analyzer
- [ ] Track `Arc<Mutex<T>>` singleton patterns
- [ ] Identify builder patterns
- [ ] Detect Actix/Rocket handler patterns
- [ ] Unit tests for new extractions

### JavaScript/TypeScript Analyzer
- [ ] Extract prototype chain information
- [ ] Extract TypeScript decorators
- [ ] Identify callback patterns
- [ ] Unit tests for each extraction

### General
- [ ] Parse time increase < 5%
- [ ] All existing tests still pass
- [ ] Documentation for new data structures

## Technical Details

### Python AST Extraction

#### Data Structures

```rust
// src/core/ast.rs - Enhanced class definition
#[derive(Debug, Clone)]
pub struct ClassDef {
    pub name: String,
    pub base_classes: Vec<String>,  // NEW: Base class names
    pub methods: Vec<MethodDef>,
    pub is_abstract: bool,
    pub decorators: Vec<String>,    // NEW: Class decorators
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct MethodDef {
    pub name: String,
    pub is_abstract: bool,
    pub decorators: Vec<String>,    // NEW: Method decorators
    pub overrides_base: bool,       // NEW: Inheritance tracking
    pub line: usize,
}

// NEW: Module-level analysis
#[derive(Debug, Clone)]
pub struct ModuleScopeAnalysis {
    pub assignments: Vec<Assignment>,
    pub singleton_instances: Vec<SingletonInstance>,
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
    ClassInstantiation { class_name: String, args: Vec<String> },
    FunctionCall { function_name: String, args: Vec<String> },
    ClassReference { class_name: String },
    Literal { value: String },
    Other,
}

impl Expression {
    pub fn is_class_instantiation(&self) -> bool {
        matches!(self, Expression::ClassInstantiation { .. })
    }

    pub fn is_class_reference(&self) -> bool {
        matches!(self, Expression::ClassReference { .. })
    }
}

#[derive(Debug, Clone)]
pub struct SingletonInstance {
    pub variable_name: String,
    pub class_name: String,
    pub line: usize,
}
```

#### AST Traversal Implementation

```rust
// src/analyzers/python_ast_extraction.rs
use rustpython_parser::ast;
use crate::core::{ClassDef, MethodDef, ModuleScopeAnalysis, Assignment, Expression, Scope};

pub struct PythonAstExtractor {
    current_scope: Scope,
}

impl PythonAstExtractor {
    pub fn new() -> Self {
        Self {
            current_scope: Scope::Module,
        }
    }

    /// Extract class decorators from ClassDef AST node
    pub fn extract_class_decorators(&self, class_def: &ast::StmtClassDef) -> Vec<String> {
        class_def
            .decorator_list
            .iter()
            .filter_map(|decorator| self.decorator_to_string(decorator))
            .collect()
    }

    /// Extract base classes from ClassDef AST node
    pub fn extract_base_classes(&self, class_def: &ast::StmtClassDef) -> Vec<String> {
        class_def
            .bases
            .iter()
            .filter_map(|base| self.expr_to_name(base))
            .collect()
    }

    /// Extract method decorators from FunctionDef AST node
    pub fn extract_method_decorators(&self, func_def: &ast::StmtFunctionDef) -> Vec<String> {
        func_def
            .decorator_list
            .iter()
            .filter_map(|decorator| self.decorator_to_string(decorator))
            .collect()
    }

    /// Check if method is abstract (has @abstractmethod decorator)
    pub fn is_abstract_method(&self, decorators: &[String]) -> bool {
        decorators.iter().any(|d| d == "abstractmethod")
    }

    /// Extract module-level assignments
    pub fn extract_module_assignments(&self, module: &ast::Mod) -> Vec<Assignment> {
        let mut assignments = Vec::new();

        if let ast::Mod::Module(mod_body) = module {
            for stmt in &mod_body.body {
                if let ast::Stmt::Assign(assign) = stmt {
                    for target in &assign.targets {
                        if let Some(name) = self.expr_to_name(target) {
                            let value = self.classify_expression(&assign.value);
                            assignments.push(Assignment {
                                name,
                                value,
                                scope: Scope::Module,
                                line: assign.range.start.to_usize(),
                            });
                        }
                    }
                }
            }
        }

        assignments
    }

    /// Classify an expression (ClassInstantiation, FunctionCall, etc.)
    fn classify_expression(&self, expr: &ast::Expr) -> Expression {
        match expr {
            ast::Expr::Call(call) => {
                if let Some(name) = self.expr_to_name(&call.func) {
                    // Check if it's a class instantiation (capitalized name)
                    if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        Expression::ClassInstantiation {
                            class_name: name,
                            args: call.arguments.args.iter()
                                .filter_map(|arg| self.expr_to_name(arg))
                                .collect(),
                        }
                    } else {
                        Expression::FunctionCall {
                            function_name: name,
                            args: call.arguments.args.iter()
                                .filter_map(|arg| self.expr_to_name(arg))
                                .collect(),
                        }
                    }
                } else {
                    Expression::Other
                }
            }
            ast::Expr::Name(name) => {
                Expression::ClassReference {
                    class_name: name.id.to_string(),
                }
            }
            _ => Expression::Other,
        }
    }

    /// Convert decorator expression to string
    fn decorator_to_string(&self, expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::Name(name) => Some(name.id.to_string()),
            ast::Expr::Attribute(attr) => {
                // Handle chained attributes like @abc.abstractmethod
                let base = self.expr_to_name(&attr.value)?;
                Some(format!("{}.{}", base, attr.attr))
            }
            ast::Expr::Call(call) => {
                // Handle decorators with arguments like @dataclass(frozen=True)
                self.expr_to_name(&call.func)
            }
            _ => None,
        }
    }

    /// Convert expression to name string
    fn expr_to_name(&self, expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::Name(name) => Some(name.id.to_string()),
            ast::Expr::Attribute(attr) => {
                let base = self.expr_to_name(&attr.value)?;
                Some(format!("{}.{}", base, attr.attr))
            }
            _ => None,
        }
    }

    /// Identify singleton instances (module-level class instantiation)
    pub fn extract_singleton_instances(&self, assignments: &[Assignment]) -> Vec<SingletonInstance> {
        assignments
            .iter()
            .filter_map(|assignment| {
                if let Expression::ClassInstantiation { class_name, .. } = &assignment.value {
                    if assignment.scope == Scope::Module {
                        return Some(SingletonInstance {
                            variable_name: assignment.name.clone(),
                            class_name: class_name.clone(),
                            line: assignment.line,
                        });
                    }
                }
                None
            })
            .collect()
    }
}
```

### Integration with PythonAnalyzer

```rust
// src/analyzers/python.rs - Enhanced analyze function
impl Analyzer for PythonAnalyzer {
    fn analyze(&self, ast: &Ast) -> FileMetrics {
        match ast {
            Ast::Python(python_ast) => {
                let extractor = PythonAstExtractor::new();

                // Extract module-level assignments
                let assignments = extractor.extract_module_assignments(&python_ast.module);
                let singleton_instances = extractor.extract_singleton_instances(&assignments);

                // Enhanced class analysis with decorators and base classes
                let classes = self.analyze_classes_enhanced(&python_ast.module, &extractor);

                // Store in FileMetrics
                let mut metrics = analyze_python_file(python_ast, self.complexity_threshold);
                metrics.module_scope = Some(ModuleScopeAnalysis {
                    assignments,
                    singleton_instances,
                });
                metrics.classes = classes;

                metrics
            }
            _ => panic!("Expected Python AST"),
        }
    }
}

impl PythonAnalyzer {
    fn analyze_classes_enhanced(
        &self,
        module: &ast::Mod,
        extractor: &PythonAstExtractor,
    ) -> Vec<ClassDef> {
        let mut classes = Vec::new();

        if let ast::Mod::Module(mod_body) = module {
            for stmt in &mod_body.body {
                if let ast::Stmt::ClassDef(class_def) = stmt {
                    let decorators = extractor.extract_class_decorators(class_def);
                    let base_classes = extractor.extract_base_classes(class_def);

                    let methods = self.analyze_methods_enhanced(&class_def.body, extractor);
                    let is_abstract = base_classes.iter().any(|b| b.contains("ABC") || b.contains("Protocol"))
                        || methods.iter().any(|m| m.is_abstract);

                    classes.push(ClassDef {
                        name: class_def.name.to_string(),
                        base_classes,
                        methods,
                        is_abstract,
                        decorators,
                        line: class_def.range.start.to_usize(),
                    });
                }
            }
        }

        classes
    }

    fn analyze_methods_enhanced(
        &self,
        body: &[ast::Stmt],
        extractor: &PythonAstExtractor,
    ) -> Vec<MethodDef> {
        let mut methods = Vec::new();

        for stmt in body {
            if let ast::Stmt::FunctionDef(func_def) = stmt {
                let decorators = extractor.extract_method_decorators(func_def);
                let is_abstract = extractor.is_abstract_method(&decorators);

                methods.push(MethodDef {
                    name: func_def.name.to_string(),
                    is_abstract,
                    decorators,
                    overrides_base: false, // Will be determined by pattern detector
                    line: func_def.range.start.to_usize(),
                });
            }
        }

        methods
    }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/core/ast.rs` - Add new data structures
  - `src/analyzers/python.rs` - Enhance Python analyzer
  - `src/analyzers/python_ast_extraction.rs` - **New module** for AST extraction
- **External Dependencies**: Uses existing `rustpython_parser`

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_class_decorators() {
        let code = r#"
@dataclass
@frozen
class Point:
    x: int
    y: int
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let decorators = extractor.extract_class_decorators(&get_first_class(&ast));

        assert_eq!(decorators, vec!["dataclass", "frozen"]);
    }

    #[test]
    fn test_extract_base_classes() {
        let code = r#"
class Observer(ABC, Protocol):
    pass
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let bases = extractor.extract_base_classes(&get_first_class(&ast));

        assert_eq!(bases, vec!["ABC", "Protocol"]);
    }

    #[test]
    fn test_extract_method_decorators() {
        let code = r#"
class Observer:
    @abstractmethod
    @property
    def on_event(self):
        pass
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let decorators = extractor.extract_method_decorators(&get_first_method(&ast));

        assert_eq!(decorators, vec!["abstractmethod", "property"]);
    }

    #[test]
    fn test_extract_module_assignments() {
        let code = r#"
manager = Manager()
config = load_config()
VALUE = 42
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let assignments = extractor.extract_module_assignments(&ast);

        assert_eq!(assignments.len(), 3);
        assert_eq!(assignments[0].name, "manager");
        assert!(matches!(assignments[0].value, Expression::ClassInstantiation { .. }));
    }

    #[test]
    fn test_extract_singleton_instances() {
        let code = r#"
class Manager:
    pass

manager = Manager()  # Singleton
        "#;

        let ast = parse_python_code(code);
        let extractor = PythonAstExtractor::new();
        let assignments = extractor.extract_module_assignments(&ast);
        let singletons = extractor.extract_singleton_instances(&assignments);

        assert_eq!(singletons.len(), 1);
        assert_eq!(singletons[0].variable_name, "manager");
        assert_eq!(singletons[0].class_name, "Manager");
    }
}
```

### Integration Tests

Test with real-world Python code:

```python
# tests/fixtures/pattern_extraction/observer.py
from abc import ABC, abstractmethod

@dataclass
class Event:
    name: str
    data: dict

class Observer(ABC):
    @abstractmethod
    def on_event(self, event: Event):
        pass

class ConcreteObserver(Observer):
    def on_event(self, event: Event):
        print(f"Received: {event.name}")

# Module-level singleton
manager = EventManager()
```

Expected extraction:
- `Event` class with `@dataclass` decorator
- `Observer` class with base class `ABC`
- `on_event` method with `@abstractmethod` decorator
- Module-level singleton: `manager = EventManager()`

## Documentation Requirements

- Document new `ClassDef` and `MethodDef` fields
- Explain AST extraction process
- Provide examples of extracted data
- Update ARCHITECTURE.md with new data structures

## Implementation Notes

### AST Traversal Best Practices
- Use pattern matching on AST nodes
- Handle `None` cases gracefully
- Preserve original line numbers
- Avoid unnecessary allocations

### Decorator Resolution
- Simple decorators: `@dataclass` → `"dataclass"`
- Attribute decorators: `@abc.abstractmethod` → `"abc.abstractmethod"`
- Call decorators: `@dataclass(frozen=True)` → `"dataclass"`

### Scope Tracking
- Track current scope during traversal
- Mark module-level vs class-level vs function-level assignments
- Essential for singleton detection

## Success Metrics

- [ ] Parse time increase < 5%
- [ ] 100% accuracy in decorator extraction (verified with test suite)
- [ ] All existing tests pass
- [ ] Integration tests with real Python projects succeed
