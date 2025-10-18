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
use lazy_static::lazy_static;
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
    let imported_modules = extract_imported_modules(&mod_module.body);
    let errors = analyze_all_functions(&mod_module.body, import_resolver, builtins, &imported_modules);

    StaticAnalysisResult { errors }
}

/// Analyze all function definitions in module
fn analyze_all_functions(
    stmts: &[ast::Stmt],
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    stmts
        .iter()
        .filter_map(|stmt| {
            if let ast::Stmt::FunctionDef(func) = stmt {
                Some(analyze_function(func, resolver, builtins, imported_modules))
            } else {
                None
            }
        })
        .flatten()
        .collect()
}

/// Extract all imported module names from statements
fn extract_imported_modules(stmts: &[ast::Stmt]) -> HashSet<String> {
    stmts
        .iter()
        .flat_map(extract_modules_from_stmt)
        .collect()
}

/// Extract module names from single import statement
fn extract_modules_from_stmt(stmt: &ast::Stmt) -> Vec<String> {
    match stmt {
        ast::Stmt::Import(import) => extract_from_import(import),
        ast::Stmt::ImportFrom(import_from) => extract_from_import_from(import_from),
        _ => Vec::new(),
    }
}

/// Extract module names from import statement
fn extract_from_import(import: &ast::StmtImport) -> Vec<String> {
    import
        .names
        .iter()
        .flat_map(|alias| {
            let mut names = vec![alias.name.as_str().to_string()];
            if let Some(alias_name) = &alias.asname {
                names.push(alias_name.as_str().to_string());
            }
            names
        })
        .collect()
}

/// Extract module name from import-from statement
fn extract_from_import_from(import_from: &ast::StmtImportFrom) -> Vec<String> {
    import_from
        .module
        .as_ref()
        .map(|m| vec![m.as_str().to_string()])
        .unwrap_or_default()
}

/// Analyze single function for errors
fn analyze_function(
    func: &ast::StmtFunctionDef,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    let symbols = extract_local_symbols(func);
    find_undefined_names(
        &func.body,
        func.name.as_str(),
        &symbols,
        resolver,
        builtins,
        imported_modules,
    )
}

/// Extract all locally defined symbols from function
fn extract_local_symbols(func: &ast::StmtFunctionDef) -> LocalSymbols {
    let mut symbols = LocalSymbols::new();
    add_implicit_params(&mut symbols);
    extract_function_parameters(&func.args, &mut symbols);
    collect_definitions(&func.body, &mut symbols);
    symbols
}

/// Add implicit parameters (self, cls)
fn add_implicit_params(symbols: &mut LocalSymbols) {
    symbols.insert("self".to_string());
    symbols.insert("cls".to_string());
}

/// Extract parameter names from function arguments
fn extract_function_parameters(args: &ast::Arguments, symbols: &mut LocalSymbols) {
    for arg in &args.args {
        symbols.insert(arg.def.arg.to_string());
    }

    if let Some(vararg) = &args.vararg {
        symbols.insert(vararg.arg.to_string());
    }

    for kwarg in &args.kwonlyargs {
        symbols.insert(kwarg.def.arg.to_string());
    }

    if let Some(kwarg) = &args.kwarg {
        symbols.insert(kwarg.arg.to_string());
    }
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
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    stmts
        .iter()
        .flat_map(|stmt| {
            check_stmt_for_undefined(stmt, func_name, symbols, resolver, builtins, imported_modules)
        })
        .collect()
}

/// Check single statement for undefined references
fn check_stmt_for_undefined(
    stmt: &ast::Stmt,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    let mut errors = check_stmt_expressions(stmt, func_name, symbols, resolver, builtins, imported_modules);
    errors.extend(check_nested_blocks(stmt, func_name, symbols, resolver, builtins, imported_modules));
    errors
}

