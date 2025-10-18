//! Python Static Error Detection
//!
//! Detects static errors in Python code including:
//! - Undefined variables
//! - Missing imports
//!
//! This module provides pure functional analysis of Python AST to identify
//! common programming errors that would fail at runtime.

use crate::analysis::python_imports::EnhancedImportResolver;
use crate::core::types::{DebtCategory, DebtItem, Severity, SourceLocation};
use rustpython_parser::ast;
use std::collections::HashSet;
use std::path::Path;

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
    let symbols = extract_local_symbols(func);
    find_undefined_names(&func.body, func.name.as_str(), &symbols, resolver, builtins)
}

/// Extract all locally defined symbols from function
fn extract_local_symbols(func: &ast::StmtFunctionDef) -> LocalSymbols {
    let mut symbols = LocalSymbols::new();

    // Add parameters - always add "self" for methods since it's implicit
    symbols.insert("self".to_string());
    symbols.insert("cls".to_string());

    // Extract parameter names from function args
    for arg in &func.args.args {
        symbols.insert(arg.def.arg.to_string());
    }

    if let Some(vararg) = &func.args.vararg {
        symbols.insert(vararg.arg.to_string());
    }

    for kwarg in &func.args.kwonlyargs {
        symbols.insert(kwarg.def.arg.to_string());
    }

    if let Some(kwarg) = &func.args.kwarg {
        symbols.insert(kwarg.arg.to_string());
    }

    // Add local assignments, loop vars, etc.
    collect_definitions(&func.body, &mut symbols);

    symbols
}

