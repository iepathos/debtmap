//! Python Type Tracking System
//!
//! Provides type inference and tracking for Python code to improve call graph accuracy.
//! Uses two-pass resolution for better method resolution and reduced false positives.

use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use rustpython_parser::ast;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Python type representation for tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PythonType {
    /// Class type (e.g., `MyClass`)
    Class(String),
    /// Instance of a class (e.g., `MyClass()`)
    Instance(String),
    /// Function or method
    Function(FunctionSignature),
    /// Module
    Module(String),
    /// Union of multiple possible types
    Union(Vec<PythonType>),
    /// Built-in type
    BuiltIn(String),
    /// Unknown type
    Unknown,
}

/// Function signature information
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<String>,
    pub return_type: Option<Box<PythonType>>,
}

/// Class information including hierarchy and members
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub bases: Vec<String>,
    pub methods: HashMap<String, FunctionId>,
    pub attributes: HashMap<String, PythonType>,
    pub static_methods: HashSet<String>,
    pub class_methods: HashSet<String>,
    pub properties: HashSet<String>,
}

/// Scope information for tracking variables
#[derive(Debug, Clone)]
pub struct Scope {
    pub variables: HashMap<String, PythonType>,
    pub parent: Option<Box<Scope>>,
}

impl Scope {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
            parent: None,
        }
    }

    fn with_parent(parent: Scope) -> Self {
        Self {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    fn lookup(&self, name: &str) -> Option<&PythonType> {
        self.variables
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }

    fn insert(&mut self, name: String, ty: PythonType) {
        self.variables.insert(name, ty);
    }
}

/// Python type tracker for improved call resolution
pub struct PythonTypeTracker {
    /// Local variable types in current scope
    pub local_types: HashMap<String, PythonType>,
    /// Class hierarchy information
    pub class_hierarchy: HashMap<String, ClassInfo>,
    /// Function signatures
    pub function_signatures: HashMap<FunctionId, FunctionSignature>,
    /// Current scope stack
    pub current_scope: Vec<Scope>,
    /// File path for generating function IDs
    file_path: PathBuf,
}

impl PythonTypeTracker {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            local_types: HashMap::new(),
            class_hierarchy: HashMap::new(),
            function_signatures: HashMap::new(),
            current_scope: vec![Scope::new()],
            file_path,
        }
    }

    /// Enter a new scope
    pub fn push_scope(&mut self) {
        let parent = self
            .current_scope
            .last()
            .cloned()
            .unwrap_or_else(Scope::new);
        self.current_scope.push(Scope::with_parent(parent));
    }

    /// Exit current scope
    pub fn pop_scope(&mut self) {
        if self.current_scope.len() > 1 {
            self.current_scope.pop();
        }
    }

    /// Get current scope
    fn current_scope_mut(&mut self) -> &mut Scope {
        self.current_scope.last_mut().expect("No current scope")
    }

    /// Infer type from expression
    pub fn infer_type(&self, expr: &ast::Expr) -> PythonType {
        match expr {
            // Literals
            ast::Expr::Constant(constant) => self.infer_constant_type(&constant.value),

            // Name lookup
            ast::Expr::Name(name) => self
                .current_scope
                .last()
                .and_then(|s| s.lookup(name.id.as_str()))
                .cloned()
                .unwrap_or(PythonType::Unknown),

            // Attribute access (e.g., obj.attr)
            ast::Expr::Attribute(attr) => {
                let base_type = self.infer_type(&attr.value);
                self.resolve_attribute(&base_type, attr.attr.as_str())
            }

            // Function/method calls
            ast::Expr::Call(call) => self.infer_call_return_type(call),

            // Binary operations
            ast::Expr::BinOp(binop) => self.infer_binop_type(&binop.left, &binop.right, &binop.op),

            // List literal
            ast::Expr::List(_) => PythonType::BuiltIn("list".to_string()),

            // Dict literal
            ast::Expr::Dict(_) => PythonType::BuiltIn("dict".to_string()),

            // Tuple literal
            ast::Expr::Tuple(_) => PythonType::BuiltIn("tuple".to_string()),

            // Set literal
            ast::Expr::Set(_) => PythonType::BuiltIn("set".to_string()),

            // Lambda
            ast::Expr::Lambda(_) => PythonType::Function(FunctionSignature {
                name: "<lambda>".to_string(),
                params: vec![],
                return_type: None,
            }),

            _ => PythonType::Unknown,
        }
    }

    /// Infer type from constant value
    fn infer_constant_type(&self, value: &ast::Constant) -> PythonType {
        match value {
            ast::Constant::Int(_) => PythonType::BuiltIn("int".to_string()),
            ast::Constant::Float(_) => PythonType::BuiltIn("float".to_string()),
            ast::Constant::Str(_) => PythonType::BuiltIn("str".to_string()),
            ast::Constant::Bool(_) => PythonType::BuiltIn("bool".to_string()),
            ast::Constant::None => PythonType::BuiltIn("None".to_string()),
            _ => PythonType::Unknown,
        }
    }

    /// Resolve attribute access on a type
    fn resolve_attribute(&self, base_type: &PythonType, attr_name: &str) -> PythonType {
        match base_type {
            PythonType::Instance(class_name) | PythonType::Class(class_name) => {
                if let Some(class_info) = self.class_hierarchy.get(class_name) {
                    if let Some(attr_type) = class_info.attributes.get(attr_name) {
                        return attr_type.clone();
                    }
                    // Check if it's a method
                    if class_info.methods.contains_key(attr_name) {
                        return PythonType::Function(FunctionSignature {
                            name: format!("{}.{}", class_name, attr_name),
                            params: vec![],
                            return_type: None,
                        });
                    }
                }
            }
            PythonType::Module(_module_name) => {
                // For module attributes, we'd need module-level tracking
                return PythonType::Unknown;
            }
            _ => {}
        }
        PythonType::Unknown
    }

    /// Infer return type from call
    fn infer_call_return_type(&self, call: &ast::ExprCall) -> PythonType {
        match &*call.func {
            ast::Expr::Name(name) => {
                // Check if it's a known class constructor
                let name_str = name.id.to_string();
                if self.class_hierarchy.contains_key(&name_str) {
                    return PythonType::Instance(name_str);
                }
                // Check built-in constructors
                match name.id.as_str() {
                    "list" => return PythonType::BuiltIn("list".to_string()),
                    "dict" => return PythonType::BuiltIn("dict".to_string()),
                    "set" => return PythonType::BuiltIn("set".to_string()),
                    "tuple" => return PythonType::BuiltIn("tuple".to_string()),
                    "str" => return PythonType::BuiltIn("str".to_string()),
                    "int" => return PythonType::BuiltIn("int".to_string()),
                    "float" => return PythonType::BuiltIn("float".to_string()),
                    "bool" => return PythonType::BuiltIn("bool".to_string()),
                    _ => {}
                }
            }
            ast::Expr::Attribute(attr) => {
                let _base_type = self.infer_type(&attr.value);
                // Method call return type would need more sophisticated tracking
                return PythonType::Unknown;
            }
            _ => {}
        }
        PythonType::Unknown
    }

    /// Infer type from binary operation
    fn infer_binop_type(
        &self,
        left: &ast::Expr,
        right: &ast::Expr,
        op: &ast::Operator,
    ) -> PythonType {
        let left_type = self.infer_type(left);
        let right_type = self.infer_type(right);

        // Simple numeric operation inference
        match (left_type, right_type, op) {
            (PythonType::BuiltIn(ref l), PythonType::BuiltIn(ref r), _)
                if l == "int" && r == "int" =>
            {
                match op {
                    ast::Operator::Div => PythonType::BuiltIn("float".to_string()),
                    _ => PythonType::BuiltIn("int".to_string()),
                }
            }
            (PythonType::BuiltIn(ref l), _, _) if l == "str" => match op {
                ast::Operator::Add | ast::Operator::Mult => PythonType::BuiltIn("str".to_string()),
                _ => PythonType::Unknown,
            },
            _ => PythonType::Unknown,
        }
    }

    /// Track assignment to update type information
    pub fn track_assignment(&mut self, target: &ast::Expr, value: &ast::Expr) {
        let inferred_type = self.infer_type(value);

        match target {
            ast::Expr::Name(name) => {
                self.current_scope_mut()
                    .insert(name.id.to_string(), inferred_type);
            }
            ast::Expr::Attribute(attr) => {
                // Track attribute assignments for class members
                if let ast::Expr::Name(name) = &*attr.value {
                    if name.id.as_str() == "self" {
                        // This is a self.attr assignment in a method
                        // We'd need to track the current class context
                    }
                }
            }
            _ => {}
        }
    }

    /// Extract class hierarchy from AST
    pub fn extract_class_info(&mut self, class_def: &ast::StmtClassDef) {
        let mut class_info = ClassInfo {
            name: class_def.name.to_string(),
            bases: vec![],
            methods: HashMap::new(),
            attributes: HashMap::new(),
            static_methods: HashSet::new(),
            class_methods: HashSet::new(),
            properties: HashSet::new(),
        };

        // Extract base classes
        for base in &class_def.bases {
            if let ast::Expr::Name(name) = base {
                class_info.bases.push(name.id.to_string());
            }
        }

        // Extract methods and attributes
        for stmt in &class_def.body {
            match stmt {
                ast::Stmt::FunctionDef(func_def) => {
                    let method_name = func_def.name.to_string();
                    let func_id = FunctionId {
                        name: format!("{}.{}", class_def.name, method_name),
                        file: self.file_path.clone(),
                        line: 0, // Would need source mapping for accurate line
                    };

                    // Check decorators for static/class methods/properties
                    for decorator in &func_def.decorator_list {
                        if let ast::Expr::Name(name) = decorator {
                            match name.id.as_str() {
                                "staticmethod" => {
                                    class_info.static_methods.insert(method_name.clone());
                                }
                                "classmethod" => {
                                    class_info.class_methods.insert(method_name.clone());
                                }
                                "property" => {
                                    class_info.properties.insert(method_name.clone());
                                }
                                _ => {}
                            }
                        }
                    }

                    class_info.methods.insert(method_name, func_id);
                }
                ast::Stmt::AnnAssign(ann_assign) => {
                    // Type-annotated class attributes
                    if let ast::Expr::Name(name) = &*ann_assign.target {
                        let attr_type = self.infer_type_from_annotation(&ann_assign.annotation);
                        class_info.attributes.insert(name.id.to_string(), attr_type);
                    }
                }
                _ => {}
            }
        }

        self.class_hierarchy
            .insert(class_def.name.to_string(), class_info);
    }

    /// Infer type from type annotation
    fn infer_type_from_annotation(&self, annotation: &ast::Expr) -> PythonType {
        match annotation {
            ast::Expr::Name(name) => match name.id.as_str() {
                "int" => PythonType::BuiltIn("int".to_string()),
                "float" => PythonType::BuiltIn("float".to_string()),
                "str" => PythonType::BuiltIn("str".to_string()),
                "bool" => PythonType::BuiltIn("bool".to_string()),
                "list" => PythonType::BuiltIn("list".to_string()),
                "dict" => PythonType::BuiltIn("dict".to_string()),
                "set" => PythonType::BuiltIn("set".to_string()),
                "tuple" => PythonType::BuiltIn("tuple".to_string()),
                class_name => {
                    if self.class_hierarchy.contains_key(class_name) {
                        PythonType::Class(class_name.to_string())
                    } else {
                        PythonType::Unknown
                    }
                }
            },
            _ => PythonType::Unknown,
        }
    }

    /// Resolve method call based on receiver type
    pub fn resolve_method_call(
        &self,
        receiver_type: &PythonType,
        method_name: &str,
    ) -> Option<FunctionId> {
        match receiver_type {
            PythonType::Instance(class_name) | PythonType::Class(class_name) => {
                self.resolve_method_in_hierarchy(class_name, method_name)
            }
            _ => None,
        }
    }

    /// Resolve method in class hierarchy (with inheritance)
    fn resolve_method_in_hierarchy(
        &self,
        class_name: &str,
        method_name: &str,
    ) -> Option<FunctionId> {
        // Check current class
        if let Some(class_info) = self.class_hierarchy.get(class_name) {
            if let Some(func_id) = class_info.methods.get(method_name) {
                return Some(func_id.clone());
            }

            // Check base classes
            for base in &class_info.bases {
                if let Some(func_id) = self.resolve_method_in_hierarchy(base, method_name) {
                    return Some(func_id);
                }
            }
        }
        None
    }
}

