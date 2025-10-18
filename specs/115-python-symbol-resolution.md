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

Debtmap v0.2.8 focuses on complexity and dead code detection but does not detect basic static errors like undefined variables and missing imports. This leads to misdiagnosis - flagging complex code that's actually broken.

**Real-World Impact from Bug Reports**:
- **Issue #5**: `ConversationPanel.on_message_added()` contains **undefined variable bug**
  ```python
  # Line 595 in conversation_panel.py
  if message is messages[index].message:  # ❌ 'messages' undefined, should be 'self.messages'
  ```
  - Debtmap flagged complexity but missed the actual bug
  - Code is broken, not just complex

- **Issue #9**: `DeliveryBoy.deliver()` references undefined `wx` module
  ```python
  wx.CallAfter(deliver, observers, message, index)  # ❌ 'wx' not imported in scope
  ```
  - Code won't run, not just unused
  - Should be flagged as broken

**Current Gaps**:
- No symbol table construction
- No undefined variable detection
- No missing import detection
- Cannot distinguish "broken code" from "dead code"
- Missing basic static error analysis

**Why This Matters**:
- Broken code is higher priority than complex code
- Users waste time on code that won't run
- Symbol resolution is a core static analyzer capability
- Debtmap should do this natively, not depend on external tools

## Objective