/// Check expressions within a statement
fn check_stmt_expressions(
    stmt: &ast::Stmt,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    extract_expressions(stmt)
        .iter()
        .flat_map(|expr| {
            check_expr_for_undefined(expr, func_name, symbols, resolver, builtins, imported_modules)
        })
        .collect()
}

/// Check nested blocks in statement
fn check_nested_blocks(
    stmt: &ast::Stmt,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    match stmt {
        ast::Stmt::If(if_stmt) => check_if_stmt(if_stmt, func_name, symbols, resolver, builtins, imported_modules),
        ast::Stmt::While(while_stmt) => check_while_stmt(while_stmt, func_name, symbols, resolver, builtins, imported_modules),
        ast::Stmt::For(for_stmt) => check_for_stmt(for_stmt, func_name, symbols, resolver, builtins, imported_modules),
        ast::Stmt::Try(try_stmt) => check_try_stmt(try_stmt, func_name, symbols, resolver, builtins, imported_modules),
        ast::Stmt::With(with_stmt) => check_with_stmt(with_stmt, func_name, symbols, resolver, builtins, imported_modules),
        _ => Vec::new(),
    }
}

/// Check if statement blocks
fn check_if_stmt(
    if_stmt: &ast::StmtIf,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    let mut errors = find_undefined_names(&if_stmt.body, func_name, symbols, resolver, builtins, imported_modules);
    errors.extend(find_undefined_names(&if_stmt.orelse, func_name, symbols, resolver, builtins, imported_modules));
    errors
}

/// Check while statement block
fn check_while_stmt(
    while_stmt: &ast::StmtWhile,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    find_undefined_names(&while_stmt.body, func_name, symbols, resolver, builtins, imported_modules)
}

/// Check for statement block
fn check_for_stmt(
    for_stmt: &ast::StmtFor,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    find_undefined_names(&for_stmt.body, func_name, symbols, resolver, builtins, imported_modules)
}

/// Check try statement blocks
fn check_try_stmt(
    try_stmt: &ast::StmtTry,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    let mut errors = find_undefined_names(&try_stmt.body, func_name, symbols, resolver, builtins, imported_modules);

    for handler in &try_stmt.handlers {
        let ast::ExceptHandler::ExceptHandler(h) = handler;
        errors.extend(find_undefined_names(&h.body, func_name, symbols, resolver, builtins, imported_modules));
    }

    errors
}

/// Check with statement block
fn check_with_stmt(
    with_stmt: &ast::StmtWith,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    find_undefined_names(&with_stmt.body, func_name, symbols, resolver, builtins, imported_modules)
}

/// Check expression for undefined references
fn check_expr_for_undefined(
    expr: &ast::Expr,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    match expr {
        ast::Expr::Name(name) if matches!(name.ctx, ast::ExprContext::Load) => {
            check_name_reference(name, func_name, symbols, builtins)
        }
        ast::Expr::Attribute(attr) => {
            check_attribute_expr(attr, func_name, symbols, resolver, builtins, imported_modules)
        }
        ast::Expr::Call(call) => {
            check_call_expr(call, func_name, symbols, resolver, builtins, imported_modules)
        }
        ast::Expr::BinOp(binop) => {
            check_binop_expr(binop, func_name, symbols, resolver, builtins, imported_modules)
        }
        ast::Expr::Compare(compare) => {
            check_compare_expr(compare, func_name, symbols, resolver, builtins, imported_modules)
        }
        ast::Expr::List(list) => {
            check_list_expr(list, func_name, symbols, resolver, builtins, imported_modules)
        }
        ast::Expr::Subscript(subscript) => {
            check_subscript_expr(subscript, func_name, symbols, resolver, builtins, imported_modules)
        }
        _ => Vec::new(),
    }
}