/// Collect variable definitions from statements
fn collect_definitions(stmts: &[ast::Stmt], symbols: &mut LocalSymbols) {
    for stmt in stmts {
        match stmt {
            ast::Stmt::Assign(assign) => collect_from_assign(assign, symbols),
            ast::Stmt::For(for_stmt) => collect_from_for(for_stmt, symbols),
            ast::Stmt::With(with) => collect_from_with(with, symbols),
            ast::Stmt::Try(try_stmt) => collect_from_try(try_stmt, symbols),
            ast::Stmt::If(if_stmt) => collect_from_if(if_stmt, symbols),
            ast::Stmt::While(while_stmt) => collect_from_while(while_stmt, symbols),
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
    collect_definitions(&for_stmt.orelse, symbols);
}

/// Extract context manager variable
fn collect_from_with(with: &ast::StmtWith, symbols: &mut LocalSymbols) {
    for item in &with.items {
        if let Some(vars) = &item.optional_vars {
            if let ast::Expr::Name(name) = vars.as_ref() {
                symbols.insert(name.id.to_string());
            }
        }
    }
    collect_definitions(&with.body, symbols);
}

/// Extract exception variables
fn collect_from_try(try_stmt: &ast::StmtTry, symbols: &mut LocalSymbols) {
    collect_definitions(&try_stmt.body, symbols);
    for handler in &try_stmt.handlers {
        let ast::ExceptHandler::ExceptHandler(h) = handler;
        if let Some(name) = &h.name {
            symbols.insert(name.to_string());
        }
        collect_definitions(&h.body, symbols);
    }
    collect_definitions(&try_stmt.orelse, symbols);
    collect_definitions(&try_stmt.finalbody, symbols);
}

/// Extract variables from if statement branches
fn collect_from_if(if_stmt: &ast::StmtIf, symbols: &mut LocalSymbols) {
    collect_definitions(&if_stmt.body, symbols);
    collect_definitions(&if_stmt.orelse, symbols);
}

/// Extract variables from while statement
fn collect_from_while(while_stmt: &ast::StmtWhile, symbols: &mut LocalSymbols) {
    collect_definitions(&while_stmt.body, symbols);
    collect_definitions(&while_stmt.orelse, symbols);
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
        errors.extend(check_stmt_for_undefined(
            stmt, func_name, symbols, resolver, builtins,
        ));
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
        errors.extend(check_expr_for_undefined(
            expr, func_name, symbols, resolver, builtins,
        ));
    }

    // Recursively check nested blocks
    match stmt {
        ast::Stmt::If(if_stmt) => {
            errors.extend(find_undefined_names(
                &if_stmt.body,
                func_name,
                symbols,
                resolver,
                builtins,
            ));
            errors.extend(find_undefined_names(
                &if_stmt.orelse,
                func_name,
                symbols,
                resolver,
                builtins,
            ));
        }
        ast::Stmt::While(while_stmt) => {
            errors.extend(find_undefined_names(
                &while_stmt.body,
                func_name,
                symbols,
                resolver,
                builtins,
            ));
        }
        ast::Stmt::For(for_stmt) => {
            errors.extend(find_undefined_names(
                &for_stmt.body,
                func_name,
                symbols,
                resolver,
                builtins,
            ));
        }
        ast::Stmt::Try(try_stmt) => {
            errors.extend(find_undefined_names(
                &try_stmt.body,
                func_name,
                symbols,
                resolver,
                builtins,
            ));
            for handler in &try_stmt.handlers {
                let ast::ExceptHandler::ExceptHandler(h) = handler;
                errors.extend(find_undefined_names(
                    &h.body, func_name, symbols, resolver, builtins,
                ));
            }
        }
        ast::Stmt::With(with_stmt) => {
            errors.extend(find_undefined_names(
                &with_stmt.body,
                func_name,
                symbols,
                resolver,
                builtins,
            ));
        }
        _ => {}
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
    let mut errors = Vec::new();

    match expr {
        ast::Expr::Name(name) if matches!(name.ctx, ast::ExprContext::Load) => {
            errors.extend(check_name_reference(name, func_name, symbols, builtins));
        }
        ast::Expr::Attribute(attr) => {
            errors.extend(check_attribute_access(attr, resolver));
            // Recursively check the base expression
            errors.extend(check_expr_for_undefined(
                &attr.value,
                func_name,
                symbols,
                resolver,
                builtins,
            ));
        }
        ast::Expr::Call(call) => {
            errors.extend(check_expr_for_undefined(
                &call.func, func_name, symbols, resolver, builtins,
            ));
            for arg in &call.args {
                errors.extend(check_expr_for_undefined(
                    arg, func_name, symbols, resolver, builtins,
                ));
            }
        }
        ast::Expr::BinOp(binop) => {
            errors.extend(check_expr_for_undefined(
                &binop.left,
                func_name,
                symbols,
                resolver,
                builtins,
            ));
            errors.extend(check_expr_for_undefined(
                &binop.right,
                func_name,
                symbols,
                resolver,
                builtins,
            ));
        }
        ast::Expr::Compare(compare) => {
            errors.extend(check_expr_for_undefined(
                &compare.left,
                func_name,
                symbols,
                resolver,
                builtins,
            ));
            for comparator in &compare.comparators {
                errors.extend(check_expr_for_undefined(
                    comparator, func_name, symbols, resolver, builtins,
                ));
            }
        }
        ast::Expr::List(list) => {
            for elt in &list.elts {
                errors.extend(check_expr_for_undefined(
                    elt, func_name, symbols, resolver, builtins,
                ));
            }
        }
        ast::Expr::Subscript(subscript) => {
            errors.extend(check_expr_for_undefined(
                &subscript.value,
                func_name,
                symbols,
                resolver,
                builtins,
            ));
        }
        _ => {}
    }

    errors
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
        line: 0, // Line info will be added when converting to DebtItem
        column: 0,
        function: func_name.to_string(),
    }]
}

/// Check attribute access for missing imports
fn check_attribute_access(
    attr: &ast::ExprAttribute,
    _resolver: &EnhancedImportResolver,
) -> Vec<StaticError> {
    if let ast::Expr::Name(base) = &*attr.value {
        let module_name = base.id.to_string();

        // Skip common false positives
        if is_false_positive(&module_name) {
            return Vec::new();
        }

        // For now, we skip module import checking since it requires
        // checking against the module-level imports, not function-level
        // This will be enhanced in a future iteration
    }

    Vec::new()
}

/// Check if name should be filtered as false positive
fn is_false_positive(name: &str) -> bool {
    matches!(name, "self" | "cls")
}