/// Unresolved call information for two-pass resolution
#[derive(Debug, Clone)]
pub struct UnresolvedCall {
    pub caller: FunctionId,
    pub call_expr: ast::Expr,
    pub receiver_type: Option<PythonType>,
    pub method_name: Option<String>,
    pub call_type: CallType,
}

/// Two-pass call graph extractor for Python
pub struct TwoPassExtractor {
    /// Phase one: collect all unresolved calls
    pub phase_one_calls: Vec<UnresolvedCall>,
    /// Type tracker
    pub type_tracker: PythonTypeTracker,
    /// Call graph being built
    pub call_graph: CallGraph,
    /// Set of known function IDs discovered in phase one
    known_functions: HashSet<FunctionId>,
    /// Current function context
    current_function: Option<FunctionId>,
    /// Current class context
    current_class: Option<String>,
}

impl TwoPassExtractor {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            phase_one_calls: Vec::new(),
            type_tracker: PythonTypeTracker::new(file_path.clone()),
            call_graph: CallGraph::new(),
            known_functions: HashSet::new(),
            current_function: None,
            current_class: None,
        }
    }

    /// Extract call graph in two passes
    pub fn extract(&mut self, module: &ast::Mod) -> CallGraph {
        // Phase 1: Build type information and collect calls
        self.phase_one(module);

        // Phase 2: Resolve calls using type information
        self.phase_two();

        self.call_graph.clone()
    }

    /// Phase 1: Build type information and collect unresolved calls
    fn phase_one(&mut self, module: &ast::Mod) {
        if let ast::Mod::Module(module) = module {
            for stmt in &module.body {
                self.analyze_stmt_phase_one(stmt);
            }
        }
    }

    /// Analyze statement in phase one
    fn analyze_stmt_phase_one(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::ClassDef(class_def) => {
                // Extract class information
                self.type_tracker.extract_class_info(class_def);

                let prev_class = self.current_class.clone();
                self.current_class = Some(class_def.name.to_string());

                // Analyze class body
                for stmt in &class_def.body {
                    self.analyze_stmt_phase_one(stmt);
                }

                self.current_class = prev_class;
            }
            ast::Stmt::FunctionDef(func_def) => {
                self.analyze_function_phase_one(func_def);
            }
            ast::Stmt::AsyncFunctionDef(func_def) => {
                self.analyze_async_function_phase_one(func_def);
            }
            ast::Stmt::Assign(assign) => {
                // Track assignments for type inference
                for target in &assign.targets {
                    self.type_tracker.track_assignment(target, &assign.value);
                }
            }
            ast::Stmt::AnnAssign(ann_assign) => {
                // Track annotated assignments
                if let Some(value) = &ann_assign.value {
                    self.type_tracker
                        .track_assignment(&ann_assign.target, value);
                }
            }
            _ => {}
        }
    }

    /// Analyze function in phase one
    fn analyze_function_phase_one(&mut self, func_def: &ast::StmtFunctionDef) {
        let func_name = if let Some(class_name) = &self.current_class {
            format!("{}.{}", class_name, func_def.name)
        } else {
            func_def.name.to_string()
        };

        let func_id = FunctionId {
            name: func_name.clone(),
            file: self.type_tracker.file_path.clone(),
            line: 0, // Would need source mapping
        };

        // Register function with default metrics
        self.call_graph.add_function(
            func_id.clone(),
            false,               // is_entry_point - could check for main() or __main__
            false,               // is_test - could check for test_ prefix
            10,                  // default complexity
            func_def.body.len(), // line count approximation
        );

        // Track function for phase two resolution
        self.known_functions.insert(func_id.clone());

        let prev_function = self.current_function.clone();
        self.current_function = Some(func_id.clone());

        // Enter new scope for function
        self.type_tracker.push_scope();

        // Track parameter types if annotated
        for arg in &func_def.args.args {
            if let Some(annotation) = &arg.def.annotation {
                let param_type = self.type_tracker.infer_type_from_annotation(annotation);
                self.type_tracker
                    .current_scope_mut()
                    .insert(arg.def.arg.to_string(), param_type);
            }
            // Special handling for 'self' parameter
            if arg.def.arg.as_str() == "self" {
                if let Some(class_name) = &self.current_class {
                    self.type_tracker
                        .current_scope_mut()
                        .insert("self".to_string(), PythonType::Instance(class_name.clone()));
                }
            }
        }

        // Analyze function body
        for stmt in &func_def.body {
            self.analyze_stmt_in_function(stmt);
        }

        // Exit scope
        self.type_tracker.pop_scope();
        self.current_function = prev_function;
    }

    /// Analyze async function in phase one
    fn analyze_async_function_phase_one(&mut self, func_def: &ast::StmtAsyncFunctionDef) {
        // Similar to regular function
        let func_name = if let Some(class_name) = &self.current_class {
            format!("{}.{}", class_name, func_def.name)
        } else {
            func_def.name.to_string()
        };

        let func_id = FunctionId {
            name: func_name.clone(),
            file: self.type_tracker.file_path.clone(),
            line: 0,
        };

        self.call_graph
            .add_function(func_id.clone(), false, false, 10, func_def.body.len());

        // Track function for phase two resolution
        self.known_functions.insert(func_id.clone());

        let prev_function = self.current_function.clone();
        self.current_function = Some(func_id.clone());

        self.type_tracker.push_scope();

        for arg in &func_def.args.args {
            if arg.def.arg.as_str() == "self" {
                if let Some(class_name) = &self.current_class {
                    self.type_tracker
                        .current_scope_mut()
                        .insert("self".to_string(), PythonType::Instance(class_name.clone()));
                }
            }
        }

        for stmt in &func_def.body {
            self.analyze_stmt_in_function(stmt);
        }

        self.type_tracker.pop_scope();
        self.current_function = prev_function;
    }

    /// Analyze statement within a function
    fn analyze_stmt_in_function(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::Expr(expr_stmt) => {
                self.analyze_expr_for_calls(&expr_stmt.value);
            }
            ast::Stmt::Assign(assign) => {
                self.type_tracker
                    .track_assignment(&assign.targets[0], &assign.value);
                self.analyze_expr_for_calls(&assign.value);
            }
            ast::Stmt::AnnAssign(ann_assign) => {
                if let Some(value) = &ann_assign.value {
                    self.type_tracker
                        .track_assignment(&ann_assign.target, value);
                    self.analyze_expr_for_calls(value);
                }
            }
            ast::Stmt::Return(ret_stmt) => {
                if let Some(value) = &ret_stmt.value {
                    self.analyze_expr_for_calls(value);
                }
            }
            ast::Stmt::If(if_stmt) => {
                self.analyze_expr_for_calls(&if_stmt.test);
                for stmt in &if_stmt.body {
                    self.analyze_stmt_in_function(stmt);
                }
                for stmt in &if_stmt.orelse {
                    self.analyze_stmt_in_function(stmt);
                }
            }
            ast::Stmt::While(while_stmt) => {
                self.analyze_expr_for_calls(&while_stmt.test);
                for stmt in &while_stmt.body {
                    self.analyze_stmt_in_function(stmt);
                }
            }
            ast::Stmt::For(for_stmt) => {
                self.analyze_expr_for_calls(&for_stmt.iter);
                for stmt in &for_stmt.body {
                    self.analyze_stmt_in_function(stmt);
                }
            }
            _ => {}
        }
    }

    /// Analyze expression for function/method calls
    fn analyze_expr_for_calls(&mut self, expr: &ast::Expr) {
        match expr {
            ast::Expr::Call(call) => {
                // Collect unresolved call
                if let Some(caller) = &self.current_function {
                    let unresolved = self.create_unresolved_call(caller.clone(), call);
                    self.phase_one_calls.push(unresolved);
                }

                // Recursively analyze arguments
                for arg in &call.args {
                    self.analyze_expr_for_calls(arg);
                }
            }
            ast::Expr::BinOp(binop) => {
                self.analyze_expr_for_calls(&binop.left);
                self.analyze_expr_for_calls(&binop.right);
            }
            ast::Expr::UnaryOp(unaryop) => {
                self.analyze_expr_for_calls(&unaryop.operand);
            }
            ast::Expr::Lambda(lambda) => {
                // Lambda body is an expression
                self.analyze_expr_for_calls(&lambda.body);
            }
            ast::Expr::ListComp(comp) => {
                self.analyze_expr_for_calls(&comp.elt);
                for generator in &comp.generators {
                    self.analyze_expr_for_calls(&generator.iter);
                }
            }
            _ => {}
        }
    }

    /// Create unresolved call for phase two resolution
    fn create_unresolved_call(&self, caller: FunctionId, call: &ast::ExprCall) -> UnresolvedCall {
        match &*call.func {
            ast::Expr::Attribute(attr) => {
                let receiver_type = self.type_tracker.infer_type(&attr.value);
                UnresolvedCall {
                    caller,
                    call_expr: ast::Expr::Call(call.clone()),
                    receiver_type: Some(receiver_type),
                    method_name: Some(attr.attr.to_string()),
                    call_type: CallType::Direct,
                }
            }
            _ => UnresolvedCall {
                caller,
                call_expr: ast::Expr::Call(call.clone()),
                receiver_type: None,
                method_name: None,
                call_type: CallType::Direct,
            },
        }
    }

    /// Phase 2: Resolve calls using type information
    fn phase_two(&mut self) {
        for unresolved in &self.phase_one_calls {
            if let Some(callee) = self.resolve_call(unresolved) {
                self.call_graph.add_call(FunctionCall {
                    caller: unresolved.caller.clone(),
                    callee,
                    call_type: unresolved.call_type.clone(),
                });
            }
        }
    }

    /// Resolve a call using type information
    fn resolve_call(&self, unresolved: &UnresolvedCall) -> Option<FunctionId> {
        if unresolved.method_name.is_some() {
            if let (Some(receiver_type), Some(method_name)) =
                (&unresolved.receiver_type, &unresolved.method_name)
            {
                return self
                    .type_tracker
                    .resolve_method_call(receiver_type, method_name);
            }
        } else {
            // Function call resolution
            if let ast::Expr::Call(call) = &unresolved.call_expr {
                if let ast::Expr::Name(name) = &*call.func {
                    // Check if it's a known function
                    let func_id = FunctionId {
                        name: name.id.to_string(),
                        file: self.type_tracker.file_path.clone(),
                        line: 0,
                    };
                    // Check if function exists in known_functions set
                    if self.known_functions.contains(&func_id) {
                        return Some(func_id);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_inference_literals() {
        let tracker = PythonTypeTracker::new(PathBuf::from("test.py"));

        // Test integer literal
        let int_expr = ast::Expr::Constant(ast::ExprConstant {
            value: ast::Constant::Int(42.into()),
            kind: None,
            range: Default::default(),
        });
        assert_eq!(
            tracker.infer_type(&int_expr),
            PythonType::BuiltIn("int".to_string())
        );

        // Test string literal
        let str_expr = ast::Expr::Constant(ast::ExprConstant {
            value: ast::Constant::Str("test".to_string()),
            kind: None,
            range: Default::default(),
        });
        assert_eq!(
            tracker.infer_type(&str_expr),
            PythonType::BuiltIn("str".to_string())
        );
    }

    #[test]
    fn test_class_hierarchy() {
        let mut tracker = PythonTypeTracker::new(PathBuf::from("test.py"));

        // Create a simple class
        let class_def = ast::StmtClassDef {
            name: "TestClass".to_string().into(),
            bases: vec![],
            keywords: vec![],
            body: vec![],
            decorator_list: vec![],
            type_params: vec![],
            range: Default::default(),
        };

        tracker.extract_class_info(&class_def);

        assert!(tracker.class_hierarchy.contains_key("TestClass"));
        let class_info = tracker.class_hierarchy.get("TestClass").unwrap();
        assert_eq!(class_info.name, "TestClass");
        assert!(class_info.bases.is_empty());
    }

    #[test]
    fn test_method_resolution() {
        let mut tracker = PythonTypeTracker::new(PathBuf::from("test.py"));

        // Create a class with a method
        let method_func = ast::StmtFunctionDef {
            name: "test_method".to_string().into(),
            args: Box::new(ast::Arguments {
                posonlyargs: vec![],
                args: vec![],
                kwonlyargs: vec![],
                kwarg: None,
                vararg: None,
                range: Default::default(),
            }),
            body: vec![],
            decorator_list: vec![],
            returns: None,
            type_comment: None,
            type_params: vec![],
            range: Default::default(),
        };

        let class_def = ast::StmtClassDef {
            name: "TestClass".to_string().into(),
            bases: vec![],
            keywords: vec![],
            body: vec![ast::Stmt::FunctionDef(method_func)],
            decorator_list: vec![],
            type_params: vec![],
            range: Default::default(),
        };

        tracker.extract_class_info(&class_def);

        // Test method resolution
        let instance_type = PythonType::Instance("TestClass".to_string());
        let resolved = tracker.resolve_method_call(&instance_type, "test_method");
        assert!(resolved.is_some());
    }

    #[test]
    fn test_scope_management() {
        let mut tracker = PythonTypeTracker::new(PathBuf::from("test.py"));

        // Add variable to current scope
        tracker
            .current_scope_mut()
            .insert("x".to_string(), PythonType::BuiltIn("int".to_string()));

        // Push new scope
        tracker.push_scope();

        // Variable should still be accessible from parent scope
        let name_expr = ast::Expr::Name(ast::ExprName {
            id: "x".to_string().into(),
            ctx: ast::ExprContext::Load,
            range: Default::default(),
        });
        assert_eq!(
            tracker.infer_type(&name_expr),
            PythonType::BuiltIn("int".to_string())
        );

        // Add shadowing variable in child scope
        tracker
            .current_scope_mut()
            .insert("x".to_string(), PythonType::BuiltIn("str".to_string()));
        assert_eq!(
            tracker.infer_type(&name_expr),
            PythonType::BuiltIn("str".to_string())
        );

        // Pop scope - should restore parent value
        tracker.pop_scope();
        assert_eq!(
            tracker.infer_type(&name_expr),
            PythonType::BuiltIn("int".to_string())
        );
    }

    #[test]
    fn test_simple_function_call_extraction() {
        let code = r#"
def helper():
    print("Helper")

def main():
    helper()
"#;

        let module =
            rustpython_parser::parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let mut extractor = TwoPassExtractor::new(PathBuf::from("test.py"));
        let call_graph = extractor.extract(&module);

        // Check that functions are registered
        let main_id = FunctionId {
            name: "main".to_string(),
            file: PathBuf::from("test.py"),
            line: 0,
        };
        let helper_id = FunctionId {
            name: "helper".to_string(),
            file: PathBuf::from("test.py"),
            line: 0,
        };

        assert!(call_graph.get_function_info(&main_id).is_some());
        assert!(call_graph.get_function_info(&helper_id).is_some());

        // Check that the call from main to helper is tracked
        let callees = call_graph.get_callees(&main_id);
        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].name, "helper");
    }

    #[test]
    fn test_method_call_extraction() {
        let code = r#"
class Calculator:
    def __init__(self):
        self.value = 0
        self.reset()

    def reset(self):
        self.value = 0

    def add(self, x):
        self.value += x
        self.log("add")

    def log(self, msg):
        print(msg)
"#;

        let module =
            rustpython_parser::parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let mut extractor = TwoPassExtractor::new(PathBuf::from("test.py"));
        let call_graph = extractor.extract(&module);

        // Check that __init__ calls reset
        let init_id = FunctionId {
            name: "Calculator.__init__".to_string(),
            file: PathBuf::from("test.py"),
            line: 0,
        };

        let init_callees = call_graph.get_callees(&init_id);
        assert!(init_callees.iter().any(|f| f.name == "Calculator.reset"));

        // Check that add calls log
        let add_id = FunctionId {
            name: "Calculator.add".to_string(),
            file: PathBuf::from("test.py"),
            line: 0,
        };
        let add_callees = call_graph.get_callees(&add_id);
        assert!(add_callees.iter().any(|f| f.name == "Calculator.log"));
    }

    #[test]
    fn test_known_functions_tracking() {
        let code = r#"
def func_a():
    pass

def func_b():
    func_a()

def func_c():
    func_b()
    func_a()
"#;

        let module =
            rustpython_parser::parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let mut extractor = TwoPassExtractor::new(PathBuf::from("test.py"));

        // After phase one, known_functions should contain all three functions
        if let ast::Mod::Module(module) = &module {
            for stmt in &module.body {
                extractor.analyze_stmt_phase_one(stmt);
            }
        }

        assert_eq!(extractor.known_functions.len(), 3);

        // Verify all functions are tracked
        let func_a = FunctionId {
            name: "func_a".to_string(),
            file: PathBuf::from("test.py"),
            line: 0,
        };
        let func_b = FunctionId {
            name: "func_b".to_string(),
            file: PathBuf::from("test.py"),
            line: 0,
        };
        let func_c = FunctionId {
            name: "func_c".to_string(),
            file: PathBuf::from("test.py"),
            line: 0,
        };

        assert!(extractor.known_functions.contains(&func_a));
        assert!(extractor.known_functions.contains(&func_b));
        assert!(extractor.known_functions.contains(&func_c));
    }
}