/// Check attribute expression
fn check_attribute_expr(
    attr: &ast::ExprAttribute,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    let mut errors = check_attribute_access(attr, resolver, imported_modules, symbols);

    if !matches!(&*attr.value, ast::Expr::Name(_)) {
        errors.extend(check_expr_for_undefined(
            &attr.value,
            func_name,
            symbols,
            resolver,
            builtins,
            imported_modules,
        ));
    }

    errors
}

/// Check call expression
fn check_call_expr(
    call: &ast::ExprCall,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    let mut errors = check_expr_for_undefined(
        &call.func,
        func_name,
        symbols,
        resolver,
        builtins,
        imported_modules,
    );

    for arg in &call.args {
        errors.extend(check_expr_for_undefined(
            arg,
            func_name,
            symbols,
            resolver,
            builtins,
            imported_modules,
        ));
    }

    errors
}

/// Check binary operation expression
fn check_binop_expr(
    binop: &ast::ExprBinOp,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    let mut errors = check_expr_for_undefined(
        &binop.left,
        func_name,
        symbols,
        resolver,
        builtins,
        imported_modules,
    );

    errors.extend(check_expr_for_undefined(
        &binop.right,
        func_name,
        symbols,
        resolver,
        builtins,
        imported_modules,
    ));

    errors
}

/// Check comparison expression
fn check_compare_expr(
    compare: &ast::ExprCompare,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    let mut errors = check_expr_for_undefined(
        &compare.left,
        func_name,
        symbols,
        resolver,
        builtins,
        imported_modules,
    );

    for comparator in &compare.comparators {
        errors.extend(check_expr_for_undefined(
            comparator,
            func_name,
            symbols,
            resolver,
            builtins,
            imported_modules,
        ));
    }

    errors
}

/// Check list expression
fn check_list_expr(
    list: &ast::ExprList,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    list.elts
        .iter()
        .flat_map(|elt| {
            check_expr_for_undefined(elt, func_name, symbols, resolver, builtins, imported_modules)
        })
        .collect()
}

/// Check subscript expression
fn check_subscript_expr(
    subscript: &ast::ExprSubscript,
    func_name: &str,
    symbols: &LocalSymbols,
    resolver: &EnhancedImportResolver,
    builtins: &HashSet<String>,
    imported_modules: &HashSet<String>,
) -> Vec<StaticError> {
    check_expr_for_undefined(
        &subscript.value,
        func_name,
        symbols,
        resolver,
        builtins,
        imported_modules,
    )
}

/// Check if name reference is defined
fn check_name_reference(
    name: &ast::ExprName,
    func_name: &str,
    symbols: &LocalSymbols,
    builtins: &HashSet<String>,
) -> Vec<StaticError> {
    let name_str = name.id.to_string();

    if is_name_defined(&name_str, symbols, builtins) {
        return Vec::new();
    }

    vec![create_undefined_var_error(&name_str, func_name)]
}

/// Check if name is defined in symbols or builtins
fn is_name_defined(name: &str, symbols: &LocalSymbols, builtins: &HashSet<String>) -> bool {
    is_false_positive(name) || symbols.contains(name) || builtins.contains(name)
}

/// Create undefined variable error
fn create_undefined_var_error(name: &str, func_name: &str) -> StaticError {
    StaticError::UndefinedVariable {
        name: name.to_string(),
        line: 0,
        column: 0,
        function: func_name.to_string(),
    }
}

/// Check attribute access for missing imports
fn check_attribute_access(
    attr: &ast::ExprAttribute,
    _resolver: &EnhancedImportResolver,
    imported_modules: &HashSet<String>,
    symbols: &LocalSymbols,
) -> Vec<StaticError> {
    if let ast::Expr::Name(base) = &*attr.value {
        return check_module_attribute_access(base, attr, imported_modules, symbols);
    }
    Vec::new()
}

