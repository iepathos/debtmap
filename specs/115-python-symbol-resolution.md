---
number: 115
title: Python Symbol Resolution and Error Detection
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-16
revised: 2025-10-18
---

# Specification 115: Python Symbol Resolution and Error Detection

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap v0.2.8 detects complexity and dead code but misses basic static errors like undefined variables and missing imports. This causes misdiagnosis—flagging complex code that's actually broken.

### Real Bugs Missed

**Issue #5**: Undefined variable in `conversation_panel.py:595`
```python
def on_message_added(self, message, index):
    if message is messages[index].message:  # ❌ 'messages' undefined
        return True                         # Should be 'self.messages'
```

**Issue #9**: Missing import in `delivery_boy.py`
```python
def deliver(observers, message, index):
    wx.CallAfter(deliver, observers, message, index)  # ❌ 'wx' not imported
```

### Current State

- **Parser**: `rustpython-parser = "0.4"` → `ast::Mod`
- **Import tracking**: `EnhancedImportResolver` exists in `src/analysis/python_imports.rs`
- **Symbol tracking**: None (gap)
- **Error detection**: None (gap)

### Why This Matters

1. **Broken code > complex code** - Higher priority
2. **Native capability** - Core feature, not external dependency
3. **Better classification** - "Won't run" vs "Hard to maintain"

## Objective

