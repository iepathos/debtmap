use rustpython_parser::ast::{self};
use std::collections::HashSet;

/// Detects whether a Python function is pure through static analysis
pub struct PythonPurityDetector {
    /// Known pure built-in functions
    known_pure_functions: HashSet<String>,
    /// Known impure built-in functions  
    known_impure_functions: HashSet<String>,
    /// Detected side effects
    side_effects: Vec<SideEffect>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SideEffect {
    GlobalWrite(String),
    AttributeMutation,
    IOOperation,
    ExternalCall(String),
    ExceptionRaising,
}

#[derive(Debug, Clone)]
pub struct PurityAnalysis {
    pub is_pure: bool,
    pub confidence: f32,
    pub side_effects: Vec<SideEffect>,
}

impl Default for PythonPurityDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonPurityDetector {
    pub fn new() -> Self {
        let mut known_pure_functions = HashSet::new();
        // Common pure built-in functions
        for func in &[
            "len",
            "abs",
            "min",
            "max",
            "sum",
            "all",
            "any",
            "zip",
            "map",
            "filter",
            "enumerate",
            "range",
            "sorted",
            "reversed",
            "isinstance",
            "issubclass",
            "hasattr",
            "getattr",
            "str",
            "int",
            "float",
            "bool",
            "list",
            "tuple",
            "set",
            "dict",
            "frozenset",
            "round",
            "pow",
            "divmod",
            "hash",
            "id",
            "type",
        ] {
            known_pure_functions.insert(func.to_string());
        }

        let mut known_impure_functions = HashSet::new();
        // Common impure built-in functions
        for func in &[
            "print",
            "input",
            "open",
            "write",
            "read",
            "exec",
            "eval",
            "compile",
            "globals",
            "locals",
            "setattr",
            "delattr",
            "__import__",
            "reload",
        ] {
            known_impure_functions.insert(func.to_string());
        }

        Self {
            known_pure_functions,
            known_impure_functions,
            side_effects: Vec::new(),
        }
    }

    /// Analyzes a function to determine if it's pure
    pub fn analyze_function(&mut self, func_def: &ast::StmtFunctionDef) -> PurityAnalysis {
        // Reset side effects
        self.side_effects.clear();

        // In Python, we can't determine parameter mutability from signature alone
        // We'll analyze the function body for actual mutations

        // Analyze the function body
        self.analyze_body(&func_def.body);

        // Calculate confidence based on detected patterns
        let confidence = self.calculate_confidence();

        PurityAnalysis {
            is_pure: self.side_effects.is_empty(),
            confidence,
            side_effects: self.side_effects.clone(),
        }
    }

    /// Analyzes an async function to determine if it's pure
    pub fn analyze_async_function(
        &mut self,
        func_def: &ast::StmtAsyncFunctionDef,
    ) -> PurityAnalysis {
        // Reset side effects
        self.side_effects.clear();

        // Async functions are generally impure by nature (I/O operations)
        self.side_effects.push(SideEffect::IOOperation);

        // Still analyze the body for other side effects
        self.analyze_body(&func_def.body);

        let confidence = self.calculate_confidence();

        PurityAnalysis {
            is_pure: false, // Async functions are inherently impure
            confidence,
            side_effects: self.side_effects.clone(),
        }
    }

    fn analyze_body(&mut self, stmts: &[ast::Stmt]) {
        for stmt in stmts {
            self.analyze_stmt(stmt);
        }
    }