/// Get Python 3.8+ builtins
fn python_builtins() -> HashSet<String> {
    [
        // Functions
        "abs",
        "all",
        "any",
        "ascii",
        "bin",
        "bool",
        "breakpoint",
        "bytearray",
        "bytes",
        "callable",
        "chr",
        "classmethod",
        "compile",
        "complex",
        "delattr",
        "dict",
        "dir",
        "divmod",
        "enumerate",
        "eval",
        "exec",
        "filter",
        "float",
        "format",
        "frozenset",
        "getattr",
        "globals",
        "hasattr",
        "hash",
        "help",
        "hex",
        "id",
        "input",
        "int",
        "isinstance",
        "issubclass",
        "iter",
        "len",
        "list",
        "locals",
        "map",
        "max",
        "memoryview",
        "min",
        "next",
        "object",
        "oct",
        "open",
        "ord",
        "pow",
        "print",
        "property",
        "range",
        "repr",
        "reversed",
        "round",
        "set",
        "setattr",
        "slice",
        "sorted",
        "staticmethod",
        "str",
        "sum",
        "super",
        "tuple",
        "type",
        "vars",
        "zip",
        "__import__",
        // Constants
        "True",
        "False",
        "None",
        "NotImplemented",
        "Ellipsis",
        // Common exceptions
        "Exception",
        "ValueError",
        "TypeError",
        "KeyError",
        "AttributeError",
        "ImportError",
        "IndexError",
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
        ast::Stmt::Assign(assign) => vec![&assign.value],
        ast::Stmt::AugAssign(aug) => vec![&aug.value],
        _ => Vec::new(),
    }
}

/// Convert StaticError to DebtItem
pub fn to_debt_item(error: &StaticError, file: &Path) -> DebtItem {
    match error {
        StaticError::UndefinedVariable {
            name,
            line,
            column,
            function,
        } => DebtItem {
            id: format!("undefined-{}-{}", name, line),
            category: DebtCategory::CodeSmell,
            severity: Severity::Critical,
            location: SourceLocation::new(file.to_path_buf(), *line, *column),
            description: format!("Undefined variable '{}' in function '{}'", name, function),
            impact: 0.9,
            effort: 0.3,
            priority: 0.9,
            suggestions: vec![format!("Define '{}' before use or import it", name)],
        },
        StaticError::MissingImport {
            module,
            line,
            usage,
        } => DebtItem {
            id: format!("missing-import-{}-{}", module, line),
            category: DebtCategory::CodeSmell,
            severity: Severity::Critical,
            location: SourceLocation::new(file.to_path_buf(), *line, 0),
            description: format!("Missing import: {}", module),
            impact: 0.9,
            effort: 0.2,
            priority: 0.9,
            suggestions: vec![format!("Add 'import {}' (used as: {})", module, usage)],
        },
    }
}

/// Convert all errors to debt items
pub fn errors_to_debt_items(result: &StaticAnalysisResult, file: &Path) -> Vec<DebtItem> {
    result
        .errors
        .iter()
        .map(|e| to_debt_item(e, file))
        .collect()
}

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
    fn test_self_not_flagged() {
        let code = r#"
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

        // risky() is undefined, but we're testing that e is not flagged
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            result.errors[0],
            StaticError::UndefinedVariable { ref name, .. } if name == "risky"
        ));
    }

    #[test]
    fn test_context_manager_variable() {
        let code = r#"
def process():
    with open("file.txt") as f:
        print(f.read())
"#;
        let ast = parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let resolver = EnhancedImportResolver::new();
        let result = analyze_static_errors(&ast, &resolver);

        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_parameters_are_defined() {
        let code = r#"
def add(a, b):
    return a + b
"#;
        let ast = parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let resolver = EnhancedImportResolver::new();
        let result = analyze_static_errors(&ast, &resolver);

        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_assignment_defines_variable() {
        let code = r#"
def process():
    x = 10
    return x + 5
"#;
        let ast = parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let resolver = EnhancedImportResolver::new();
        let result = analyze_static_errors(&ast, &resolver);

        assert_eq!(result.errors.len(), 0);
    }
}