Build a symbol resolution system for Python to detect common static errors:
- Undefined variables (Issue #5)
- Missing imports (Issue #9)
- Unreachable code

**Approach**: Native implementation using debtmap's existing Python parser, with functional design and no external dependencies.

## Requirements

### Functional Requirements

1. **Symbol Table Construction**
   - Build symbol tables via visitor pattern on AST
   - Track variable definitions per scope (module, class, function, comprehension)
   - Include function parameters, local assignments, class attributes
   - Track imported names and aliases (`import X as Y`)
   - Include Python 3 builtins (comprehensive list)
   - Handle `for` loop variables, exception handlers, context managers
   - Support `global` and `nonlocal` declarations

2. **Undefined Variable Detection**
   - Identify references to undefined names
   - Check against symbol table with proper scope chain
   - Walk up scope chain (local → enclosing → global → builtin)
   - Report line number and context
   - Filter false positives: `self`, `cls`, decorator magic

3. **Missing Import Detection**
   - Track module references (e.g., `numpy.array()`)
   - Compare against imported modules
   - Handle import aliases (`import numpy as np`)
   - Detect undefined base names in attribute access
   - Report missing imports with suggestions

4. **Unreachable Code Detection** (Phase 2)
   - Detect code after unconditional return
   - Detect code after unconditional raise
   - Detect code in `if False:` blocks
   - Report dead branches

5. **Finding Enhancement**
   - Add static errors to debtmap findings
   - Classify code as "broken" vs "complex" vs "dead"
   - Prioritize broken code over complex code
   - Show errors in context of function analysis

6. **Error Classification**
   - Category: undefined-variable, missing-import, unreachable-code
   - Confidence scoring (high for obvious errors: 0.9-0.95)
   - Link errors to specific functions
   - Provide fix suggestions where possible

### Non-Functional Requirements

1. **Performance**
   - Static analysis adds < 10% to total analysis time
   - Single-pass AST traversal (not multiple walks)
   - Reuse existing AST parse tree (no re-parsing)
   - Efficient immutable data structures (`im` crate)
   - Minimal memory overhead (< 5MB per 1000 LOC)

2. **Accuracy**
   - 95%+ accuracy for undefined variables
   - 90%+ accuracy for missing imports
   - Low false positive rate (< 5%)
   - Handle common Python 3 idioms correctly

3. **Maintainability**
   - Pure functional analysis core
   - Immutable symbol tables (using `im` crate)
   - Testable components (all functions < 20 lines)
   - Clear error messages with context

4. **Compatibility**
   - Works with existing Python parser
   - No external dependencies
   - Python 3.6+ support (no Python 2)
   - Graceful handling of parse errors

## Acceptance Criteria

### Phase 1 (MVP)
- [ ] Symbol table construction with visitor pattern
- [ ] Undefined variable in Issue #5 detected and reported
- [ ] Missing import in Issue #9 detected and reported
- [ ] "Broken code" classification separate from "dead code"
- [ ] Findings enhanced with static error information
- [ ] Performance overhead < 10%
- [ ] False positive rate < 5%
- [ ] 95%+ test coverage for static analysis module
- [ ] Integration tests with real-world Python files
- [ ] Documentation with examples

### Phase 2 (Future)
- [ ] Unreachable code detection
- [ ] Conditional import handling
- [ ] Cross-file symbol resolution
- [ ] Type annotation awareness

## Current Python Parser

Debtmap currently uses [**TODO: Document actual parser**]:
- Parser library: tree-sitter-python / RustPython / Custom?
- AST structure: [TODO: Document Node types]
- Current capabilities: Parsing, complexity metrics
- Missing capabilities: Symbol tables, scope tracking

**Required additions**:
1. Visitor pattern for AST traversal with enter/exit callbacks
2. Scope context tracking during traversal
3. Import statement extraction
4. Name reference collection

## Technical Design

### Architecture

```
src/
├── analyzers/
│   └── python/
│       ├── parser.rs           # Existing
│       ├── ast.rs              # Existing - UPDATE with visitor pattern
│       ├── symbols.rs          # NEW: Immutable symbol table
│       ├── scope.rs            # NEW: Scope tracking
│       └── static_analysis.rs  # NEW: Static error detection
└── debt/
    └── dead_code.rs            # Updated: Use static errors
```

### Immutable Symbol Table

```rust
// src/analyzers/python/symbols.rs

use im::{HashMap, HashSet, Vector};

/// Immutable symbol table for tracking definitions
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SymbolTable {
    /// Module-level symbols (globals)
    globals: HashMap<String, SymbolDef>,
    /// All scopes (module, function, class, comprehension)
    scopes: Vector<Scope>,
    /// Imported modules and names
    imports: HashMap<String, ImportDef>,
    /// Python builtin names
    builtins: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    pub id: ScopeId,
    pub scope_type: ScopeType,
    pub name: String,
    pub symbols: HashMap<String, SymbolDef>,
    pub parent: Option<ScopeId>,
}

pub type ScopeId = usize;

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeType {
    Module,
    Function,
    Class,
    Comprehension,
    Lambda,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolDef {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub scope: ScopeId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Parameter,
    Function,
    Class,
    Import,
    LoopVariable,      // for x in items:
    ExceptionVariable, // except E as e:
    ContextVariable,   // with open(f) as fp:
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDef {
    pub module: String,
    pub alias: Option<String>,  // For "import numpy as np"
    pub names: Vec<String>,     // For "from X import a, b"
    pub is_star: bool,          // For "from X import *"
    pub line: usize,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            scopes: Vector::new(),
            imports: HashMap::new(),
            builtins: Self::python3_builtins(),
        }
    }

    /// Get comprehensive Python 3 builtin names
    fn python3_builtins() -> HashSet<String> {
        let builtins = vec![
            // Built-in functions
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

            // Built-in constants
            "True", "False", "None", "NotImplemented", "Ellipsis",
            "__debug__", "__name__", "__doc__", "__file__", "__package__",

            // Built-in exceptions
            "BaseException", "Exception", "ArithmeticError", "AssertionError",
            "AttributeError", "BlockingIOError", "BrokenPipeError",
            "BufferError", "ChildProcessError", "ConnectionAbortedError",
            "ConnectionError", "ConnectionRefusedError", "ConnectionResetError",
            "EOFError", "FileExistsError", "FileNotFoundError",
            "FloatingPointError", "GeneratorExit", "ImportError",
            "IndentationError", "IndexError", "InterruptedError",
            "IsADirectoryError", "KeyError", "KeyboardInterrupt",
            "LookupError", "MemoryError", "ModuleNotFoundError",
            "NameError", "NotADirectoryError", "NotImplementedError",
            "OSError", "OverflowError", "PermissionError", "ProcessLookupError",
            "RecursionError", "ReferenceError", "RuntimeError",
            "StopAsyncIteration", "StopIteration", "SyntaxError",
            "SystemError", "SystemExit", "TabError", "TimeoutError",
            "TypeError", "UnboundLocalError", "UnicodeDecodeError",
            "UnicodeEncodeError", "UnicodeError", "UnicodeTranslateError",
            "ValueError", "ZeroDivisionError",
        ];

        builtins.into_iter().map(String::from).collect()
    }

    /// Look up a symbol, walking up the scope chain (pure function)
    pub fn lookup(&self, name: &str, scope_id: ScopeId) -> Option<&SymbolDef> {
        // Check current scope
        if let Some(scope) = self.scopes.get(scope_id) {
            if let Some(sym) = scope.symbols.get(name) {
                return Some(sym);
            }

            // Walk up to parent scope
            if let Some(parent_id) = scope.parent {
                return self.lookup(name, parent_id);
            }
        }

        // Check globals
        if let Some(sym) = self.globals.get(name) {
            return Some(sym);
        }

        // Check builtins (return None if found - not an error)
        if self.builtins.contains(name) {
            return None;
        }

        None
    }

    /// Check if a module is imported (pure function)
    pub fn is_imported(&self, module: &str) -> bool {
        self.imports.contains_key(module) ||
        self.imports.values().any(|imp|
            imp.alias.as_ref().map_or(false, |alias| alias == module)
        )
    }

    /// Add a symbol definition (pure - returns new table)
    pub fn with_symbol(mut self, symbol: SymbolDef) -> Self {
        if symbol.scope == 0 {
            self.globals = self.globals.update(symbol.name.clone(), symbol);
        } else if let Some(scope) = self.scopes.get_mut(symbol.scope) {
            let mut updated_scope = scope.clone();
            updated_scope.symbols = updated_scope.symbols.update(symbol.name.clone(), symbol);
            self.scopes[symbol.scope] = updated_scope;
        }
        self
    }

    /// Add a scope (pure - returns new table with scope)
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scopes.push_back(scope);
        self
    }

    /// Add an import (pure - returns new table)
    pub fn with_import(mut self, import: ImportDef) -> Self {
        let key = import.alias.clone()
            .unwrap_or_else(|| import.module.clone());
        self.imports = self.imports.update(key, import);
        self
    }

    /// Get scope by ID
    pub fn get_scope(&self, scope_id: ScopeId) -> Option<&Scope> {
        self.scopes.get(scope_id)
    }

    /// Number of scopes
    pub fn scope_count(&self) -> usize {
        self.scopes.len()
    }
}
```

### Visitor Pattern for AST Traversal

```rust
// src/analyzers/python/ast.rs (additions to existing)

pub trait AstVisitor {
    fn enter_module(&mut self, module: &Module);
    fn exit_module(&mut self, module: &Module);

    fn enter_function(&mut self, func: &FunctionDef);
    fn exit_function(&mut self, func: &FunctionDef);

    fn enter_class(&mut self, class: &ClassDef);
    fn exit_class(&mut self, class: &ClassDef);

    fn visit_import(&mut self, import: &Import);
    fn visit_import_from(&mut self, import: &ImportFrom);

    fn visit_assign(&mut self, assign: &Assign);
    fn visit_name(&mut self, name: &Name, context: NameContext);
    fn visit_attribute(&mut self, attr: &Attribute);

    fn enter_for(&mut self, for_stmt: &For);
    fn exit_for(&mut self, for_stmt: &For);

    fn enter_with(&mut self, with: &With);
    fn exit_with(&mut self, with: &With);

    fn enter_except_handler(&mut self, handler: &ExceptHandler);
    fn exit_except_handler(&mut self, handler: &ExceptHandler);

    fn enter_comprehension(&mut self, comp: &Comprehension);
    fn exit_comprehension(&mut self, comp: &Comprehension);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NameContext {
    Load,   // Reading a variable
    Store,  // Assigning to a variable
    Del,    // Deleting a variable
}

impl PythonAst {
    /// Walk AST with visitor pattern
    pub fn accept<V: AstVisitor>(&self, visitor: &mut V) {
        visitor.enter_module(&self.module);

        for stmt in &self.module.body {
            self.visit_stmt(stmt, visitor);
        }

        visitor.exit_module(&self.module);
    }

    fn visit_stmt<V: AstVisitor>(&self, stmt: &Stmt, visitor: &mut V) {
        match stmt {
            Stmt::FunctionDef(func) => {
                visitor.enter_function(func);
                for stmt in &func.body {
                    self.visit_stmt(stmt, visitor);
                }
                visitor.exit_function(func);
            }
            Stmt::ClassDef(class) => {
                visitor.enter_class(class);
                for stmt in &class.body {
                    self.visit_stmt(stmt, visitor);
                }
                visitor.exit_class(class);
            }
            Stmt::Import(import) => visitor.visit_import(import),
            Stmt::ImportFrom(import) => visitor.visit_import_from(import),
            Stmt::Assign(assign) => visitor.visit_assign(assign),
            Stmt::For(for_stmt) => {
                visitor.enter_for(for_stmt);
                for stmt in &for_stmt.body {
                    self.visit_stmt(stmt, visitor);
                }
                visitor.exit_for(for_stmt);
            }
            Stmt::With(with) => {
                visitor.enter_with(with);
                for stmt in &with.body {
                    self.visit_stmt(stmt, visitor);
                }
                visitor.exit_with(with);
            }
            // ... other statement types
            _ => {}
        }
    }
}
```

### Symbol Table Builder

```rust
// src/analyzers/python/scope.rs

use super::symbols::*;
use super::ast::*;

/// Builds symbol table via visitor pattern (pure - collects state)
pub struct SymbolTableBuilder {
    table: SymbolTable,
    scope_stack: Vec<ScopeId>,
    next_scope_id: ScopeId,
}

impl SymbolTableBuilder {
    pub fn new() -> Self {
        Self {
            table: SymbolTable::new(),
            scope_stack: vec![0], // Start at module scope (ID 0)
            next_scope_id: 1,
        }
    }

    /// Convert builder into final symbol table
    pub fn into_table(self) -> SymbolTable {
        self.table
    }

    /// Get current scope ID
    fn current_scope(&self) -> ScopeId {
        *self.scope_stack.last().unwrap_or(&0)
    }

    /// Enter a new scope
    fn enter_scope(&mut self, scope_type: ScopeType, name: String) -> ScopeId {
        let scope_id = self.next_scope_id;
        self.next_scope_id += 1;

        let scope = Scope {
            id: scope_id,
            scope_type,
            name,
            symbols: HashMap::new(),
            parent: Some(self.current_scope()),
        };

        self.table = self.table.with_scope(scope);
        self.scope_stack.push(scope_id);
        scope_id
    }

    /// Exit current scope
    fn exit_scope(&mut self) {
        self.scope_stack.pop();
    }

    /// Add symbol to current scope
    fn add_symbol(&mut self, name: String, kind: SymbolKind, line: usize) {
        let symbol = SymbolDef {
            name,
            kind,
            line,
            scope: self.current_scope(),
        };
        self.table = self.table.with_symbol(symbol);
    }
}

impl AstVisitor for SymbolTableBuilder {
    fn enter_module(&mut self, _module: &Module) {
        // Module scope is already initialized (ID 0)
    }

    fn exit_module(&mut self, _module: &Module) {
        // Nothing to do
    }

    fn enter_function(&mut self, func: &FunctionDef) {
        // Add function to current scope
        self.add_symbol(func.name.clone(), SymbolKind::Function, func.line);

        // Enter function scope
        self.enter_scope(ScopeType::Function, func.name.clone());

        // Add parameters
        for param in &func.params {
            self.add_symbol(param.name.clone(), SymbolKind::Parameter, func.line);
        }
    }

    fn exit_function(&mut self, _func: &FunctionDef) {
        self.exit_scope();
    }

    fn enter_class(&mut self, class: &ClassDef) {
        self.add_symbol(class.name.clone(), SymbolKind::Class, class.line);
        self.enter_scope(ScopeType::Class, class.name.clone());
    }

    fn exit_class(&mut self, _class: &ClassDef) {
        self.exit_scope();
    }

    fn visit_import(&mut self, import: &Import) {
        for alias in &import.names {
            let import_def = ImportDef {
                module: alias.name.clone(),
                alias: alias.asname.clone(),
                names: vec![],
                is_star: false,
                line: import.line,
            };
            self.table = self.table.with_import(import_def);

            // Also add to symbol table
            let name = alias.asname.clone().unwrap_or_else(|| alias.name.clone());
            self.add_symbol(name, SymbolKind::Import, import.line);
        }
    }

    fn visit_import_from(&mut self, import: &ImportFrom) {
        if let Some(module) = &import.module {
            let is_star = import.names.iter().any(|n| n.name == "*");

            let import_def = ImportDef {
                module: module.clone(),
                alias: None,
                names: import.names.iter().map(|n| n.name.clone()).collect(),
                is_star,
                line: import.line,
            };
            self.table = self.table.with_import(import_def);

            // Add imported names to symbol table (unless star import)
            if !is_star {
                for alias in &import.names {
                    let name = alias.asname.clone().unwrap_or_else(|| alias.name.clone());
                    self.add_symbol(name, SymbolKind::Import, import.line);
                }
            }
        }
    }

    fn visit_assign(&mut self, assign: &Assign) {
        for target in &assign.targets {
            if let Some(name) = extract_simple_name(target) {
                self.add_symbol(name, SymbolKind::Variable, assign.line);
            }
        }
    }

    fn visit_name(&mut self, _name: &Name, _context: NameContext) {
        // Name references are tracked separately during error detection
    }

    fn visit_attribute(&mut self, _attr: &Attribute) {
        // Attribute access tracked during error detection
    }

    fn enter_for(&mut self, for_stmt: &For) {
        // for x in items: - 'x' is defined
        if let Some(name) = extract_simple_name(&for_stmt.target) {
            self.add_symbol(name, SymbolKind::LoopVariable, for_stmt.line);
        }
    }

    fn exit_for(&mut self, _for_stmt: &For) {
        // For loop doesn't create a new scope in Python
    }

    fn enter_with(&mut self, with: &With) {
        // with open(f) as fp: - 'fp' is defined
        for item in &with.items {
            if let Some(var) = &item.optional_vars {
                if let Some(name) = extract_simple_name(var) {
                    self.add_symbol(name, SymbolKind::ContextVariable, with.line);
                }
            }
        }
    }

    fn exit_with(&mut self, _with: &With) {
        // With doesn't create scope
    }

    fn enter_except_handler(&mut self, handler: &ExceptHandler) {
        // except ValueError as e: - 'e' is defined
        if let Some(name) = &handler.name {
            self.add_symbol(name.clone(), SymbolKind::ExceptionVariable, handler.line);
        }
    }

    fn exit_except_handler(&mut self, _handler: &ExceptHandler) {
        // Exception handler doesn't create scope
    }

    fn enter_comprehension(&mut self, _comp: &Comprehension) {
        // Comprehensions have their own scope in Python 3
        self.enter_scope(ScopeType::Comprehension, "<comprehension>".to_string());
    }

    fn exit_comprehension(&mut self, _comp: &Comprehension) {
        self.exit_scope();
    }
}

/// Extract simple name from expression (helper)
fn extract_simple_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Name(name) => Some(name.id.clone()),
        _ => None, // Ignore complex assignments like a.b = x
    }
}
```

### Static Analysis Core (Single Pass)

```rust
// src/analyzers/python/static_analysis.rs

use super::symbols::*;
use super::scope::*;
use super::ast::*;

#[derive(Debug, Clone, PartialEq)]
pub struct StaticAnalysisResult {
    pub undefined_vars: Vec<UndefinedVariable>,
    pub missing_imports: Vec<MissingImport>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UndefinedVariable {
    pub name: String,
    pub line: usize,
    pub column: Option<usize>,
    pub context: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MissingImport {
    pub module: String,
    pub line: usize,
    pub usage: String,
    pub suggestion: Option<String>,
}

/// Analyze Python AST for static errors (pure function)
pub fn analyze(ast: &PythonAst) -> StaticAnalysisResult {
    // Build symbol table
    let symbol_table = build_symbol_table(ast);

    // Find errors
    find_errors(ast, &symbol_table)
}

/// Build symbol table from AST (pure function)
fn build_symbol_table(ast: &PythonAst) -> SymbolTable {
    let mut builder = SymbolTableBuilder::new();
    ast.accept(&mut builder);
    builder.into_table()
}

/// Find all errors in single pass (pure function)
fn find_errors(ast: &PythonAst, symbols: &SymbolTable) -> StaticAnalysisResult {
    let mut finder = ErrorFinder::new(symbols.clone());
    ast.accept(&mut finder);
    finder.into_result()
}

/// Error finder visitor
struct ErrorFinder {
    symbols: SymbolTable,
    scope_stack: Vec<ScopeId>,
    undefined_vars: Vec<UndefinedVariable>,
    missing_imports: Vec<MissingImport>,
    context_stack: Vec<String>,
}

impl ErrorFinder {
    fn new(symbols: SymbolTable) -> Self {
        Self {
            symbols,
            scope_stack: vec![0],
            undefined_vars: Vec::new(),
            missing_imports: Vec::new(),
            context_stack: vec!["module".to_string()],
        }
    }

    fn into_result(self) -> StaticAnalysisResult {
        StaticAnalysisResult {
            undefined_vars: self.undefined_vars,
            missing_imports: self.missing_imports,
        }
    }

    fn current_scope(&self) -> ScopeId {
        *self.scope_stack.last().unwrap_or(&0)
    }

    fn current_context(&self) -> String {
        self.context_stack.last()
            .cloned()
            .unwrap_or_else(|| "module".to_string())
    }

    fn should_skip_name(&self, name: &str) -> bool {
        // Skip common false positives
        matches!(name, "self" | "cls" | "__class__" | "__name__" | "__file__")
    }
}

impl AstVisitor for ErrorFinder {
    fn enter_module(&mut self, _module: &Module) {
        // Already at module scope
    }

    fn exit_module(&mut self, _module: &Module) {}

    fn enter_function(&mut self, func: &FunctionDef) {
        self.context_stack.push(format!("function '{}'", func.name));
        // Find the scope ID for this function
        if let Some(scope_id) = find_function_scope(&self.symbols, &func.name, self.current_scope()) {
            self.scope_stack.push(scope_id);
        }
    }

    fn exit_function(&mut self, _func: &FunctionDef) {
        self.scope_stack.pop();
        self.context_stack.pop();
    }

    fn enter_class(&mut self, class: &ClassDef) {
        self.context_stack.push(format!("class '{}'", class.name));
        if let Some(scope_id) = find_class_scope(&self.symbols, &class.name, self.current_scope()) {
            self.scope_stack.push(scope_id);
        }
    }

    fn exit_class(&mut self, _class: &ClassDef) {
        self.scope_stack.pop();
        self.context_stack.pop();
    }

    fn visit_import(&mut self, _import: &Import) {
        // Already handled by symbol table builder
    }

    fn visit_import_from(&mut self, _import: &ImportFrom) {
        // Already handled by symbol table builder
    }

    fn visit_assign(&mut self, _assign: &Assign) {
        // Already handled by symbol table builder
    }

    fn visit_name(&mut self, name: &Name, context: NameContext) {
        // Only check Load context (reading variables)
        if context != NameContext::Load {
            return;
        }

        // Skip false positives
        if self.should_skip_name(&name.id) {
            return;
        }

        // Check if defined
        if self.symbols.lookup(&name.id, self.current_scope()).is_none()
            && !self.symbols.builtins.contains(&name.id) {
            self.undefined_vars.push(UndefinedVariable {
                name: name.id.clone(),
                line: name.line,
                column: Some(name.col),
                context: self.current_context(),
            });
        }
    }

    fn visit_attribute(&mut self, attr: &Attribute) {
        // Check for module.attribute pattern (e.g., wx.CallAfter)
        if let Expr::Name(name) = &*attr.value {
            // Check if base name is undefined and not imported
            if !self.should_skip_name(&name.id)
                && self.symbols.lookup(&name.id, self.current_scope()).is_none()
                && !self.symbols.builtins.contains(&name.id)
                && !self.symbols.is_imported(&name.id) {
                // Likely a missing import
                self.missing_imports.push(MissingImport {
                    module: name.id.clone(),
                    line: attr.line,
                    usage: format!("{}.{}", name.id, attr.attr),
                    suggestion: Some(format!("import {}", name.id)),
                });
            }
        }
    }

    fn enter_for(&mut self, _for_stmt: &For) {}
    fn exit_for(&mut self, _for_stmt: &For) {}
    fn enter_with(&mut self, _with: &With) {}
    fn exit_with(&mut self, _with: &With) {}
    fn enter_except_handler(&mut self, _handler: &ExceptHandler) {}
    fn exit_except_handler(&mut self, _handler: &ExceptHandler) {}
    fn enter_comprehension(&mut self, _comp: &Comprehension) {
        if let Some(scope_id) = find_comprehension_scope(&self.symbols, self.current_scope()) {
            self.scope_stack.push(scope_id);
        }
    }
    fn exit_comprehension(&mut self, _comp: &Comprehension) {
        self.scope_stack.pop();
    }
}

// Helper functions (all pure, < 20 lines)

fn find_function_scope(symbols: &SymbolTable, name: &str, parent: ScopeId) -> Option<ScopeId> {
    (0..symbols.scope_count())
        .find(|&id| {
            if let Some(scope) = symbols.get_scope(id) {
                scope.scope_type == ScopeType::Function
                    && scope.name == name
                    && scope.parent == Some(parent)
            } else {
                false
            }
        })
}

fn find_class_scope(symbols: &SymbolTable, name: &str, parent: ScopeId) -> Option<ScopeId> {
    (0..symbols.scope_count())
        .find(|&id| {
            if let Some(scope) = symbols.get_scope(id) {
                scope.scope_type == ScopeType::Class
                    && scope.name == name
                    && scope.parent == Some(parent)
            } else {
                false
            }
        })
}

fn find_comprehension_scope(symbols: &SymbolTable, parent: ScopeId) -> Option<ScopeId> {
    (0..symbols.scope_count())
        .rev()
        .find(|&id| {
            if let Some(scope) = symbols.get_scope(id) {
                scope.scope_type == ScopeType::Comprehension
                    && scope.parent == Some(parent)
            } else {
                false
            }
        })
}
```

### Integration with Dead Code Detection

```rust
// src/debt/dead_code.rs (updated)

use crate::analyzers::python::static_analysis::*;

#[derive(Debug, Clone)]
pub struct DeadCodeFinding {
    pub function: FunctionDef,
    pub confidence: f32,
    pub reason: String,
    pub static_errors: Vec<StaticError>,
    pub is_broken: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StaticError {
    pub kind: StaticErrorKind,
    pub line: usize,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StaticErrorKind {
    UndefinedVariable,
    MissingImport,
}

impl DeadCodeDetector {
    pub fn detect_with_static_analysis(
        &self,
        function: &FunctionDef,
        static_result: &StaticAnalysisResult,
    ) -> Option<DeadCodeFinding> {
        let errors = extract_function_errors(function, static_result);
        let is_broken = has_critical_errors(&errors);
        let has_callers = self.has_callers(function);

        match (has_callers, is_broken) {
            (false, true) => Some(create_broken_dead_finding(function, errors)),
            (false, false) => Some(create_dead_finding(function, errors)),
            (true, true) => Some(create_broken_live_finding(function, errors)),
            (true, false) => None, // Live, working code
        }
    }
}

// Pure helper functions (all < 20 lines)

fn extract_function_errors(
    function: &FunctionDef,
    static_result: &StaticAnalysisResult,
) -> Vec<StaticError> {
    let mut errors = Vec::new();

    for undef in &static_result.undefined_vars {
        if function.line_range.contains(&undef.line) {
            errors.push(StaticError {
                kind: StaticErrorKind::UndefinedVariable,
                line: undef.line,
                message: format!("Undefined variable '{}'", undef.name),
                suggestion: suggest_fix_for_undefined(&undef.name),
            });
        }
    }

    for missing in &static_result.missing_imports {
        if function.line_range.contains(&missing.line) {
            errors.push(StaticError {
                kind: StaticErrorKind::MissingImport,
                line: missing.line,
                message: format!("Missing import: {}", missing.module),
                suggestion: missing.suggestion.clone(),
            });
        }
    }

    errors
}

fn has_critical_errors(errors: &[StaticError]) -> bool {
    errors.iter().any(|e| matches!(
        e.kind,
        StaticErrorKind::UndefinedVariable | StaticErrorKind::MissingImport
    ))
}

fn create_broken_dead_finding(
    function: &FunctionDef,
    errors: Vec<StaticError>,
) -> DeadCodeFinding {
    DeadCodeFinding {
        function: function.clone(),
        confidence: 0.95,
        reason: format_broken_reason(&errors),
        static_errors: errors,
        is_broken: true,
    }
}

fn create_dead_finding(
    function: &FunctionDef,
    errors: Vec<StaticError>,
) -> DeadCodeFinding {
    DeadCodeFinding {
        function: function.clone(),
        confidence: 0.75,
        reason: "Function has no callers".to_string(),
        static_errors: errors,
        is_broken: false,
    }
}

fn create_broken_live_finding(
    function: &FunctionDef,
    errors: Vec<StaticError>,
) -> DeadCodeFinding {
    DeadCodeFinding {
        function: function.clone(),
        confidence: 0.9,
        reason: "Function is called but contains errors".to_string(),
        static_errors: errors,
        is_broken: true,
    }
}

fn format_broken_reason(errors: &[StaticError]) -> String {
    let error_summary: Vec<String> = errors.iter()
        .take(3)
        .map(|e| format!("line {}: {}", e.line, e.message))
        .collect();

    format!("Code is BROKEN: {}", error_summary.join("; "))
}

fn suggest_fix_for_undefined(name: &str) -> Option<String> {
    // Common patterns
    if name.ends_with('s') {
        Some(format!("Did you mean 'self.{}'?", name))
    } else {
        None
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_undefined_variable() {
        let code = r#"
def process(item):
    return items[0]  # 'items' undefined
"#;
        let ast = parse_python(code).unwrap();
        let result = analyze(&ast);

        assert_eq!(result.undefined_vars.len(), 1);
        assert_eq!(result.undefined_vars[0].name, "items");
    }

    #[test]
    fn test_for_loop_variable_defined() {
        let code = r#"
for x in range(10):
    print(x)  # ✅ x is defined
print(x)      # ✅ x is still defined (Python quirk)
"#;
        let ast = parse_python(code).unwrap();
        let result = analyze(&ast);

        assert_eq!(result.undefined_vars.len(), 0);
    }

    #[test]
    fn test_exception_variable_defined() {
        let code = r#"
try:
    risky()
except ValueError as e:
    print(e)  # ✅ e is defined
"#;
        let ast = parse_python(code).unwrap();
        let result = analyze(&ast);

        assert_eq!(result.undefined_vars.len(), 0);
    }

    #[test]
    fn test_context_manager_variable() {
        let code = r#"
with open('file.txt') as fp:
    print(fp)  # ✅ fp is defined
"#;
        let ast = parse_python(code).unwrap();
        let result = analyze(&ast);

        assert_eq!(result.undefined_vars.len(), 0);
    }

    #[test]
    fn test_comprehension_scope_python3() {
        let code = r#"
[x for x in range(10)]
print(x)  # ❌ x is NOT defined in Python 3
"#;
        let ast = parse_python(code).unwrap();
        let result = analyze(&ast);

        assert_eq!(result.undefined_vars.len(), 1);
        assert_eq!(result.undefined_vars[0].name, "x");
    }

    #[test]
    fn test_nested_scope_resolution() {
        let code = r#"
x = 1  # global

def outer():
    y = 2  # outer scope

    def inner():
        z = 3  # inner scope
        return x + y + z  # All should resolve

    return inner()
"#;
        let ast = parse_python(code).unwrap();
        let result = analyze(&ast);

        assert_eq!(result.undefined_vars.len(), 0);
    }

    #[test]
    fn test_builtins_not_flagged() {
        let code = r#"
def process():
    data = [1, 2, 3]
    result = len(data) + sum(data)  # len, sum are builtins
    return isinstance(result, int)  # isinstance is builtin
"#;
        let ast = parse_python(code).unwrap();
        let result = analyze(&ast);

        assert_eq!(result.undefined_vars.len(), 0);
    }

    #[test]
    fn test_self_not_flagged() {
        let code = r#"
class Example:
    def method(self):
        return self.value  # self is implicit
"#;
        let ast = parse_python(code).unwrap();
        let result = analyze(&ast);

        assert_eq!(result.undefined_vars.len(), 0);
    }

    #[test]
    fn test_import_alias() {
        let code = r#"
import numpy as np

def process():
    return np.array([1, 2, 3])  # ✅ np is imported
"#;
        let ast = parse_python(code).unwrap();
        let result = analyze(&ast);

        assert_eq!(result.missing_imports.len(), 0);
    }

    #[test]
    fn test_missing_import() {
        let code = r#"
def use_numpy():
    return numpy.array([1, 2, 3])  # ❌ numpy not imported
"#;
        let ast = parse_python(code).unwrap();
        let result = analyze(&ast);

        assert_eq!(result.missing_imports.len(), 1);
        assert_eq!(result.missing_imports[0].module, "numpy");
    }

    #[test]
    fn test_from_import() {
        let code = r#"
from collections import defaultdict

def process():
    return defaultdict(list)  # ✅ defaultdict imported
"#;
        let ast = parse_python(code).unwrap();
        let result = analyze(&ast);

        assert_eq!(result.undefined_vars.len(), 0);
    }

    #[test]
    fn test_issue_5_detection() {
        let code = r#"
class ConversationPanel:
    def __init__(self):
        self.messages = []

    def on_message_added(self, message, index):
        if message is messages[index].message:  # ❌ Should be self.messages
            return True
        return False
"#;
        let ast = parse_python(code).unwrap();
        let result = analyze(&ast);

        assert_eq!(result.undefined_vars.len(), 1);
        assert_eq!(result.undefined_vars[0].name, "messages");
        assert!(result.undefined_vars[0].context.contains("on_message_added"));
    }

    #[test]
    fn test_issue_9_detection() {
        let code = r#"
def deliver(observers, message, index):
    wx.CallAfter(deliver, observers, message, index)  # ❌ wx not imported
"#;
        let ast = parse_python(code).unwrap();
        let result = analyze(&ast);

        assert_eq!(result.missing_imports.len(), 1);
        assert_eq!(result.missing_imports[0].module, "wx");
        assert_eq!(result.missing_imports[0].suggestion, Some("import wx".to_string()));
    }
}
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn symbol_table_never_panics(code in any::<String>()) {
        if let Ok(ast) = parse_python(&code) {
            let _ = build_symbol_table(&ast);
        }
    }

    #[test]
    fn analysis_deterministic(code in valid_python_code()) {
        let ast = parse_python(&code).unwrap();
        let result1 = analyze(&ast);
        let result2 = analyze(&ast);
        prop_assert_eq!(result1, result2);
    }

    #[test]
    fn symbol_table_immutable(code in valid_python_code()) {
        let ast = parse_python(&code).unwrap();
        let table1 = build_symbol_table(&ast);
        let table2 = table1.clone();
        prop_assert_eq!(table1, table2);
    }
}
```

## Performance Requirements

| Metric | Target | Measurement |
|--------|--------|-------------|
| Overhead | < 10% | Single-pass visitor pattern |
| Memory | < 5MB per 1000 LOC | Immutable structures with structural sharing |
| Accuracy | > 95% | False positive rate < 5% |
| Functions | < 20 lines | All analysis functions pure and small |

## Implementation Phases

### Phase 1: AST Visitor Pattern [2-3 days]
**Goal**: Add visitor pattern to existing Python AST

**Tasks**:
- Define `AstVisitor` trait
- Implement `accept()` on `PythonAst`
- Add enter/exit callbacks for all node types
- Write tests for visitor traversal

**Success Criteria**:
- Visitor walks entire AST
- Enter/exit callbacks fire correctly
- Scope tracking works (push/pop)
- All tests pass

### Phase 2: Immutable Symbol Table [2-3 days]
**Goal**: Build symbol table with pure functions

**Tasks**:
- Implement `SymbolTable` with `im` crate
- Implement `SymbolTableBuilder` visitor
- Add comprehensive Python 3 builtins
- Handle all variable definition contexts (for, except, with)
- Write comprehensive unit tests

**Success Criteria**:
- Symbol table is immutable
- All Python 3 constructs supported
- Scope resolution works correctly
- Tests pass for nested scopes
- Issue #5 code creates correct symbol table

### Phase 3: Error Detection [3-4 days]
**Goal**: Detect undefined variables and missing imports

**Tasks**:
- Implement `ErrorFinder` visitor
- Detect undefined variable references
- Detect missing imports via attribute access
- Add false positive filtering
- Test with Issue #5 and Issue #9

**Success Criteria**:
- Issue #5 bug detected
- Issue #9 bug detected
- False positive rate < 5%
- All unit tests pass
- Property tests pass

### Phase 4: Integration [2-3 days]
**Goal**: Integrate with debtmap findings

**Tasks**:
- Update `DeadCodeFinding` structure
- Classify broken vs dead code
- Update output formatting
- Add fix suggestions
- Performance testing

**Success Criteria**:
- Broken code classified correctly
- Output includes static errors
- Performance overhead < 10%
- All acceptance criteria met
- Documentation complete

**Total Estimated Time**: 9-13 days

## Known Limitations

This implementation will NOT detect:

1. **Dynamic code**: `exec()`, `eval()`, `__import__()`
2. **Metaclass magic**: Dynamic attribute injection
3. **Monkey patching**: Runtime attribute additions
4. **Star imports**: `from module import *` (imports unknown names)
5. **C extensions**: Native module attribute access
6. **Conditional imports**: Platform-specific imports
7. **Type checking**: Only syntax, not semantics
8. **global/nonlocal**: Not implemented in Phase 1

These are inherent limitations of static analysis or deferred to future phases.

## Dependencies

### External Crates

Add to `Cargo.toml`:

```toml
[dependencies]
im = "15.1"  # Immutable data structures
```

### Current Parser Status

**TODO**: Document debtmap's current Python parser:
- Parser library: [tree-sitter-python / RustPython / Custom?]
- AST node types: [List node types]
- Current capabilities: [What exists]
- Required changes: [What needs to be added]

## Success Metrics

- **Error Detection**: Catch 95%+ of undefined variables and missing imports
- **Accuracy**: False positive rate < 5%
- **Performance**: < 10% overhead
- **Code Quality**: All functions < 20 lines, pure where possible
- **Test Coverage**: 95%+ coverage
- **User Impact**: Detect real bugs in production codebases

## Future Enhancements (Phase 2+)

1. **Unreachable Code Detection**: After return/raise, `if False:` blocks
2. **global/nonlocal Support**: Proper handling of scope modifiers
3. **Type Inference**: Simple type tracking for better error detection
4. **Cross-File Resolution**: Import tracking across modules
5. **Fix Suggestions**: Auto-fix for common errors
6. **Custom Rules**: User-defined error patterns
7. **Other Languages**: Extend to JavaScript, TypeScript

## Related Specifications

- Spec 112: Cross-File Dependency Analysis (symbol resolution)
- Spec 114b: Local Pattern Detection (uses static analysis)
- Spec 116: Confidence Scoring System (enhanced by error detection)

## Revision History

- 2025-10-16: Initial draft (external tool integration approach)
- 2025-10-18: Major revision to native implementation approach
- 2025-10-18: Complete redesign with functional architecture, visitor pattern, immutable data structures