Extend Python analysis to detect static errors:
1. Undefined variables (Issue #5)
2. Missing imports (Issue #9)
3. Integrate with existing `EnhancedImportResolver`

**Non-goals for MVP**: Unreachable code, type checking, cross-file resolution

## Requirements

### MVP Scope

**Must Have**:
- Detect undefined variables in function scopes (Issue #5)
- Detect missing module imports (Issue #9)
- Track local symbols (parameters, assignments, loop vars)
- Generate `DebtItem` for each error
- All functions ≤ 20 lines

**Deferred**:
- Unreachable code detection
- Cross-file resolution
- `global`/`nonlocal` support
- Comprehension scopes
- Class attribute tracking

### Functional Requirements

1. **Local Symbol Tracking**
   - Function parameters
   - Local assignments (`x = 1`)
   - Loop variables (`for x in items`)
   - Exception variables (`except E as e`)
   - Context variables (`with f as fp`)

2. **Undefined Variable Detection**
   - Check `Name` nodes with `ctx=Load`
   - Verify against: local symbols + module imports + builtins
   - Filter false positives: `self`, `cls`
   - Report: line, column, function context

3. **Missing Import Detection**
   - Detect `module.attr` where `module` undefined
   - Check against `EnhancedImportResolver`
   - Suggest `import module`

4. **DebtItem Integration**
   - Convert errors to `DebtItem`
   - Add to `FileMetrics.debt_items`
   - Set appropriate `DebtType` and `Priority`

### Non-Functional Requirements

- **Performance**: < 10% overhead
- **Accuracy**: < 5% false positives
- **Code quality**: All functions ≤ 20 lines, pure where possible
- **Testing**: > 90% coverage

## Acceptance Criteria

**Success = All tests pass**:
- [ ] Issue #5 undefined variable detected
- [ ] Issue #9 missing import detected
- [ ] `self`, `cls` not flagged as undefined
- [ ] Python builtins not flagged
- [ ] Loop/exception/context variables tracked
- [ ] Errors converted to `DebtItem` with location
- [ ] Performance overhead < 10%
- [ ] False positive rate < 5%
- [ ] All functions ≤ 20 lines
- [ ] Test coverage > 90%

## RustPython AST Reference

### Key Types

```rust
// From rustpython_parser::ast

pub enum Mod {
    Module(ModModule),  // Normal Python file
}

pub struct ModModule {
    pub body: Vec<Stmt>,
    pub range: TextRange,
}

pub enum Stmt {
    FunctionDef(StmtFunctionDef),
    Assign(StmtAssign),
    For(StmtFor),
    With(StmtWith),
    // ...others
}

pub struct StmtFunctionDef {
    pub name: Identifier,
    pub parameters: Box<Parameters>,
    pub body: Vec<Stmt>,
    pub range: TextRange,
}

pub struct Parameters {
    pub args: Vec<ParameterWithDefault>,
    pub vararg: Option<Box<Parameter>>,
    pub kwonlyargs: Vec<ParameterWithDefault>,
    pub kwarg: Option<Box<Parameter>>,
}

pub enum Expr {
    Name(ExprName),        // Variable reference
    Attribute(ExprAttribute),  // obj.attr
    Call(ExprCall),
}

pub struct ExprName {
    pub id: Identifier,
    pub ctx: ExprContext,  // Load, Store, Del
    pub range: TextRange,
}

pub enum ExprContext {
    Load,   // Reading
    Store,  // Writing
    Del,    // Deleting
}
```

### Traversal Pattern

Existing code uses **recursive pattern matching** (not visitor traits):

```rust
// Example from src/analyzers/python_ast_extraction.rs
fn extract_module_assignments(&self, module: &ast::Mod) -> Vec<Assignment> {
    if let ast::Mod::Module(mod_body) = module {
        for stmt in &mod_body.body {
            if let ast::Stmt::Assign(assign) = stmt {
                // Process
            }
        }
    }
}
```

## Technical Design

### Module Structure

```
src/analysis/
└── python_static_errors.rs  # NEW: ~400 lines

src/core/
└── mod.rs                    # UPDATE: Add DebtType::StaticError
```

### Core Types

```rust
// src/analysis/python_static_errors.rs

use rustpython_parser::ast;
use std::collections::HashSet;

/// Function-local symbol table
#[derive(Debug, Clone, Default)]
pub struct LocalSymbols {
    symbols: HashSet<String>,
}

impl LocalSymbols {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: String) {
        self.symbols.insert(name);
    }

    pub fn contains(&self, name: &str) -> bool {
        self.symbols.contains(name)
    }
}

/// Static error types
#[derive(Debug, Clone, PartialEq)]
pub enum StaticError {
    UndefinedVariable {
        name: String,
        line: usize,
        column: usize,
        function: String,
    },
    MissingImport {
        module: String,
        line: usize,
        usage: String,
    },
}

/// Analysis result
#[derive(Debug, Clone, Default)]
pub struct StaticAnalysisResult {
    pub errors: Vec<StaticError>,
}
```

### Implementation: Small Pure Functions

All functions ≤ 20 lines, following functional programming principles:

```rust
// src/analysis/python_static_errors.rs

use rustpython_parser::ast;
use crate::analysis::python_imports::EnhancedImportResolver;

/// Main entry point: Analyze module for static errors
pub fn analyze_static_errors(
    module: &ast::Mod,
    import_resolver: &EnhancedImportResolver,
) -> StaticAnalysisResult {
    let ast::Mod::Module(mod_module) = module else {
        return StaticAnalysisResult::default();
    };

    let builtins = python_builtins();
    let mut errors = Vec::new();

    for stmt in &mod_module.body {
        if let ast::Stmt::FunctionDef(func) = stmt {
            errors.extend(analyze_function(func, import_resolver, &builtins));
        }
    }

    StaticAnalysisResult { errors }
}

/// Analyze single function for errors
fn analyze_function(
    func: &ast::StmtFunctionDef,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
) -> Vec<StaticError> {
    let mut symbols = extract_local_symbols(func);
    find_undefined_names(&func.body, &func.name.to_string(), &symbols, resolver, builtins)
}

/// Extract all locally defined symbols from function
fn extract_local_symbols(func: &ast::StmtFunctionDef) -> LocalSymbols {
    let mut symbols = LocalSymbols::new();

    // Add parameters
    for param in extract_parameters(&func.parameters) {
        symbols.insert(param);
    }

    // Add local assignments, loop vars, etc.
    collect_definitions(&func.body, &mut symbols);

    symbols
}

/// Extract parameter names from function signature
fn extract_parameters(params: &ast::Parameters) -> Vec<String> {
    let mut names = Vec::new();

    for arg in &params.args {
        names.push(arg.parameter.name.to_string());
    }

    if let Some(vararg) = &params.vararg {
        names.push(vararg.name.to_string());
    }

    for kwarg in &params.kwonlyargs {
        names.push(kwarg.parameter.name.to_string());
    }

    if let Some(kwarg) = &params.kwarg {
        names.push(kwarg.name.to_string());
    }

    names
}

/// Collect variable definitions from statements
fn collect_definitions(stmts: &[ast::Stmt], symbols: &mut LocalSymbols) {
    for stmt in stmts {
        match stmt {
            ast::Stmt::Assign(assign) => collect_from_assign(assign, symbols),
            ast::Stmt::For(for_stmt) => collect_from_for(for_stmt, symbols),
            ast::Stmt::With(with) => collect_from_with(with, symbols),
            ast::Stmt::Try(try_stmt) => collect_from_try(try_stmt, symbols),
            _ => {}
        }
    }
}

/// Extract names from assignment targets
fn collect_from_assign(assign: &ast::StmtAssign, symbols: &mut LocalSymbols) {
    for target in &assign.targets {
        if let ast::Expr::Name(name) = target {
            symbols.insert(name.id.to_string());
        }
    }
}

/// Extract loop variable from for statement
fn collect_from_for(for_stmt: &ast::StmtFor, symbols: &mut LocalSymbols) {
    if let ast::Expr::Name(name) = &*for_stmt.target {
        symbols.insert(name.id.to_string());
    }
    collect_definitions(&for_stmt.body, symbols);
}

/// Extract context manager variable
fn collect_from_with(with: &ast::StmtWith, symbols: &mut LocalSymbols) {
    for item in &with.items {
        if let Some(ast::Expr::Name(name)) = &item.optional_vars {
            symbols.insert(name.id.to_string());
        }
    }
    collect_definitions(&with.body, symbols);
}

/// Extract exception variables
fn collect_from_try(try_stmt: &ast::StmtTry, symbols: &mut LocalSymbols) {
    for handler in &try_stmt.handlers {
        if let Some(name) = &handler.name {
            symbols.insert(name.to_string());
        }
        collect_definitions(&handler.body, symbols);
    }
}

/// Find all undefined name references in function body
fn find_undefined_names(
    stmts: &[ast::Stmt],
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
) -> Vec<StaticError> {
    let mut errors = Vec::new();

    for stmt in stmts {
        errors.extend(check_stmt_for_undefined(stmt, func_name, symbols, resolver, builtins));
    }

    errors
}

/// Check single statement for undefined references
fn check_stmt_for_undefined(
    stmt: &ast::Stmt,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
) -> Vec<StaticError> {
    let mut errors = Vec::new();

    // Check expressions in statement
    for expr in extract_expressions(stmt) {
        errors.extend(check_expr_for_undefined(expr, func_name, symbols, resolver, builtins));
    }

    errors
}

/// Check expression for undefined references
fn check_expr_for_undefined(
    expr: &ast::Expr,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
) -> Vec<StaticError> {
    match expr {
        ast::Expr::Name(name) if name.ctx.is_load() => {
            check_name_reference(name, func_name, symbols, builtins)
        }
        ast::Expr::Attribute(attr) => {
            check_attribute_access(attr, resolver)
        }
        _ => Vec::new(),
    }
}

/// Check if name reference is defined
fn check_name_reference(
    name: &ast::ExprName,
    func_name: &str,
    symbols: &LocalSymbols,
    builtins: &HashSet<String>,
) -> Vec<StaticError> {
    let name_str = name.id.to_string();

    if is_false_positive(&name_str) {
        return Vec::new();
    }

    if symbols.contains(&name_str) || builtins.contains(&name_str) {
        return Vec::new();
    }

    vec![StaticError::UndefinedVariable {
        name: name_str,
        line: name.range.start().to_usize(),
        column: name.range.start().column().to_usize(),
        function: func_name.to_string(),
    }]
}

/// Check attribute access for missing imports
fn check_attribute_access(
    attr: &ast::ExprAttribute,
    resolver: &EnhancedImportResolver,
) -> Vec<StaticError> {
    if let ast::Expr::Name(base) = &*attr.value {
        let module_name = base.id.to_string();

        if !is_module_imported(&module_name, resolver) {
            return vec![StaticError::MissingImport {
                module: module_name.clone(),
                line: attr.range.start().to_usize(),
                usage: format!("{}.{}", module_name, attr.attr),
            }];
        }
    }

    Vec::new()
}

/// Check if name should be filtered as false positive
fn is_false_positive(name: &str) -> bool {
    matches!(name, "self" | "cls")
}

/// Check if module is imported
fn is_module_imported(module: &str, resolver: &EnhancedImportResolver) -> bool {
    // Use existing EnhancedImportResolver's symbol resolution
    resolver.resolve_symbol(Path::new(""), module).is_some()
}

/// Get Python 3.8+ builtins
fn python_builtins() -> HashSet<String> {
    [
        // Functions
        "abs", "all", "any", "ascii", "bin", "bool", "breakpoint",
        "bytearray", "bytes", "callable", "chr", "classmethod",
        "compile", "complex", "delattr", "dict", "dir", "divmod",
        "enumerate", "eval", "exec", "filter", "float", "format",
        "frozenset", "getattr", "globals", "hasattr", "hash", "help",
        "hex", "id", "input", "int", "isinstance", "issubclass",
        "iter", "len", "list", "locals", "map", "max", "memoryview",
        "min", "next", "object", "oct", "open", "ord", "pow", "print",
        "property", "range", "repr", "reversed", "round", "set",
        "setattr", "slice", "sorted", "staticmethod", "str", "sum",
        "super", "tuple", "type", "vars", "zip", "__import__",
        // Constants
        "True", "False", "None", "NotImplemented", "Ellipsis",
        // Common exceptions
        "Exception", "ValueError", "TypeError", "KeyError",
        "AttributeError", "ImportError", "IndexError",
    ]
    .iter()
    .map(|&s| s.to_string())
    .collect()
}

/// Helper: Extract expressions from statement
fn extract_expressions(stmt: &ast::Stmt) -> Vec<&ast::Expr> {
    match stmt {
        ast::Stmt::Expr(expr_stmt) => vec![&expr_stmt.value],
        ast::Stmt::Return(ret) => ret.value.as_ref().map(|e| vec![&**e]).unwrap_or_default(),
        ast::Stmt::If(if_stmt) => vec![&if_stmt.test],
        ast::Stmt::While(while_stmt) => vec![&while_stmt.test],
        // Add more as needed
        _ => Vec::new(),
    }
}
```

### Integration with DebtItem

```rust
// src/analysis/python_static_errors.rs

use crate::core::{DebtItem, DebtType, Priority};
use std::path::Path;

/// Convert StaticError to DebtItem
pub fn to_debt_item(error: &StaticError, file: &Path) -> DebtItem {
    match error {
        StaticError::UndefinedVariable { name, line, column, function } => {
            DebtItem {
                id: format!("undefined-{}-{}", name, line),
                debt_type: DebtType::StaticError,
                priority: Priority::High,
                file: file.to_path_buf(),
                line: *line,
                column: Some(*column),
                message: format!(
                    "Undefined variable '{}' in function '{}'",
                    name, function
                ),
                context: Some(format!("Should '{}' be 'self.{}'?", name, name)),
            }
        }
        StaticError::MissingImport { module, line, usage } => {
            DebtItem {
                id: format!("missing-import-{}-{}", module, line),
                debt_type: DebtType::StaticError,
                priority: Priority::High,
                file: file.to_path_buf(),
                line: *line,
                column: None,
                message: format!("Missing import: {}", module),
                context: Some(format!("Add 'import {}' (used as: {})", module, usage)),
            }
        }
    }
}

/// Convert all errors to debt items
pub fn errors_to_debt_items(result: &StaticAnalysisResult, file: &Path) -> Vec<DebtItem> {
    result.errors.iter().map(|e| to_debt_item(e, file)).collect()
}
```

### Update Core Types

```rust
// src/core/mod.rs - Add to DebtType enum

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum DebtType {
    Todo,
    Fixme,
    CodeSmell,
    StaticError,  // NEW
    // ... existing variants
}
```

## Implementation Phases

### Phase 0: Prototype [2 days]

**Goal**: Validate approach with minimal code

**Tasks**:
1. Create `src/analysis/python_static_errors.rs`
2. Implement only Issue #5 detection (undefined variables)
3. Write test case for Issue #5
4. Integrate with `PythonAnalyzer.analyze()`
5. Verify error appears in output

**Success**: Issue #5 detected in test

### Phase 1: Complete MVP [3 days]

**Goal**: Full static error detection

**Tasks**:
1. Add missing import detection (Issue #9)
2. Add loop/exception/context variable tracking
3. Implement builtin filtering
4. Add `DebtType::StaticError` to core
5. Convert errors to `DebtItem`

**Success**: All acceptance criteria met

### Phase 2: Polish [2 days]

**Goal**: Production ready

**Tasks**:
1. Performance benchmarking
2. False positive testing on real codebases
3. Error message refinement
4. Documentation
5. Integration tests

**Success**: < 10% overhead, < 5% false positives

**Total**: 7 days

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_parser::parse;

    #[test]
    fn test_issue_5_undefined_variable() {
        let code = r#"
def on_message_added(self, message, index):
    if message is messages[index].message:
        return True
"#;
        let ast = parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let resolver = EnhancedImportResolver::new();
        let result = analyze_static_errors(&ast, &resolver);

        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            result.errors[0],
            StaticError::UndefinedVariable { ref name, .. } if name == "messages"
        ));
    }

    #[test]
    fn test_issue_9_missing_import() {
        let code = r#"
def deliver(observers, message, index):
    wx.CallAfter(deliver, observers, message, index)
"#;
        let ast = parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let resolver = EnhancedImportResolver::new();
        let result = analyze_static_errors(&ast, &resolver);

        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            result.errors[0],
            StaticError::MissingImport { ref module, .. } if module == "wx"
        ));
    }

    #[test]
    fn test_self_not_flagged() {
        let code = r#"
class Example:
    def method(self):
        return self.value
"#;
        let ast = parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let resolver = EnhancedImportResolver::new();
        let result = analyze_static_errors(&ast, &resolver);

        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_builtins_not_flagged() {
        let code = r#"
def process():
    return len([1, 2, 3]) + sum([1, 2, 3])
"#;
        let ast = parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let resolver = EnhancedImportResolver::new();
        let result = analyze_static_errors(&ast, &resolver);

        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_loop_variable_defined() {
        let code = r#"
def process():
    for x in range(10):
        print(x)
"#;
        let ast = parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let resolver = EnhancedImportResolver::new();
        let result = analyze_static_errors(&ast, &resolver);

        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_exception_variable_defined() {
        let code = r#"
def handle():
    try:
        risky()
    except ValueError as e:
        print(e)
"#;
        let ast = parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let resolver = EnhancedImportResolver::new();
        let result = analyze_static_errors(&ast, &resolver);

        assert_eq!(result.errors.len(), 0);
    }
}
```

### Integration Test

```rust
#[test]
fn test_static_errors_in_file_metrics() {
    let code = r#"
def broken_function():
    return undefined_var
"#;
    let analyzer = PythonAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.py")).unwrap();
    let metrics = analyzer.analyze(&ast);

    assert!(metrics.debt_items.iter().any(|item|
        item.debt_type == DebtType::StaticError
        && item.message.contains("undefined_var")
    ));
}
```

## Success Metrics

- **Accuracy**: > 95% for undefined variables, > 90% for missing imports
- **False positives**: < 5%
- **Performance**: < 10% overhead
- **Code quality**: All functions ≤ 20 lines
- **Test coverage**: > 90%
- **User impact**: Detect real bugs in production codebases

## Known Limitations

**Will NOT detect** (deferred to Phase 2):
1. Cross-file undefined variables
2. `global`/`nonlocal` declarations
3. Comprehension scopes (Python 3 scoping)
4. Dynamic code (`exec`, `eval`, `__import__`)
5. Star imports (`from module import *`)
6. Class attribute references
7. Unreachable code

These are acceptable limitations for MVP.

## Future Enhancements

1. Cross-file symbol resolution (use `EnhancedImportResolver` fully)
2. Unreachable code detection
3. `global`/`nonlocal` support
4. Comprehension scopes
5. Type annotation awareness
6. Auto-fix suggestions
7. Integration with LSP for editor support

## Related Specifications

- Existing: `EnhancedImportResolver` in `src/analysis/python_imports.rs`
- Future: Spec 112 (Cross-File Dependency Analysis)
- Future: Spec 114b (Local Pattern Detection)

## Revision History

- 2025-10-16: Initial draft
- 2025-10-18: Complete redesign - simplified to MVP, removed visitor pattern, integrated with existing infrastructure, all functions < 20 lines