    fn analyze_stmt(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::Global(global) => {
                // Global statement indicates potential for modifying global state
                for name in &global.names {
                    self.side_effects
                        .push(SideEffect::GlobalWrite(name.to_string()));
                }
            }
            ast::Stmt::Nonlocal(nonlocal) => {
                // Nonlocal statement indicates modification of enclosing scope
                for name in &nonlocal.names {
                    self.side_effects
                        .push(SideEffect::GlobalWrite(format!("nonlocal:{}", name)));
                }
            }
            ast::Stmt::Expr(expr_stmt) => {
                self.analyze_expr(&expr_stmt.value);
            }
            ast::Stmt::Return(return_stmt) => {
                if let Some(value) = &return_stmt.value {
                    self.analyze_expr(value);
                }
            }
            ast::Stmt::Assign(assign) => {
                // Check if we're assigning to global or external state
                for target in &assign.targets {
                    if self.is_external_mutation(target) {
                        self.side_effects.push(SideEffect::AttributeMutation);
                    }
                }
                self.analyze_expr(&assign.value);
            }
            ast::Stmt::AugAssign(aug_assign) => {
                if self.is_external_mutation(&aug_assign.target) {
                    self.side_effects.push(SideEffect::AttributeMutation);
                }
                self.analyze_expr(&aug_assign.value);
            }
            ast::Stmt::AnnAssign(ann_assign) => {
                if let Some(value) = &ann_assign.value {
                    self.analyze_expr(value);
                }
            }
            ast::Stmt::For(for_stmt) => {
                self.analyze_expr(&for_stmt.iter);
                self.analyze_body(&for_stmt.body);
                self.analyze_body(&for_stmt.orelse);
            }
            ast::Stmt::While(while_stmt) => {
                self.analyze_expr(&while_stmt.test);
                self.analyze_body(&while_stmt.body);
                self.analyze_body(&while_stmt.orelse);
            }
            ast::Stmt::If(if_stmt) => {
                self.analyze_expr(&if_stmt.test);
                self.analyze_body(&if_stmt.body);
                self.analyze_body(&if_stmt.orelse);
            }
            ast::Stmt::With(with_stmt) => {
                // Context managers often involve I/O or resource management
                for item in &with_stmt.items {
                    self.analyze_expr(&item.context_expr);
                    // Opening files, acquiring locks, etc. are impure
                    if self.is_io_context(&item.context_expr) {
                        self.side_effects.push(SideEffect::IOOperation);
                    }
                }
                self.analyze_body(&with_stmt.body);
            }
            ast::Stmt::AsyncWith(async_with) => {
                // Async context managers are inherently impure
                self.side_effects.push(SideEffect::IOOperation);
                self.analyze_body(&async_with.body);
            }
            ast::Stmt::Raise(raise_stmt) => {
                // Raising exceptions is a side effect
                self.side_effects.push(SideEffect::ExceptionRaising);
                if let Some(exc) = &raise_stmt.exc {
                    self.analyze_expr(exc);
                }
            }
            ast::Stmt::Try(try_stmt) => {
                self.analyze_body(&try_stmt.body);
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    self.analyze_body(&h.body);
                }
                self.analyze_body(&try_stmt.orelse);
                self.analyze_body(&try_stmt.finalbody);
            }
            ast::Stmt::Assert(assert_stmt) => {
                // Assertions can raise exceptions
                self.analyze_expr(&assert_stmt.test);
                if let Some(msg) = &assert_stmt.msg {
                    self.analyze_expr(msg);
                }
            }
            ast::Stmt::Import(_) | ast::Stmt::ImportFrom(_) => {
                // Imports can have side effects in Python
                self.side_effects
                    .push(SideEffect::ExternalCall("import".to_string()));
            }
            ast::Stmt::FunctionDef(func_def) => {
                // Nested function definitions
                self.analyze_body(&func_def.body);
            }
            ast::Stmt::AsyncFunctionDef(func_def) => {
                // Nested async function definitions
                self.analyze_body(&func_def.body);
            }
            ast::Stmt::ClassDef(class_def) => {
                // Class definitions
                self.analyze_body(&class_def.body);
            }
            _ => {}
        }
    }

    fn analyze_expr(&mut self, expr: &ast::Expr) {
        match expr {
            ast::Expr::Call(call) => {
                // Check if it's a known impure function call
                if let ast::Expr::Name(name) = &call.func.as_ref() {
                    let func_name = name.id.to_string();
                    if self.known_impure_functions.contains(&func_name) {
                        self.side_effects.push(SideEffect::IOOperation);
                    } else if !self.known_pure_functions.contains(&func_name) {
                        // Unknown function - be conservative
                        self.side_effects.push(SideEffect::ExternalCall(func_name));
                    }
                }
                // Check for method calls that might be impure
                if let ast::Expr::Attribute(attr) = &call.func.as_ref() {
                    let attr_name = attr.attr.to_string();
                    if self.is_mutation_method(&attr_name) {
                        self.side_effects.push(SideEffect::AttributeMutation);
                    }
                    if self.is_io_method(&attr_name) {
                        self.side_effects.push(SideEffect::IOOperation);
                    }
                }
                // Analyze arguments
                for arg in &call.args {
                    self.analyze_expr(arg);
                }
            }
            ast::Expr::Lambda(lambda) => {
                // Lambda functions can contain side effects
                self.analyze_expr(&lambda.body);
            }
            ast::Expr::ListComp(comp) => {
                self.analyze_expr(&comp.elt);
                for generator in &comp.generators {
                    self.analyze_expr(&generator.iter);
                    for cond in &generator.ifs {
                        self.analyze_expr(cond);
                    }
                }
            }
            ast::Expr::SetComp(comp) => {
                self.analyze_expr(&comp.elt);
                for generator in &comp.generators {
                    self.analyze_expr(&generator.iter);
                    for cond in &generator.ifs {
                        self.analyze_expr(cond);
                    }
                }
            }
            ast::Expr::GeneratorExp(comp) => {
                self.analyze_expr(&comp.elt);
                for generator in &comp.generators {
                    self.analyze_expr(&generator.iter);
                    for cond in &generator.ifs {
                        self.analyze_expr(cond);
                    }
                }
            }
            ast::Expr::DictComp(comp) => {
                self.analyze_expr(&comp.key);
                self.analyze_expr(&comp.value);
                for generator in &comp.generators {
                    self.analyze_expr(&generator.iter);
                    for cond in &generator.ifs {
                        self.analyze_expr(cond);
                    }
                }
            }
            ast::Expr::Yield(_) | ast::Expr::YieldFrom(_) => {
                // Generators can be pure if their body is pure
                // We'll analyze the yielded value but not mark as impure
            }
            ast::Expr::Await(await_expr) => {
                // Await expressions are inherently impure (async I/O)
                self.side_effects.push(SideEffect::IOOperation);
                self.analyze_expr(&await_expr.value);
            }
            ast::Expr::BinOp(binop) => {
                self.analyze_expr(&binop.left);
                self.analyze_expr(&binop.right);
            }
            ast::Expr::UnaryOp(unaryop) => {
                self.analyze_expr(&unaryop.operand);
            }
            ast::Expr::IfExp(ifexp) => {
                self.analyze_expr(&ifexp.test);
                self.analyze_expr(&ifexp.body);
                self.analyze_expr(&ifexp.orelse);
            }
            ast::Expr::Compare(compare) => {
                self.analyze_expr(&compare.left);
                for comp in &compare.comparators {
                    self.analyze_expr(comp);
                }
            }
            ast::Expr::BoolOp(boolop) => {
                for value in &boolop.values {
                    self.analyze_expr(value);
                }
            }
            ast::Expr::Subscript(subscript) => {
                self.analyze_expr(&subscript.value);
                self.analyze_expr(&subscript.slice);
            }
            ast::Expr::Slice(slice) => {
                if let Some(lower) = &slice.lower {
                    self.analyze_expr(lower);
                }
                if let Some(upper) = &slice.upper {
                    self.analyze_expr(upper);
                }
                if let Some(step) = &slice.step {
                    self.analyze_expr(step);
                }
            }
            ast::Expr::List(list) => {
                for elt in &list.elts {
                    self.analyze_expr(elt);
                }
            }
            ast::Expr::Tuple(tuple) => {
                for elt in &tuple.elts {
                    self.analyze_expr(elt);
                }
            }
            ast::Expr::Set(set) => {
                for elt in &set.elts {
                    self.analyze_expr(elt);
                }
            }
            ast::Expr::Dict(dict) => {
                for key in dict.keys.iter().flatten() {
                    self.analyze_expr(key);
                }
                for value in &dict.values {
                    self.analyze_expr(value);
                }
            }
            _ => {}
        }
    }

    fn is_external_mutation(&self, target: &ast::Expr) -> bool {
        match target {
            ast::Expr::Attribute(_) => true, // Attribute assignment might mutate objects
            ast::Expr::Subscript(_) => true, // Subscript assignment might mutate containers
            ast::Expr::Name(name) => {
                // Check if it's a local variable or parameter
                // This is a simplified check - in reality we'd need scope analysis
                let name_str = name.id.to_string();
                !name_str.starts_with('_') && name_str != "self"
            }
            _ => false,
        }
    }

    fn is_io_context(&self, expr: &ast::Expr) -> bool {
        if let ast::Expr::Call(call) = expr {
            if let ast::Expr::Name(name) = &call.func.as_ref() {
                let name_str = name.id.to_string();
                return name_str == "open"
                    || name_str.contains("file")
                    || name_str.contains("socket");
            }
        }
        false
    }

    fn is_mutation_method(&self, method_name: &str) -> bool {
        matches!(
            method_name,
            "append"
                | "extend"
                | "insert"
                | "remove"
                | "pop"
                | "clear"
                | "sort"
                | "reverse"
                | "update"
                | "add"
                | "discard"
                | "setdefault"
                | "popitem"
                | "setattr"
                | "delattr"
                | "__setitem__"
                | "__delitem__"
                | "__setattr__"
                | "__delattr__"
        )
    }

    fn is_io_method(&self, method_name: &str) -> bool {
        matches!(
            method_name,
            "write"
                | "writelines"
                | "read"
                | "readline"
                | "readlines"
                | "flush"
                | "seek"
                | "tell"
                | "truncate"
                | "close"
                | "send"
                | "recv"
                | "sendall"
                | "sendto"
                | "recvfrom"
        )
    }

    fn calculate_confidence(&self) -> f32 {
        if self.side_effects.is_empty() {
            // High confidence if no side effects detected
            0.85 // Not 100% due to potential false negatives
        } else if self.side_effects.len() == 1 {
            match &self.side_effects[0] {
                SideEffect::ExternalCall(name)
                    if !self.known_impure_functions.contains(name.as_str()) =>
                {
                    // Lower confidence for unknown external calls
                    0.6
                }
                _ => 0.9, // High confidence for clearly detected side effects
            }
        } else {
            // Very high confidence with multiple side effects
            0.95
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_and_analyze(code: &str) -> PurityAnalysis {
        let module = rustpython_parser::parse(code, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse Python code");

        let mut detector = PythonPurityDetector::new();

        // Find the first function definition in the module
        if let ast::Mod::Module(module) = module {
            for stmt in &module.body {
                if let ast::Stmt::FunctionDef(func_def) = stmt {
                    return detector.analyze_function(func_def);
                }
            }
        }

        panic!("No function found in code");
    }

    #[test]
    fn test_pure_function() {
        let analysis = parse_and_analyze(
            r#"
def add(a, b):
    return a + b
"#,
        );
        assert!(analysis.is_pure);
        assert!(analysis.side_effects.is_empty());
    }

    #[test]
    fn test_function_with_print() {
        let analysis = parse_and_analyze(
            r#"
def debug_add(a, b):
    print(f"Adding {a} + {b}")
    return a + b
"#,
        );
        assert!(!analysis.is_pure);
        assert!(analysis.side_effects.contains(&SideEffect::IOOperation));
    }

    #[test]
    fn test_function_with_global() {
        let analysis = parse_and_analyze(
            r#"
def modify_global():
    global counter
    counter += 1
    return counter
"#,
        );
        assert!(!analysis.is_pure);
        assert!(analysis
            .side_effects
            .iter()
            .any(|s| matches!(s, SideEffect::GlobalWrite(_))));
    }

    #[test]
    fn test_function_with_list_mutation() {
        let analysis = parse_and_analyze(
            r#"
def append_to_list(lst, item):
    lst.append(item)
    return lst
"#,
        );
        assert!(!analysis.is_pure);
        assert!(analysis
            .side_effects
            .contains(&SideEffect::AttributeMutation));
    }

    #[test]
    fn test_pure_list_comprehension() {
        let analysis = parse_and_analyze(
            r#"
def double_list(items):
    return [x * 2 for x in items]
"#,
        );
        assert!(analysis.is_pure);
        assert!(analysis.side_effects.is_empty());
    }

    #[test]
    fn test_function_with_file_io() {
        let analysis = parse_and_analyze(
            r#"
def read_file(filename):
    with open(filename, 'r') as f:
        return f.read()
"#,
        );
        assert!(!analysis.is_pure);
        assert!(analysis.side_effects.contains(&SideEffect::IOOperation));
    }

    #[test]
    fn test_function_with_exception() {
        let analysis = parse_and_analyze(
            r#"
def divide(a, b):
    if b == 0:
        raise ValueError("Cannot divide by zero")
    return a / b
"#,
        );
        assert!(!analysis.is_pure);
        assert!(analysis
            .side_effects
            .contains(&SideEffect::ExceptionRaising));
    }

    #[test]
    fn test_pure_recursive_function() {
        let analysis = parse_and_analyze(
            r#"
def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)
"#,
        );
        // Recursive calls to self are considered pure if the function itself is pure
        // This is a simplified analysis - the external call is to itself
        assert!(!analysis.is_pure); // Conservative: marks as impure due to external call
    }
}