/// Check if module attribute access has proper import
fn check_module_attribute_access(
    base: &ast::ExprName,
    attr: &ast::ExprAttribute,
    imported_modules: &HashSet<String>,
    symbols: &LocalSymbols,
) -> Vec<StaticError> {
    let module_name = base.id.to_string();

    if should_skip_module_check(&module_name, symbols) {
        return Vec::new();
    }

    if !imported_modules.contains(&module_name) {
        return vec![create_missing_import_error(&module_name, attr)];
    }

    Vec::new()
}

/// Check if module check should be skipped
fn should_skip_module_check(module_name: &str, symbols: &LocalSymbols) -> bool {
    is_false_positive(module_name) || symbols.contains(module_name)
}

/// Create missing import error
fn create_missing_import_error(module_name: &str, attr: &ast::ExprAttribute) -> StaticError {
    let usage = format!("{}.{}", module_name, attr.attr.as_str());
    StaticError::MissingImport {
        module: module_name.to_string(),
        line: 0,
        usage,
    }
}

/// Check if name should be filtered as false positive
fn is_false_positive(name: &str) -> bool {
    matches!(name, "self" | "cls")
}

lazy_static! {
    /// Python 3.8+ builtins
    static ref PYTHON_BUILTINS: HashSet<String> = {
        vec![
            // Functions
            "abs", "all", "any", "ascii", "bin", "bool", "breakpoint", "bytearray", "bytes",
            "callable", "chr", "classmethod", "compile", "complex", "delattr", "dict", "dir",
            "divmod", "enumerate", "eval", "exec", "filter", "float", "format", "frozenset",
            "getattr", "globals", "hasattr", "hash", "help", "hex", "id", "input", "int",
            "isinstance", "issubclass", "iter", "len", "list", "locals", "map", "max",
            "memoryview", "min", "next", "object", "oct", "open", "ord", "pow", "print",
            "property", "range", "repr", "reversed", "round", "set", "setattr", "slice",
            "sorted", "staticmethod", "str", "sum", "super", "tuple", "type", "vars", "zip",
            "__import__",
            // Constants
            "True", "False", "None", "NotImplemented", "Ellipsis",
            // Common exceptions
            "Exception", "ValueError", "TypeError", "KeyError", "AttributeError",
            "ImportError", "IndexError",
        ]
        .iter()
        .map(|&s| s.to_string())
        .collect()
    };
}

/// Get Python 3.8+ builtins
fn python_builtins() -> &'static HashSet<String> {
    &PYTHON_BUILTINS
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

/// Create debt item for undefined variable error
fn create_undefined_var_debt_item(
    name: &str,
    line: usize,
    column: usize,
    function: &str,
    file: &Path,
) -> DebtItem {
    DebtItem {
        id: format!("undefined-{}-{}", name, line),
        category: DebtCategory::CodeSmell,
        severity: Severity::Critical,
        location: SourceLocation::new(file.to_path_buf(), line, column),
        description: format!("Undefined variable '{}' in function '{}'", name, function),
        impact: 0.9,
        effort: 0.3,
        priority: 0.9,
        suggestions: vec![format!("Define '{}' before use or import it", name)],
    }
}

/// Create debt item for missing import error
fn create_missing_import_debt_item(
    module: &str,
    line: usize,
    usage: &str,
    file: &Path,
) -> DebtItem {
    DebtItem {
        id: format!("missing-import-{}-{}", module, line),
        category: DebtCategory::CodeSmell,
        severity: Severity::Critical,
        location: SourceLocation::new(file.to_path_buf(), line, 0),
        description: format!("Missing import: {}", module),
        impact: 0.9,
        effort: 0.2,
        priority: 0.9,
        suggestions: vec![format!("Add 'import {}' (used as: {})", module, usage)],
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
        } => create_undefined_var_debt_item(name, *line, *column, function, file),
        StaticError::MissingImport {
            module,
            line,
            usage,
        } => create_missing_import_debt_item(module, *line, usage, file),
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

    #[test]
    fn test_issue_9_missing_import() {
        let code = r#"
def test(param):
    wx.CallAfter(param)
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
}
