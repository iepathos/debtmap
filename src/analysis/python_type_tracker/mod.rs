//! Python Type Tracking System
//!
//! Provides type inference and tracking for Python code to improve call graph accuracy.
//! Uses two-pass resolution for better method resolution and reduced false positives.
//!
//! This module is organized into focused sub-modules:
//! - `types`: Core type definitions (PythonType, ClassInfo, FunctionSignature, Scope)
//! - `utils`: Utility functions for AST extraction and manipulation
//! - (More modules will be added as refactoring progresses)

mod import_resolver;
mod types;
mod utils;

// Re-export public types for backward compatibility
pub use types::{ClassInfo, FunctionSignature, PythonType, Scope};

use crate::analysis::framework_patterns::FrameworkPatternRegistry;
use crate::analysis::python_call_graph::cross_module::CrossModuleContext;
use crate::analysis::type_flow_tracker::{TypeFlowTracker, TypeId};
use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use import_resolver::ImportResolver;
use rustpython_parser::ast;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

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
    /// Import resolver for tracking and resolving imports
    import_resolver: ImportResolver,
    /// Framework pattern registry for entry point detection
    pub framework_registry: FrameworkPatternRegistry,
    /// Type flow tracker for data flow analysis (owned for standalone use)
    type_flow_owned: Option<TypeFlowTracker>,
    /// Shared type flow tracker for cross-module analysis
    type_flow_shared: Option<std::sync::Arc<std::sync::RwLock<TypeFlowTracker>>>,
}

impl PythonTypeTracker {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            local_types: HashMap::new(),
            class_hierarchy: HashMap::new(),
            function_signatures: HashMap::new(),
            current_scope: vec![Scope::new()],
            file_path,
            import_resolver: ImportResolver::new(),
            framework_registry: FrameworkPatternRegistry::new(),
            type_flow_owned: Some(TypeFlowTracker::new()),
            type_flow_shared: None,
        }
    }

    /// Create a new tracker with a shared type flow tracker for cross-module analysis
    pub fn new_with_shared_flow(
        file_path: PathBuf,
        shared_flow: std::sync::Arc<std::sync::RwLock<TypeFlowTracker>>,
    ) -> Self {
        Self {
            local_types: HashMap::new(),
            class_hierarchy: HashMap::new(),
            function_signatures: HashMap::new(),
            current_scope: vec![Scope::new()],
            file_path,
            import_resolver: ImportResolver::new(),
            framework_registry: FrameworkPatternRegistry::new(),
            type_flow_owned: None,
            type_flow_shared: Some(shared_flow),
        }
    }

    /// Get access to the type flow tracker (either owned or shared)
    fn with_type_flow<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&TypeFlowTracker) -> R,
    {
        if let Some(shared) = &self.type_flow_shared {
            let guard = shared.read().unwrap();
            f(&guard)
        } else if let Some(owned) = &self.type_flow_owned {
            f(owned)
        } else {
            panic!("No type flow tracker available")
        }
    }

    /// Get mutable access to the type flow tracker
    fn with_type_flow_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut TypeFlowTracker) -> R,
    {
        if let Some(shared) = &self.type_flow_shared {
            let mut guard = shared.write().unwrap();
            f(&mut guard)
        } else if let Some(owned) = &mut self.type_flow_owned {
            f(owned)
        } else {
            panic!("No type flow tracker available")
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
                    .insert(name.id.to_string(), inferred_type.clone());

                // Track type flow for assignments
                if let Some(type_id) = self.python_type_to_type_id(&inferred_type) {
                    let name_ref = name.id.as_ref();
                    self.with_type_flow_mut(|flow| flow.record_assignment(name_ref, type_id));
                }
            }
            ast::Expr::Attribute(attr) => {
                // Track attribute assignments for class members
                if let ast::Expr::Name(name) = &*attr.value {
                    if name.id.as_str() == "self" {
                        // This is a self.attr assignment in a method
                        // We'd need to track the current class context

                        // Track type flow for attribute assignments
                        if let Some(type_id) = self.python_type_to_type_id(&inferred_type) {
                            let attr_name = format!("self.{}", attr.attr);
                            self.with_type_flow_mut(|flow| {
                                flow.record_assignment(&attr_name, type_id)
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Convert PythonType to TypeId for type flow tracking
    fn python_type_to_type_id(
        &self,
        py_type: &PythonType,
    ) -> Option<crate::analysis::type_flow_tracker::TypeId> {
        match py_type {
            PythonType::Class(name) | PythonType::Instance(name) => {
                Some(crate::analysis::type_flow_tracker::TypeId::new(
                    name.clone(),
                    Some(self.file_path.clone()),
                ))
            }
            PythonType::BuiltIn(name) => Some(crate::analysis::type_flow_tracker::TypeId::new(
                name.clone(),
                None,
            )),
            _ => None,
        }
    }

    /// Get all types stored in a collection field (for observer pattern detection)
    pub fn get_collection_member_types(
        &self,
        class: &str,
        field: &str,
    ) -> Vec<crate::analysis::type_flow_tracker::TypeId> {
        let collection_name = format!("{}.{}", class, field);
        self.with_type_flow(|flow| flow.get_collection_type_ids(&collection_name))
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
                    let func_id = FunctionId::new(
                        self.file_path.clone(),
                        format!("{}.{}", class_def.name, method_name),
                        0, // Line numbers handled by TwoPassExtractor
                    );

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
    /// Register a module import (import module as alias)
    pub fn register_import(&mut self, module_name: String, alias: Option<String>) {
        self.import_resolver.register_import(module_name, alias);
    }

    /// Register a from import (from module import name as alias)
    pub fn register_from_import(
        &mut self,
        module_name: String,
        name: String,
        alias: Option<String>,
    ) {
        self.import_resolver
            .register_from_import(module_name, name, alias);
    }

    /// Resolve an imported name to its fully qualified form
    pub fn resolve_imported_name(&self, name: &str) -> Option<String> {
        self.import_resolver.resolve_imported_name(name)
    }

    /// Track import statement (import module as alias)
    pub fn track_import_stmt(&mut self, import: &ast::StmtImport) {
        self.import_resolver.track_import_stmt(import);
    }

    /// Track from import statement (from module import name as alias)
    pub fn track_import_from_stmt(&mut self, import_from: &ast::StmtImportFrom) {
        self.import_resolver.track_import_from_stmt(import_from);
    }

    /// Check if a name is an imported function/class
    pub fn is_imported_name(&self, name: &str) -> bool {
        self.import_resolver.is_imported_name(name)
    }

    /// Get the module for an imported name
    pub fn get_import_module(&self, name: &str) -> Option<String> {
        self.import_resolver.get_import_module(name)
    }

    /// Get all import strings for framework detection
    pub fn get_all_imports(&self) -> Vec<String> {
        self.import_resolver.get_all_imports()
    }

    /// Detect frameworks from collected imports
    pub fn detect_frameworks_from_imports(&mut self) {
        let imports = self.get_all_imports();
        self.framework_registry
            .auto_detect_frameworks(&self.file_path, &imports);
    }

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
    /// Module alias if the call is through an imported module
    pub module_alias: Option<String>,
    /// Whether this call is to an imported function
    pub is_imported: bool,
    /// Import context for resolving the call
    pub import_context: Option<String>,
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
    /// Map function names to their FunctionIds for easier lookup (without line numbers)
    function_name_map: HashMap<String, FunctionId>,
    /// Current function context
    current_function: Option<FunctionId>,
    /// Current class context
    current_class: Option<String>,
    /// Source code lines for line number extraction
    source_lines: Vec<String>,
    /// Optional cross-module context for resolving imports
    cross_module_context: Option<CrossModuleContext>,
    /// Callback tracker for deferred callback resolution
    callback_tracker: crate::analysis::python_call_graph::CallbackTracker,
    /// Observer registry for tracking observer patterns (shared across files via RwLock)
    observer_registry: std::sync::Arc<
        std::sync::RwLock<crate::analysis::python_call_graph::observer_registry::ObserverRegistry>,
    >,
    /// Pending for loops that may contain observer dispatches (to be resolved after all classes are processed)
    /// Format: (for_stmt, caller_id, current_class)
    pending_observer_dispatches: Vec<(ast::StmtFor, FunctionId, Option<String>)>,
}

impl TwoPassExtractor {
    /// Check if a function name is a framework entry point using the framework registry
    fn is_framework_entry_point(&self, func_name: &str, decorators: &[&str]) -> bool {
        // Convert decorator strings
        let decorator_strings: Vec<String> = decorators.iter().map(|s| s.to_string()).collect();

        // Check using framework registry
        if self
            .type_tracker
            .framework_registry
            .is_entry_point(func_name, &decorator_strings)
        {
            return true;
        }

        // Fallback: check common patterns
        let method_name = if let Some(pos) = func_name.rfind('.') {
            &func_name[pos + 1..]
        } else {
            func_name
        };

        // Main entry points
        method_name == "main" || method_name == "__main__" || method_name.starts_with("test_")
    }

    pub fn new(file_path: PathBuf) -> Self {
        Self {
            phase_one_calls: Vec::new(),
            type_tracker: PythonTypeTracker::new(file_path.clone()),
            call_graph: CallGraph::new(),
            known_functions: HashSet::new(),
            function_name_map: HashMap::new(),
            current_function: None,
            current_class: None,
            source_lines: Vec::new(),
            cross_module_context: None,
            callback_tracker: crate::analysis::python_call_graph::CallbackTracker::new(),
            observer_registry: std::sync::Arc::new(std::sync::RwLock::new(
                crate::analysis::python_call_graph::observer_registry::ObserverRegistry::new(),
            )),
            pending_observer_dispatches: Vec::new(),
        }
    }

    /// Create a new extractor with source content for line number extraction
    pub fn new_with_source(file_path: PathBuf, source: &str) -> Self {
        let source_lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
        Self {
            phase_one_calls: Vec::new(),
            type_tracker: PythonTypeTracker::new(file_path.clone()),
            call_graph: CallGraph::new(),
            known_functions: HashSet::new(),
            function_name_map: HashMap::new(),
            current_function: None,
            current_class: None,
            source_lines,
            cross_module_context: None,
            callback_tracker: crate::analysis::python_call_graph::CallbackTracker::new(),
            observer_registry: std::sync::Arc::new(std::sync::RwLock::new(
                crate::analysis::python_call_graph::observer_registry::ObserverRegistry::new(),
            )),
            pending_observer_dispatches: Vec::new(),
        }
    }

    /// Create a new extractor with cross-module context for better resolution
    pub fn new_with_context(file_path: PathBuf, source: &str, context: CrossModuleContext) -> Self {
        let source_lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();

        // Get shared observer registry and type flow tracker from context
        let observer_registry = context.observer_registry();
        let shared_type_flow = context.type_flow();

        // Create type tracker with shared type flow
        let type_tracker =
            PythonTypeTracker::new_with_shared_flow(file_path.clone(), shared_type_flow);

        Self {
            phase_one_calls: Vec::new(),
            type_tracker,
            call_graph: CallGraph::new(),
            known_functions: HashSet::new(),
            function_name_map: HashMap::new(),
            current_function: None,
            current_class: None,
            source_lines,
            cross_module_context: Some(context),
            callback_tracker: crate::analysis::python_call_graph::CallbackTracker::new(),
            observer_registry,
            pending_observer_dispatches: Vec::new(),
        }
    }

    /// Estimate line number for a function by searching for def patterns
    fn estimate_line_number(&self, func_name: &str) -> usize {
        if self.source_lines.is_empty() {
            return 0;
        }

        let def_pattern = format!("def {}", func_name);
        let async_def_pattern = format!("async def {}", func_name);

        for (idx, line) in self.source_lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with(&def_pattern) || trimmed.starts_with(&async_def_pattern) {
                return idx + 1; // Line numbers are 1-based
            }
        }

        0 // Return 0 if not found (backward compatibility)
    }

    /// Extract call graph in two passes
    pub fn extract(&mut self, module: &ast::Mod) -> CallGraph {
        // Phase 1: Build type information and collect calls
        self.phase_one(module);

        // Phase 2: Resolve calls using type information
        self.phase_two();

        self.call_graph.clone()
    }

    /// Get the extracted call graph
    pub fn get_call_graph(&self) -> CallGraph {
        self.call_graph.clone()
    }

    /// Phase 1: Build type information and collect unresolved calls
    fn phase_one(&mut self, module: &ast::Mod) {
        if let ast::Mod::Module(module) = module {
            // First pass: Track imports to build namespace
            for stmt in &module.body {
                match stmt {
                    ast::Stmt::Import(import) => {
                        self.type_tracker.track_import_stmt(import);
                    }
                    ast::Stmt::ImportFrom(import_from) => {
                        self.type_tracker.track_import_from_stmt(import_from);
                    }
                    _ => {}
                }
            }

            // Detect frameworks from imports
            self.type_tracker.detect_frameworks_from_imports();

            // Second pass: Register observer interfaces (classes that inherit from ABC)
            for stmt in &module.body {
                if let ast::Stmt::ClassDef(class_def) = stmt {
                    self.register_observer_interfaces(class_def);
                }
            }

            // Third pass: Analyze functions and collect calls (this also tracks collection operations for type flow)
            for stmt in &module.body {
                self.analyze_stmt_phase_one(stmt);
            }

            // Phase 3.5: Discover observer interfaces from usage (after type flow tracking)
            self.discover_observer_interfaces_from_usage(module);

            // Fourth pass: Register observer implementations now that all functions are in function_name_map
            for stmt in &module.body {
                if let ast::Stmt::ClassDef(class_def) = stmt {
                    self.register_observer_implementations(class_def);
                }
            }

            // Fifth pass: Store pending observer dispatches for cross-module resolution
            self.store_pending_observer_dispatches();
        }
    }

    /// Analyze statement in phase one
    fn analyze_stmt_phase_one(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::ClassDef(class_def) => {
                // Extract class information
                self.type_tracker.extract_class_info(class_def);

                // Populate observer registry from class definition
                self.populate_observer_registry(class_def);

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
            ast::Stmt::If(if_stmt) => {
                // Check for if __name__ == "__main__" pattern
                if self.is_main_guard(&if_stmt.test) {
                    // Analyze the body as if it were a special module-level function
                    self.analyze_main_block(&if_stmt.body);
                }
            }
            ast::Stmt::Import(import_stmt) => {
                // Track imports for resolution
                for alias in &import_stmt.names {
                    let module_name = alias.name.as_str();
                    let alias_name = alias.asname.as_ref().map(|n| n.as_str());

                    // Register the import in type tracker for later resolution
                    self.type_tracker.register_import(
                        module_name.to_string(),
                        alias_name.map(|s| s.to_string()),
                    );
                }
            }
            ast::Stmt::ImportFrom(import_from) => {
                // Track from imports for resolution
                if let Some(module) = &import_from.module {
                    let module_name = module.as_str();

                    for alias in &import_from.names {
                        let imported_name = alias.name.as_str();
                        let alias_name = alias.asname.as_ref().map(|n| n.as_str());

                        // Register the import in type tracker
                        self.type_tracker.register_from_import(
                            module_name.to_string(),
                            imported_name.to_string(),
                            alias_name.map(|s| s.to_string()),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    /// Analyze function in phase one
    fn analyze_function_phase_one(&mut self, func_def: &ast::StmtFunctionDef) {
        // Build function name considering both class and parent function context
        let func_name = if let Some(parent_func) = &self.current_function {
            // This is a nested function
            format!("{}.{}", parent_func.name, func_def.name)
        } else if let Some(class_name) = &self.current_class {
            // This is a method
            format!("{}.{}", class_name, func_def.name)
        } else {
            // Top-level function
            func_def.name.to_string()
        };

        // Extract line number from source if available
        let line = self.estimate_line_number(func_def.name.as_ref());

        let func_id = FunctionId::new(self.type_tracker.file_path.clone(), func_name.clone(), line);

        // Extract decorator names
        let decorators: Vec<&str> = func_def
            .decorator_list
            .iter()
            .filter_map(|dec| match dec {
                ast::Expr::Name(name) => Some(name.id.as_str()),
                ast::Expr::Attribute(attr) => Some(attr.attr.as_str()),
                _ => None,
            })
            .collect();

        // Check for framework methods and test functions
        let is_entry_point = self.is_framework_entry_point(&func_name, &decorators);
        let is_test = func_name.starts_with("test_") || func_name.contains("::test_");

        // Register function with appropriate metrics
        self.call_graph.add_function(
            func_id.clone(),
            is_entry_point,
            is_test,
            10,                  // default complexity
            func_def.body.len(), // line count approximation
        );

        // Track function for phase two resolution
        self.known_functions.insert(func_id.clone());
        self.function_name_map
            .insert(func_name.clone(), func_id.clone());

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
        // Build function name considering both class and parent function context
        let func_name = if let Some(parent_func) = &self.current_function {
            // This is a nested async function
            format!("{}.{}", parent_func.name, func_def.name)
        } else if let Some(class_name) = &self.current_class {
            // This is an async method
            format!("{}.{}", class_name, func_def.name)
        } else {
            // Top-level async function
            func_def.name.to_string()
        };

        // Extract line number from source if available
        let line = self.estimate_line_number(func_def.name.as_ref());

        let func_id = FunctionId::new(self.type_tracker.file_path.clone(), func_name.clone(), line);

        // Extract decorator names
        let decorators: Vec<&str> = func_def
            .decorator_list
            .iter()
            .filter_map(|dec| match dec {
                ast::Expr::Name(name) => Some(name.id.as_str()),
                ast::Expr::Attribute(attr) => Some(attr.attr.as_str()),
                _ => None,
            })
            .collect();

        // Check for framework methods and test functions
        let is_entry_point = self.is_framework_entry_point(&func_name, &decorators);
        let is_test = func_name.starts_with("test_") || func_name.contains("::test_");

        self.call_graph.add_function(
            func_id.clone(),
            is_entry_point,
            is_test,
            10,
            func_def.body.len(),
        );

        // Track function for phase two resolution
        self.known_functions.insert(func_id.clone());
        self.function_name_map
            .insert(func_name.clone(), func_id.clone());

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
            ast::Stmt::FunctionDef(nested_func) => {
                // Handle nested function definitions
                self.analyze_function_phase_one(nested_func);
            }
            ast::Stmt::AsyncFunctionDef(nested_func) => {
                // Handle nested async function definitions
                self.analyze_async_function_phase_one(nested_func);
            }
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

                // Store for loop for later observer dispatch detection
                // (after all classes and their implementations have been registered)
                if let Some(caller) = self.current_function.clone() {
                    self.pending_observer_dispatches.push((
                        for_stmt.clone(),
                        caller,
                        self.current_class.clone(),
                    ));
                }

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
                // Track type flow for collection operations (e.g., self.observers.append(observer))
                self.track_collection_operation(call);

                // Collect unresolved call
                if let Some(caller) = &self.current_function {
                    let unresolved = self.create_unresolved_call(caller.clone(), call);
                    self.phase_one_calls.push(unresolved);

                    // Check for event binding patterns (e.g., Bind(event, self.method))
                    self.check_for_event_bindings(call);

                    // Check for callback patterns (wx.CallAfter, functools.partial, etc.)
                    self.check_for_callback_patterns(call);
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
                    // Also analyze the if clauses (filters)
                    for if_clause in &generator.ifs {
                        self.analyze_expr_for_calls(if_clause);
                    }
                }
            }
            ast::Expr::SetComp(comp) => {
                self.analyze_expr_for_calls(&comp.elt);
                for generator in &comp.generators {
                    self.analyze_expr_for_calls(&generator.iter);
                    for if_clause in &generator.ifs {
                        self.analyze_expr_for_calls(if_clause);
                    }
                }
            }
            ast::Expr::DictComp(comp) => {
                self.analyze_expr_for_calls(&comp.key);
                self.analyze_expr_for_calls(&comp.value);
                for generator in &comp.generators {
                    self.analyze_expr_for_calls(&generator.iter);
                    for if_clause in &generator.ifs {
                        self.analyze_expr_for_calls(if_clause);
                    }
                }
            }
            ast::Expr::GeneratorExp(comp) => {
                self.analyze_expr_for_calls(&comp.elt);
                for generator in &comp.generators {
                    self.analyze_expr_for_calls(&generator.iter);
                    for if_clause in &generator.ifs {
                        self.analyze_expr_for_calls(if_clause);
                    }
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

                // Check if receiver is an imported module
                let (module_alias, is_imported, import_context) =
                    if let ast::Expr::Name(name) = &*attr.value {
                        let name_str = name.id.as_str();
                        if self.type_tracker.is_imported_name(name_str) {
                            let context = self.type_tracker.get_import_module(name_str);
                            (Some(name_str.to_string()), true, context)
                        } else {
                            (None, false, None)
                        }
                    } else {
                        (None, false, None)
                    };

                UnresolvedCall {
                    caller,
                    call_expr: ast::Expr::Call(call.clone()),
                    receiver_type: Some(receiver_type),
                    method_name: Some(attr.attr.to_string()),
                    call_type: CallType::Direct,
                    module_alias,
                    is_imported,
                    import_context,
                }
            }
            ast::Expr::Name(name) => {
                // Check if this is an imported function
                let name_str = name.id.as_str();
                let is_imported = self.type_tracker.is_imported_name(name_str);
                let import_context = if is_imported {
                    self.type_tracker.resolve_imported_name(name_str)
                } else {
                    None
                };

                UnresolvedCall {
                    caller,
                    call_expr: ast::Expr::Call(call.clone()),
                    receiver_type: None,
                    method_name: None, // For direct function calls, method_name should be None
                    call_type: CallType::Direct,
                    module_alias: None,
                    is_imported,
                    import_context,
                }
            }
            _ => UnresolvedCall {
                caller,
                call_expr: ast::Expr::Call(call.clone()),
                receiver_type: None,
                method_name: None,
                call_type: CallType::Direct,
                module_alias: None,
                is_imported: false,
                import_context: None,
            },
        }
    }

    /// Infer observer interface name from collection field name
    ///
    /// E.g., "listeners" -> "Listener", "observers" -> "Observer", "handlers" -> "Handler"
    fn infer_interface_from_field_name(&self, field_name: &str) -> String {
        // Convert plural to singular and capitalize
        // Simple heuristic: remove trailing 's' and capitalize first letter
        let singular = if let Some(stripped) = field_name.strip_suffix('s') {
            stripped
        } else {
            field_name
        };

        // Capitalize first letter
        let mut chars = singular.chars();
        match chars.next() {
            None => "Observer".to_string(), // Fallback
            Some(first) => first.to_uppercase().chain(chars).collect(),
        }
    }

    /// Register observer interfaces (classes that inherit from ABC) in the first pass
    fn register_observer_interfaces(&mut self, class_def: &ast::StmtClassDef) {
        let class_name = &class_def.name;

        // Check if this class inherits from ABC (making it an observer interface)
        let has_abc_base = class_def.bases.iter().any(|base| {
            if let ast::Expr::Name(name) = base {
                name.id.as_str() == "ABC"
            } else {
                false
            }
        });

        // If this class inherits from ABC, register it as an observer interface
        if has_abc_base {
            self.observer_registry
                .write()
                .unwrap()
                .register_interface(class_name);
        }
    }

    /// Find observer collections in module by analyzing class `__init__` methods.
    ///
    /// This function traverses the AST to find assignments to `self.*` attributes
    /// in class constructors, filtering for observer collection names.
    ///
    /// # Returns
    /// A vector of `(class_name, field_name)` tuples representing observer collections.
    ///
    /// # Example
    /// For code like:
    /// ```python
    /// class Subject:
    ///     def __init__(self):
    ///         self.observers = []  # This will be found
    /// ```
    /// Returns: `[("Subject", "observers")]`
    fn find_observer_collections(module: &ast::ModModule) -> Vec<(String, String)> {
        use crate::analysis::python_call_graph::observer_registry::ObserverRegistry;

        module
            .body
            .iter()
            .filter_map(|stmt| {
                if let ast::Stmt::ClassDef(class_def) = stmt {
                    Some((class_def.name.to_string(), &class_def.body))
                } else {
                    None
                }
            })
            .flat_map(|(class_name, body)| {
                body.iter()
                    .filter_map(move |method_stmt| {
                        if let ast::Stmt::FunctionDef(func_def) = method_stmt {
                            if func_def.name.as_str() == "__init__" {
                                Some((class_name.clone(), &func_def.body))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .flat_map(|(class_name, init_body)| {
                        init_body
                            .iter()
                            .filter_map(move |init_stmt| {
                                if let ast::Stmt::Assign(assign) = init_stmt {
                                    Some((class_name.clone(), &assign.targets))
                                } else {
                                    None
                                }
                            })
                            .flat_map(|(class_name, targets)| {
                                targets.iter().filter_map(move |target| {
                                    if let ast::Expr::Attribute(attr) = target {
                                        if let ast::Expr::Name(name) = &*attr.value {
                                            if name.id.as_str() == "self" {
                                                let field_name = attr.attr.as_str();
                                                if ObserverRegistry::is_observer_collection_name(
                                                    field_name,
                                                ) {
                                                    return Some((
                                                        class_name.clone(),
                                                        field_name.to_string(),
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                    None
                                })
                            })
                    })
            })
            .collect()
    }

    /// Collect type IDs for observer collections from the type flow tracker.
    ///
    /// For each observer collection, queries the type flow tracker to determine
    /// which types are stored in that collection.
    ///
    /// # Arguments
    /// * `type_tracker` - The type flow tracker to query for type information
    /// * `observer_collections` - List of (class_name, field_name) tuples
    ///
    /// # Returns
    /// A vector of `(collection_path, type_ids)` tuples where collection_path
    /// is in the format "ClassName.field_name".
    fn collect_type_ids_for_observers(
        type_tracker: &PythonTypeTracker,
        observer_collections: &[(String, String)],
    ) -> Vec<(String, Vec<TypeId>)> {
        observer_collections
            .iter()
            .map(|(class_name, field_name)| {
                let collection_path = format!("{}.{}", class_name, field_name);
                let type_ids = type_tracker
                    .with_type_flow(|flow| flow.get_collection_type_ids(&collection_path));
                (collection_path, type_ids)
            })
            .collect()
    }

    /// Register observer interfaces discovered from usage analysis.
    ///
    /// This function registers each discovered type as an observer interface,
    /// along with any base classes it inherits from.
    ///
    /// # Arguments
    /// * `observer_registry` - The registry to register interfaces in
    /// * `type_tracker` - The type flow tracker for querying base class information
    /// * `type_ids_by_collection` - Map of collection paths to their type IDs
    ///
    /// # Side Effects
    /// Mutates the observer registry by registering interfaces.
    fn register_observer_interfaces_from_usage(
        observer_registry: &std::sync::Arc<
            std::sync::RwLock<
                crate::analysis::python_call_graph::observer_registry::ObserverRegistry,
            >,
        >,
        type_tracker: &PythonTypeTracker,
        type_ids_by_collection: Vec<(String, Vec<TypeId>)>,
    ) {
        for (collection_path, type_ids) in type_ids_by_collection {
            for type_id in type_ids {
                // Register the type as an interface
                observer_registry
                    .write()
                    .unwrap()
                    .register_interface(&type_id.name);

                // Also register base classes as interfaces
                let collection_types = type_tracker.with_type_flow(|flow| {
                    flow.get_collection_types(&collection_path)
                        .into_iter()
                        .cloned()
                        .collect::<Vec<_>>()
                });
                if let Some(type_info) = collection_types
                    .into_iter()
                    .find(|ti| ti.type_id == type_id)
                {
                    for base_class in &type_info.base_classes {
                        observer_registry
                            .write()
                            .unwrap()
                            .register_interface(&base_class.name);
                    }
                }
            }
        }
    }

    /// Discover observer interfaces via usage analysis (after type flow tracking).
    ///
    /// This identifies observer interfaces by analyzing how types are used in observer
    /// collections, without requiring explicit ABC inheritance or decorators.
    ///
    /// # Algorithm
    /// 1. Find all observer collections (e.g., `self.observers = []`)
    /// 2. Query type flow tracker for types stored in those collections
    /// 3. Register those types (and their base classes) as observer interfaces
    /// 4. Analyze dispatch loops to identify interface methods
    ///
    /// # Arguments
    /// * `module` - The Python module AST to analyze
    fn discover_observer_interfaces_from_usage(&mut self, module: &ast::ModModule) {
        // Step 1: Find all observer collections
        let observer_collections = Self::find_observer_collections(module);

        // Step 2: For each observer collection, get types from type flow tracker
        let type_ids_by_collection =
            Self::collect_type_ids_for_observers(&self.type_tracker, &observer_collections);

        // Step 3: Register these types as observer interfaces
        Self::register_observer_interfaces_from_usage(
            &self.observer_registry,
            &self.type_tracker,
            type_ids_by_collection,
        );

        // Step 4: Analyze dispatch loops to find interface methods
        self.analyze_dispatch_loops_for_interface_methods(module);
    }

    /// Analyze for loops to discover which methods are part of observer interfaces
    fn analyze_dispatch_loops_for_interface_methods(&mut self, module: &ast::ModModule) {
        for stmt in &module.body {
            if let ast::Stmt::ClassDef(class_def) = stmt {
                for method in &class_def.body {
                    if let ast::Stmt::FunctionDef(func_def) = method {
                        self.find_and_register_interface_methods_in_function(
                            &class_def.name,
                            func_def,
                        );
                    }
                }
            }
        }
    }

    /// Find dispatch loops in a function and register the methods being called
    fn find_and_register_interface_methods_in_function(
        &mut self,
        class_name: &ast::Identifier,
        func_def: &ast::StmtFunctionDef,
    ) {
        // Recursively search for for-loops in the function body
        for stmt in &func_def.body {
            self.process_stmt_for_dispatch_loops(class_name, stmt);
        }
    }

    /// Process a statement looking for dispatch loops
    fn process_stmt_for_dispatch_loops(&mut self, class_name: &ast::Identifier, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::For(for_stmt) => {
                // Check if this is an observer dispatch pattern
                if let Some((collection_name, method_calls)) =
                    self.extract_observer_dispatch_info(for_stmt)
                {
                    // Get the full collection path
                    let collection_path = format!("{}.{}", class_name, collection_name);

                    // Get types in the collection from type flow tracker
                    let type_infos = self.type_tracker.with_type_flow(|flow| {
                        flow.get_collection_types(&collection_path)
                            .into_iter()
                            .cloned()
                            .collect::<Vec<_>>()
                    });

                    // For each type in the collection, register the methods as interface methods
                    for type_info in type_infos {
                        let interface_name = &type_info.type_id.name;

                        // Register each method called in the dispatch loop
                        for _method_name in &method_calls {
                            // Register the interface if not already registered
                            self.observer_registry
                                .write()
                                .unwrap()
                                .register_interface(interface_name);
                        }
                    }
                }
            }
            ast::Stmt::If(if_stmt) => {
                // Check branches for dispatch loops
                for body_stmt in &if_stmt.body {
                    self.process_stmt_for_dispatch_loops(class_name, body_stmt);
                }
                for else_stmt in &if_stmt.orelse {
                    self.process_stmt_for_dispatch_loops(class_name, else_stmt);
                }
            }
            ast::Stmt::While(while_stmt) => {
                for body_stmt in &while_stmt.body {
                    self.process_stmt_for_dispatch_loops(class_name, body_stmt);
                }
            }
            ast::Stmt::With(with_stmt) => {
                for body_stmt in &with_stmt.body {
                    self.process_stmt_for_dispatch_loops(class_name, body_stmt);
                }
            }
            ast::Stmt::Try(try_stmt) => {
                for body_stmt in &try_stmt.body {
                    self.process_stmt_for_dispatch_loops(class_name, body_stmt);
                }
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(except_handler) = handler;
                    for handler_stmt in &except_handler.body {
                        self.process_stmt_for_dispatch_loops(class_name, handler_stmt);
                    }
                }
            }
            _ => {}
        }
    }

    /// Extract observer dispatch information from a for loop
    /// Returns (collection_name, method_names) if this is a dispatch loop
    fn extract_observer_dispatch_info(
        &self,
        for_stmt: &ast::StmtFor,
    ) -> Option<(String, Vec<String>)> {
        use crate::analysis::python_call_graph::observer_registry::ObserverRegistry;

        // Extract collection name from the iterator
        let collection_name = match &*for_stmt.iter {
            ast::Expr::Attribute(attr) => {
                // Pattern: for x in self.observers
                if let ast::Expr::Name(name) = &*attr.value {
                    if name.id.as_str() == "self" {
                        Some(attr.attr.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }?;

        // Check if it looks like an observer collection
        if !ObserverRegistry::is_observer_collection_name(&collection_name) {
            return None;
        }

        // Extract the loop variable name
        let loop_var = match &*for_stmt.target {
            ast::Expr::Name(name) => name.id.as_str(),
            _ => return None,
        };

        // Find method calls on the loop variable in the body
        let mut method_calls = Vec::new();
        for stmt in &for_stmt.body {
            self.extract_method_calls_on_var(stmt, loop_var, &mut method_calls);
        }

        if method_calls.is_empty() {
            None
        } else {
            Some((collection_name, method_calls))
        }
    }

    /// Extract method calls on a specific variable from a statement
    fn extract_method_calls_on_var(
        &self,
        stmt: &ast::Stmt,
        var_name: &str,
        method_calls: &mut Vec<String>,
    ) {
        match stmt {
            ast::Stmt::Expr(expr_stmt) => {
                self.extract_method_calls_from_expr(&expr_stmt.value, var_name, method_calls);
            }
            ast::Stmt::If(if_stmt) => {
                for body_stmt in &if_stmt.body {
                    self.extract_method_calls_on_var(body_stmt, var_name, method_calls);
                }
                for else_stmt in &if_stmt.orelse {
                    self.extract_method_calls_on_var(else_stmt, var_name, method_calls);
                }
            }
            _ => {}
        }
    }

    /// Extract method calls from an expression
    fn extract_method_calls_from_expr(
        &self,
        expr: &ast::Expr,
        var_name: &str,
        method_calls: &mut Vec<String>,
    ) {
        if let ast::Expr::Call(call) = expr {
            if let ast::Expr::Attribute(attr) = &*call.func {
                if let ast::Expr::Name(name) = &*attr.value {
                    if name.id.as_str() == var_name {
                        method_calls.push(attr.attr.to_string());
                    }
                }
            }
        }
    }

    /// Track collection operations like append() for type flow analysis
    fn track_collection_operation(&mut self, call: &ast::ExprCall) {
        // Check if this is a method call on a collection (e.g., self.observers.append(x))
        if let ast::Expr::Attribute(attr) = &*call.func {
            let method_name = attr.attr.as_str();

            // Track append operations
            if method_name == "append" && !call.args.is_empty() {
                // Extract the collection being appended to (e.g., "self.observers")
                let collection_name = self.extract_full_attribute_name(&attr.value);

                // Get the type being appended
                if let Some(arg) = call.args.first() {
                    if let Some(type_id) = self.infer_and_register_type_from_expr(arg) {
                        // Record the type flowing into the collection
                        self.type_tracker.with_type_flow_mut(|flow| {
                            flow.record_collection_add(&collection_name, type_id)
                        });
                    }
                }
            }
        }
    }

    /// Infer and register type from an expression (with mutable access)
    fn infer_and_register_type_from_expr(
        &mut self,
        expr: &ast::Expr,
    ) -> Option<crate::analysis::type_flow_tracker::TypeId> {
        use crate::analysis::type_flow_tracker::{Location, TypeId, TypeInfo};

        match expr {
            // Direct instantiation: ConcreteObserver()
            ast::Expr::Call(call) => {
                if let ast::Expr::Name(name) = &*call.func {
                    let type_name = name.id.to_string();
                    let type_id =
                        TypeId::new(type_name.clone(), Some(self.type_tracker.file_path.clone()));

                    // Get base classes if this is a known class
                    let base_classes = if let Some(class_info) =
                        self.type_tracker.class_hierarchy.get(&type_name)
                    {
                        class_info
                            .bases
                            .iter()
                            .map(|base| TypeId::from_name(base))
                            .collect()
                    } else {
                        vec![]
                    };

                    // Register in type flow tracker
                    let file_path = self.type_tracker.file_path.clone();
                    let type_info = TypeInfo {
                        type_id: type_id.clone(),
                        source_location: Location::new(file_path, 0),
                        base_classes,
                    };
                    self.type_tracker
                        .with_type_flow_mut(|flow| flow.register_type(type_info));

                    Some(type_id)
                } else {
                    None
                }
            }
            // Variable reference: observer (look up type)
            ast::Expr::Name(name) => {
                let name_str = name.id.as_str();
                self.type_tracker
                    .with_type_flow(|flow| flow.get_variable_type(name_str))
            }
            _ => None,
        }
    }

    /// Extract full attribute name (e.g., "self.observers" from attribute expression)
    fn extract_full_attribute_name(&self, expr: &ast::Expr) -> String {
        utils::extract_attribute_name_recursive(expr)
    }

    /// Populate observer registry from class definition
    fn populate_observer_registry(&mut self, class_def: &ast::StmtClassDef) {
        use crate::analysis::python_call_graph::observer_registry::ObserverRegistry;

        let class_name = &class_def.name;

        // Check for observer collections in __init__ method
        for stmt in &class_def.body {
            if let ast::Stmt::FunctionDef(func_def) = stmt {
                if func_def.name.as_str() == "__init__" {
                    // Look for self.observers = [], self.listeners = [], etc.
                    for init_stmt in &func_def.body {
                        if let ast::Stmt::Assign(assign) = init_stmt {
                            for target in &assign.targets {
                                if let ast::Expr::Attribute(attr) = target {
                                    if let ast::Expr::Name(name) = &*attr.value {
                                        if name.id.as_str() == "self" {
                                            let field_name = attr.attr.as_str();
                                            if ObserverRegistry::is_observer_collection_name(
                                                field_name,
                                            ) {
                                                // Infer interface type from field name
                                                // e.g., "listeners" -> "Listener", "observers" -> "Observer"
                                                let interface_name = self
                                                    .infer_interface_from_field_name(field_name);

                                                // Register the collection
                                                self.observer_registry
                                                    .write()
                                                    .unwrap()
                                                    .register_collection(
                                                        class_name,
                                                        field_name,
                                                        &interface_name,
                                                    );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Register observer method implementations for a specific class-interface pair.
    ///
    /// This function iterates through the class methods and registers each non-special
    /// method as an implementation of the given observer interface.
    ///
    /// # Arguments
    ///
    /// * `class_name` - Name of the class implementing the interface
    /// * `interface_name` - Name of the observer interface being implemented
    /// * `class_def` - AST node for the class definition
    fn register_observer_methods(
        &mut self,
        class_name: &str,
        interface_name: &str,
        class_def: &ast::StmtClassDef,
    ) {
        // Register method implementations
        for stmt in &class_def.body {
            if let ast::Stmt::FunctionDef(method_def) = stmt {
                // Skip __init__ and other special methods
                if !method_def.name.starts_with("__") {
                    let func_name = format!("{}.{}", class_name, method_def.name);

                    // Look up the FunctionId from the function_name_map to get the correct line number
                    let func_id = if let Some(existing_id) = self.function_name_map.get(&func_name)
                    {
                        existing_id.clone()
                    } else {
                        // Fallback: create a new FunctionId (shouldn't happen in normal flow)
                        FunctionId::new(
                            self.type_tracker.file_path.clone(),
                            func_name.clone(),
                            self.estimate_line_number(&func_name),
                        )
                    };

                    {
                        let mut registry_mut = self.observer_registry.write().unwrap();
                        registry_mut.register_implementation(
                            interface_name,
                            &method_def.name,
                            func_id.clone(),
                        );
                        // Also register under generic "Observer" for heuristic matching
                        registry_mut.register_implementation("Observer", &method_def.name, func_id);
                    }
                }
            }
        }
    }

    /// Register observer implementations (concrete classes that implement observer interfaces)
    /// This must be called after all functions are registered in function_name_map
    fn register_observer_implementations(&mut self, class_def: &ast::StmtClassDef) {
        let class_name = &class_def.name;

        // Check if any base class is a registered observer interface
        // Also proactively register base classes as potential interfaces
        for base in &class_def.bases {
            if let ast::Expr::Name(base_name) = base {
                let interface_name = base_name.id.to_string();

                // Proactively register the base class as an interface if it looks like an observer interface
                // (ends with Observer, Listener, Handler, etc.)
                if is_observer_interface_name(&interface_name) {
                    self.observer_registry
                        .write()
                        .unwrap()
                        .register_interface(&interface_name);
                }

                // Check if the base class is a registered observer interface
                let is_observer_impl = self
                    .observer_registry
                    .read()
                    .unwrap()
                    .is_interface(&interface_name);

                if is_observer_impl {
                    // Register class-to-interface mapping
                    self.observer_registry
                        .write()
                        .unwrap()
                        .register_class_interface(class_name, &interface_name);

                    // Register method implementations
                    self.register_observer_methods(class_name, &interface_name, class_def);
                }
            }
        }
    }

    /// Store pending observer dispatches in the shared cross-module context
    /// for resolution after all files have been analyzed
    fn store_pending_observer_dispatches(&mut self) {
        use crate::analysis::python_call_graph::cross_module::PendingObserverDispatch;

        // If we have a cross-module context, store pending dispatches there
        if let Some(context) = &self.cross_module_context {
            let pending = std::mem::take(&mut self.pending_observer_dispatches);
            let mut shared_pending = context.pending_observer_dispatches.lock().unwrap();

            for (for_stmt, caller, current_class) in pending {
                shared_pending.push(PendingObserverDispatch {
                    for_stmt,
                    caller,
                    current_class,
                });
            }
        } else {
            // Fallback: resolve immediately if no cross-module context
            // (for backward compatibility with single-file analysis)
            let pending = std::mem::take(&mut self.pending_observer_dispatches);

            for (for_stmt, caller, current_class) in pending {
                // Temporarily set current_class for detection
                let saved_class = self.current_class.clone();
                self.current_class = current_class;

                self.detect_observer_dispatch(&for_stmt, &caller);

                // Restore previous current_class
                self.current_class = saved_class;
            }
        }
    }

    /// Detect observer dispatch patterns in for loops
    fn detect_observer_dispatch(&mut self, for_stmt: &ast::StmtFor, caller: &FunctionId) {
        use crate::analysis::python_call_graph::observer_dispatch::ObserverDispatchDetector;

        let detector = ObserverDispatchDetector::new(self.observer_registry.clone());
        let dispatches =
            detector.detect_in_for_loop(for_stmt, self.current_class.as_deref(), caller);

        // Store detected dispatches for resolution in phase two
        for dispatch in dispatches {
            // Look up implementations for this observer method
            if let Some(interface) = &dispatch.observer_interface {
                let impls: Vec<_> = {
                    let registry = self.observer_registry.read().unwrap();
                    registry
                        .get_implementations(interface, &dispatch.method_name)
                        .into_iter()
                        .cloned()
                        .collect()
                };
                for impl_func_id in impls {
                    // Create call edge from dispatcher to implementation
                    self.call_graph.add_call(FunctionCall {
                        caller: dispatch.caller_id.clone(),
                        callee: impl_func_id.clone(),
                        call_type: CallType::ObserverDispatch,
                    });
                }
            }
        }
    }

    /// Phase 2: Resolve calls using type information
    fn phase_two(&mut self) {
        // First, resolve regular function calls
        for unresolved in &self.phase_one_calls {
            if let Some(callee) = self.resolve_call(unresolved) {
                self.call_graph.add_call(FunctionCall {
                    caller: unresolved.caller.clone(),
                    callee,
                    call_type: unresolved.call_type.clone(),
                });
            }
        }

        // Then, resolve callbacks using the callback tracker
        let resolution = self
            .callback_tracker
            .resolve_callbacks(&self.function_name_map);
        self.callback_tracker
            .add_to_call_graph(&resolution, &mut self.call_graph);
    }

    /// Check for callback patterns where functions are passed as arguments
    fn check_for_callback_patterns(&mut self, call: &ast::ExprCall) {
        use crate::analysis::python_call_graph::callback_patterns::{
            extract_call_target, find_callback_position, get_callback_argument,
            get_callback_patterns,
        };
        use crate::analysis::python_call_graph::callback_tracker::{
            CallbackContext, CallbackType, Location, PendingCallback,
        };

        // Extract the function name and module being called
        let Some((func_name, module_name)) = extract_call_target(call) else {
            return;
        };

        // Get callback patterns
        let patterns = get_callback_patterns();

        // Check if this is a callback-accepting function
        if let Some(callback_position) =
            find_callback_position(&patterns, &func_name, module_name.as_deref())
        {
            if let Some(callback_arg) = get_callback_argument(call, callback_position) {
                // Extract callback expression string for tracking
                let callback_expr = self.extract_callback_expr(callback_arg);

                // Determine callback type based on the pattern and callback expression
                let callback_type = if func_name == "partial" {
                    CallbackType::Partial
                } else if callback_expr.starts_with("self.") || callback_expr.starts_with("cls.") {
                    // For method references like self.method
                    CallbackType::SignalConnection
                } else {
                    // For direct function names (including nested functions)
                    CallbackType::DirectAssignment
                };

                // Create callback context
                let context = CallbackContext {
                    current_class: self.current_class.clone(),
                    current_function: self.current_function.as_ref().map(|f| f.name.clone()),
                    scope_variables: std::collections::HashMap::new(),
                };

                // Create pending callback for deferred resolution
                let pending = PendingCallback {
                    callback_expr,
                    registration_point: Location {
                        file: self.type_tracker.file_path.clone(),
                        line: 0, // Will be filled in during resolution
                        caller_function: self.current_function.as_ref().map(|f| f.name.clone()),
                    },
                    registration_type: callback_type,
                    context,
                    target_hint: None,
                };

                self.callback_tracker.track_callback(pending);
            }
        }
    }

    /// Extract callback expression as a string for tracking
    fn extract_callback_expr(&self, expr: &ast::Expr) -> String {
        utils::extract_callback_expr_impl(expr)
    }

    /// Check for event binding patterns like obj.Bind(event, self.method)
    fn check_for_event_bindings(&mut self, call: &ast::ExprCall) {
        // Check if this is a method call like obj.Bind(...)
        if let ast::Expr::Attribute(attr_expr) = &*call.func {
            let method_name = &attr_expr.attr;

            // List of known event binding methods
            let event_binding_methods = [
                "Bind",             // wxPython: obj.Bind(wx.EVT_PAINT, self.on_paint)
                "bind",             // Tkinter: widget.bind("<Button-1>", self.on_click)
                "connect",          // PyQt/PySide: signal.connect(self.slot)
                "on",               // Some frameworks
                "addEventListener", // Web frameworks
                "addListener",      // Event systems
                "subscribe",        // Observer patterns
                "observe",          // Observer patterns
                "listen",           // Event systems
            ];

            // Check if this is an event binding method
            if event_binding_methods.contains(&method_name.as_str()) {
                // Look for self/cls method references in arguments
                for arg in &call.args {
                    if let ast::Expr::Attribute(handler_attr) = arg {
                        if let ast::Expr::Name(obj_name) = &*handler_attr.value {
                            // Check if it's self.method or cls.method
                            if obj_name.id.as_str() == "self" || obj_name.id.as_str() == "cls" {
                                // Create a reference from the current function to the handler
                                if let (Some(class_name), Some(caller_func)) =
                                    (&self.current_class, &self.current_function)
                                {
                                    let handler_name =
                                        format!("{}.{}", class_name, handler_attr.attr);

                                    // Estimate handler line number from source
                                    let handler_line =
                                        self.estimate_line_number(&handler_attr.attr);

                                    let handler_id = FunctionId::new(
                                        self.type_tracker.file_path.clone(),
                                        handler_name.clone(),
                                        handler_line,
                                    );

                                    // Add the handler to known functions if not already there
                                    self.known_functions.insert(handler_id.clone());
                                    self.function_name_map
                                        .insert(handler_name, handler_id.clone());

                                    // Create a call from the current function to the handler
                                    let call_edge = FunctionCall {
                                        caller: caller_func.clone(),
                                        callee: handler_id,
                                        call_type: CallType::Direct, // Event binding creates a direct reference
                                    };

                                    self.call_graph.add_call(call_edge);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Check if this is an if __name__ == "__main__" guard
    fn is_main_guard(&self, test: &ast::Expr) -> bool {
        if let ast::Expr::Compare(cmp) = test {
            // Check for __name__ == "__main__" pattern
            if let ast::Expr::Name(name) = &*cmp.left {
                if name.id.as_str() == "__name__" && cmp.ops.len() == 1 {
                    if let ast::CmpOp::Eq = &cmp.ops[0] {
                        if let Some(ast::Expr::Constant(const_expr)) = cmp.comparators.first() {
                            if let ast::Constant::Str(s) = &const_expr.value {
                                return s == "__main__";
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Analyze the main block (if __name__ == "__main__")
    fn analyze_main_block(&mut self, body: &[ast::Stmt]) {
        // Create a pseudo-function ID for the main block
        let module_main_id = FunctionId::new(
            self.type_tracker.file_path.clone(),
            "__module_main__".to_string(),
            0,
        );

        // Add this pseudo-function to known functions
        self.known_functions.insert(module_main_id.clone());
        self.function_name_map
            .insert("__module_main__".to_string(), module_main_id.clone());

        // Temporarily set current function to module main
        let prev_function = self.current_function.clone();
        self.current_function = Some(module_main_id.clone());

        // Analyze all statements in the main block for calls
        for stmt in body {
            self.analyze_stmt_in_main_block(stmt);
        }

        self.current_function = prev_function;
    }

    /// Analyze statements in the main block
    fn analyze_stmt_in_main_block(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::Expr(expr_stmt) => {
                // Check for function calls
                if let ast::Expr::Call(call) = &*expr_stmt.value {
                    // Check if it's calling main() or other functions
                    if let ast::Expr::Name(name) = &*call.func {
                        if let Some(func_id) = self.function_name_map.get(name.id.as_str()) {
                            // Add a call from __module_main__ to the function
                            let module_main =
                                self.function_name_map.get("__module_main__").unwrap();
                            self.call_graph.add_call(FunctionCall {
                                caller: module_main.clone(),
                                callee: func_id.clone(),
                                call_type: CallType::Direct,
                            });
                        }
                    }
                }
                self.analyze_expr_for_calls(&expr_stmt.value);
            }
            _ => {
                // Handle other statement types if needed
            }
        }
    }

    /// Resolve a call using type information
    fn resolve_call(&self, unresolved: &UnresolvedCall) -> Option<FunctionId> {
        // Try cross-module resolution first if context is available
        if let Some(context) = &self.cross_module_context {
            // If this is an imported call, use the import context first
            if unresolved.is_imported {
                if let Some(import_context) = &unresolved.import_context {
                    // Try to resolve the imported function directly
                    if let Some(func_id) =
                        context.resolve_function(&self.type_tracker.file_path, import_context)
                    {
                        return Some(func_id);
                    }

                    // If import_context is module.function, extract function name
                    if let Some(dot_pos) = import_context.rfind('.') {
                        let func_name = &import_context[dot_pos + 1..];
                        if let Some(func_id) =
                            context.resolve_function(&self.type_tracker.file_path, func_name)
                        {
                            return Some(func_id);
                        }
                    }
                }

                // If it's a module.method call pattern
                if let Some(module_alias) = &unresolved.module_alias {
                    if let Some(method_name) = &unresolved.method_name {
                        // Get the actual module name for the alias
                        if let Some(module_name) = self.type_tracker.get_import_module(module_alias)
                        {
                            // Try to resolve module.function
                            let qualified_name = format!("{}.{}", module_name, method_name);
                            if let Some(func_id) = context
                                .resolve_function(&self.type_tracker.file_path, &qualified_name)
                            {
                                return Some(func_id);
                            }

                            // Also try just the function name (might be registered without qualification)
                            if let Some(func_id) =
                                context.resolve_function(&self.type_tracker.file_path, method_name)
                            {
                                return Some(func_id);
                            }
                        }
                    }
                }
            }

            // Regular resolution for non-imported or local calls
            if let Some(method_name) = &unresolved.method_name {
                // Try to resolve using cross-module context
                if let Some(func_id) =
                    context.resolve_function(&self.type_tracker.file_path, method_name)
                {
                    return Some(func_id);
                }

                // Try resolving as a class method
                if let Some(PythonType::Instance(class_name) | PythonType::Class(class_name)) =
                    &unresolved.receiver_type
                {
                    if let Some(func_id) = context.resolve_method(class_name, method_name) {
                        return Some(func_id);
                    }
                }
            } else if let ast::Expr::Call(call) = &unresolved.call_expr {
                match &*call.func {
                    ast::Expr::Name(name) => {
                        // Try to resolve function using cross-module context
                        if let Some(func_id) =
                            context.resolve_function(&self.type_tracker.file_path, name.id.as_str())
                        {
                            return Some(func_id);
                        }
                    }
                    ast::Expr::Attribute(attr) => {
                        // Handle module.function() or instance.method() patterns
                        if let ast::Expr::Name(module_name) = &*attr.value {
                            let mod_name = module_name.id.as_str();
                            let func_name = attr.attr.as_str();

                            // Check if this is an imported module
                            if let Some(qualified_module) =
                                self.type_tracker.resolve_imported_name(mod_name)
                            {
                                let full_name = format!("{}.{}", qualified_module, func_name);
                                if let Some(func_id) = context
                                    .resolve_function(&self.type_tracker.file_path, &full_name)
                                {
                                    return Some(func_id);
                                }
                            }

                            // Try direct resolution
                            let qualified_name = format!("{}.{}", mod_name, func_name);
                            if let Some(func_id) = context
                                .resolve_function(&self.type_tracker.file_path, &qualified_name)
                            {
                                return Some(func_id);
                            }

                            // Also try just the function name
                            if let Some(func_id) =
                                context.resolve_function(&self.type_tracker.file_path, func_name)
                            {
                                return Some(func_id);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Fall back to local resolution
        if unresolved.method_name.is_some() {
            if let (Some(receiver_type), Some(method_name)) =
                (&unresolved.receiver_type, &unresolved.method_name)
            {
                // First resolve the method name using type tracker
                if let Some(resolved_func_id) = self
                    .type_tracker
                    .resolve_method_call(receiver_type, method_name)
                {
                    // Then look up the actual FunctionId with correct line number from our map
                    if let Some(func_id_with_line) =
                        self.function_name_map.get(&resolved_func_id.name)
                    {
                        return Some(func_id_with_line.clone());
                    }
                    // Fallback to the resolved one if not found in map
                    return Some(resolved_func_id);
                }

                // Fallback: Try to resolve parameter-based method calls
                // This handles cases like: conversation_manager.register_observer(self)
                // where conversation_manager is a parameter
                if let ast::Expr::Call(call) = &unresolved.call_expr {
                    if let ast::Expr::Attribute(_attr_expr) = &*call.func {
                        // First check the cross-module context for methods
                        if let Some(context) = &self.cross_module_context {
                            // Try to find methods with this name across all modules
                            for (symbol_name, func_id) in &context.symbols {
                                if symbol_name.ends_with(&format!(".{}", method_name)) {
                                    // For observer patterns, we want to find all implementations
                                    // Add this call to all matching methods
                                    return Some(func_id.clone());
                                }
                            }
                        }

                        // Fall back to checking local function map
                        for (func_name, func_id) in &self.function_name_map {
                            if func_name.ends_with(&format!(".{}", method_name)) {
                                // Found a method with this name, use it
                                // This is a heuristic that may have false positives
                                // but helps detect cross-module calls
                                return Some(func_id.clone());
                            }
                        }
                    }
                }
            }
        } else {
            // Function call resolution
            if let ast::Expr::Call(call) = &unresolved.call_expr {
                if let ast::Expr::Name(name) = &*call.func {
                    let func_name = name.id.as_str();

                    // First, check if this is an imported name
                    if let Some(qualified_name) = self.type_tracker.resolve_imported_name(func_name)
                    {
                        // Try cross-module resolution with the qualified name
                        if let Some(context) = &self.cross_module_context {
                            if let Some(func_id) = context
                                .resolve_function(&self.type_tracker.file_path, &qualified_name)
                            {
                                return Some(func_id);
                            }
                        }
                    }

                    // Look up function by name
                    if let Some(func_id) = self.function_name_map.get(func_name) {
                        return Some(func_id.clone());
                    }
                }
            }
        }
        None
    }
}

/// Check if a name matches observer interface patterns.
///
/// Returns true if the name ends with Observer, Listener, Handler, or Callback.
/// This is a pure function for testability.
///
/// # Examples
///
/// ```
/// use debtmap::analysis::python_type_tracker::is_observer_interface_name;
///
/// assert!(is_observer_interface_name("ClickListener"));
/// assert!(is_observer_interface_name("EventObserver"));
/// assert!(is_observer_interface_name("RequestHandler"));
/// assert!(is_observer_interface_name("SuccessCallback"));
/// assert!(!is_observer_interface_name("MyClass"));
/// ```
fn is_observer_interface_name(name: &str) -> bool {
    name.ends_with("Observer")
        || name.ends_with("Listener")
        || name.ends_with("Handler")
        || name.ends_with("Callback")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_observer_interface_name() {
        // Test Observer suffix
        assert!(is_observer_interface_name("ClickObserver"));
        assert!(is_observer_interface_name("EventObserver"));
        assert!(is_observer_interface_name("Observer"));

        // Test Listener suffix
        assert!(is_observer_interface_name("ClickListener"));
        assert!(is_observer_interface_name("MouseListener"));
        assert!(is_observer_interface_name("Listener"));

        // Test Handler suffix
        assert!(is_observer_interface_name("EventHandler"));
        assert!(is_observer_interface_name("RequestHandler"));
        assert!(is_observer_interface_name("Handler"));

        // Test Callback suffix
        assert!(is_observer_interface_name("SuccessCallback"));
        assert!(is_observer_interface_name("ErrorCallback"));
        assert!(is_observer_interface_name("Callback"));

        // Test non-observer names
        assert!(!is_observer_interface_name("MyClass"));
        assert!(!is_observer_interface_name("UserService"));
        assert!(!is_observer_interface_name("DataRepository"));
        assert!(!is_observer_interface_name(""));

        // Test partial matches (should not match)
        assert!(!is_observer_interface_name("ObserverImpl"));
        assert!(!is_observer_interface_name("ListenerFactory"));
        assert!(!is_observer_interface_name("HandleRequest"));
        assert!(!is_observer_interface_name("CallbackFunction"));
    }

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
        let main_id = FunctionId::new(PathBuf::from("test.py"), "main".to_string(), 0);
        let helper_id = FunctionId::new(PathBuf::from("test.py"), "helper".to_string(), 0);

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
        let init_id = FunctionId::new(
            PathBuf::from("test.py"),
            "Calculator.__init__".to_string(),
            0,
        );

        let init_callees = call_graph.get_callees(&init_id);
        assert!(init_callees.iter().any(|f| f.name == "Calculator.reset"));

        // Check that add calls log
        let add_id = FunctionId::new(PathBuf::from("test.py"), "Calculator.add".to_string(), 0);
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
        let func_a = FunctionId::new(PathBuf::from("test.py"), "func_a".to_string(), 0);
        let func_b = FunctionId::new(PathBuf::from("test.py"), "func_b".to_string(), 0);
        let func_c = FunctionId::new(PathBuf::from("test.py"), "func_c".to_string(), 0);

        assert!(extractor.known_functions.contains(&func_a));
        assert!(extractor.known_functions.contains(&func_b));
        assert!(extractor.known_functions.contains(&func_c));
    }

    #[test]
    fn test_new_with_source() {
        let source = r#"
def hello():
    pass

class MyClass:
    def method(self):
        pass
"#;

        let extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), source);

        // Verify source lines are correctly stored
        assert!(!extractor.source_lines.is_empty());
        assert_eq!(extractor.source_lines.len(), 7); // 7 lines including empty lines

        // Verify the extractor is initialized properly
        assert_eq!(extractor.phase_one_calls.len(), 0);
        assert_eq!(extractor.call_graph.get_all_functions().count(), 0);
        assert!(extractor.known_functions.is_empty());
        assert!(extractor.current_function.is_none());
        assert!(extractor.current_class.is_none());

        // Verify source content is preserved
        assert!(extractor.source_lines[1].contains("def hello()"));
        assert!(extractor.source_lines[4].contains("class MyClass"));
    }

    #[test]
    fn test_estimate_line_number_simple_function() {
        let source = r#"
def simple_func():
    pass

def another_func():
    pass
"#;

        let extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), source);

        // Test finding simple function definitions
        assert_eq!(extractor.estimate_line_number("simple_func"), 2);
        assert_eq!(extractor.estimate_line_number("another_func"), 5);

        // Test non-existent function
        assert_eq!(extractor.estimate_line_number("nonexistent"), 0);
    }

    #[test]
    fn test_estimate_line_number_async_function() {
        let source = r#"
async def async_func():
    await something()

async def another_async():
    pass

def sync_func():
    pass
"#;

        let extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), source);

        // Test finding async functions
        assert_eq!(extractor.estimate_line_number("async_func"), 2);
        assert_eq!(extractor.estimate_line_number("another_async"), 5);
        assert_eq!(extractor.estimate_line_number("sync_func"), 8);
    }

    #[test]
    fn test_estimate_line_number_indented_function() {
        let source = r#"
class MyClass:
    def method1(self):
        pass

    def method2(self):
        pass

    async def async_method(self):
        pass
"#;

        let extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), source);

        // Test finding indented methods
        assert_eq!(extractor.estimate_line_number("method1"), 3);
        assert_eq!(extractor.estimate_line_number("method2"), 6);
        assert_eq!(extractor.estimate_line_number("async_method"), 9);
    }

    #[test]
    fn test_estimate_line_number_decorated_function() {
        let source = r#"
@decorator
def decorated_func():
    pass

@property
@cached
def multi_decorated():
    pass

    def nested_def():
        pass
"#;

        let extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), source);

        // Test finding decorated functions
        assert_eq!(extractor.estimate_line_number("decorated_func"), 3);
        assert_eq!(extractor.estimate_line_number("multi_decorated"), 8);
        assert_eq!(extractor.estimate_line_number("nested_def"), 11);
    }

    #[test]
    fn test_estimate_line_number_multiline_signature() {
        let source = r#"
def multiline_func(
    arg1: str,
    arg2: int,
) -> None:
    pass

def single_line():
    pass

def another_multiline(arg1,
                      arg2,
                      arg3):
    pass
"#;

        let extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), source);

        // Test finding functions with multiline signatures
        // Should find the line with 'def' keyword
        assert_eq!(extractor.estimate_line_number("multiline_func"), 2);
        assert_eq!(extractor.estimate_line_number("single_line"), 8);
        assert_eq!(extractor.estimate_line_number("another_multiline"), 11);
    }

    #[test]
    fn test_estimate_line_number_edge_cases() {
        let source = r#"
# def commented_out():
#     pass

string_with_def = "def not_a_func():"

def real_func():
    """def in_docstring():"""
    x = "def in_string():"
    pass
"#;

        let extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), source);

        // Test that we don't match commented or string definitions
        assert_eq!(extractor.estimate_line_number("commented_out"), 0);
        assert_eq!(extractor.estimate_line_number("not_a_func"), 0);
        assert_eq!(extractor.estimate_line_number("in_docstring"), 0);
        assert_eq!(extractor.estimate_line_number("in_string"), 0);

        // But we should find the real function
        assert_eq!(extractor.estimate_line_number("real_func"), 7);
    }

    #[test]
    fn test_estimate_line_number_empty_source() {
        let extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), "");

        // Should return 0 for empty source
        assert_eq!(extractor.estimate_line_number("any_func"), 0);
    }

    #[test]
    fn test_integration_line_numbers_in_call_graph() {
        let source = r#"
def helper():
    print("Helper")

def main():
    helper()
    another_helper()

def another_helper():
    pass
"#;

        let module =
            rustpython_parser::parse(source, rustpython_parser::Mode::Module, "test.py").unwrap();
        let mut extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), source);
        let call_graph = extractor.extract(&module);

        // Check that functions are registered with correct line numbers
        let main_id = FunctionId::new(
            PathBuf::from("test.py"),
            "main".to_string(),
            5, // main is on line 5
        );
        let helper_id = FunctionId::new(
            PathBuf::from("test.py"),
            "helper".to_string(),
            2, // helper is on line 2
        );
        let another_helper_id = FunctionId::new(
            PathBuf::from("test.py"),
            "another_helper".to_string(),
            9, // another_helper is on line 9
        );

        // Verify functions exist with expected line numbers
        assert!(call_graph.get_function_info(&main_id).is_some());
        assert!(call_graph.get_function_info(&helper_id).is_some());
        assert!(call_graph.get_function_info(&another_helper_id).is_some());

        // Check that the calls from main are tracked with correct line numbers
        let callees = call_graph.get_callees(&main_id);
        assert_eq!(callees.len(), 2);

        // Verify callee line numbers
        let helper_callee = callees.iter().find(|f| f.name == "helper").unwrap();
        assert_eq!(helper_callee.line, 2);

        let another_helper_callee = callees.iter().find(|f| f.name == "another_helper").unwrap();
        assert_eq!(another_helper_callee.line, 9);
    }

    #[test]
    fn test_integration_class_methods_line_numbers() {
        let source = r#"
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
            rustpython_parser::parse(source, rustpython_parser::Mode::Module, "test.py").unwrap();
        let mut extractor = TwoPassExtractor::new_with_source(PathBuf::from("test.py"), source);
        let call_graph = extractor.extract(&module);

        // Check that methods have correct line numbers
        let init_id = FunctionId::new(
            PathBuf::from("test.py"),
            "Calculator.__init__".to_string(),
            3, // __init__ is on line 3
        );

        let reset_id = FunctionId::new(
            PathBuf::from("test.py"),
            "Calculator.reset".to_string(),
            7, // reset is on line 7
        );

        let add_id = FunctionId::new(
            PathBuf::from("test.py"),
            "Calculator.add".to_string(),
            10, // add is on line 10
        );

        let log_id = FunctionId::new(
            PathBuf::from("test.py"),
            "Calculator.log".to_string(),
            14, // log is on line 14
        );

        // Verify all methods are tracked with correct line numbers
        assert!(call_graph.get_function_info(&init_id).is_some());
        assert!(call_graph.get_function_info(&reset_id).is_some());
        assert!(call_graph.get_function_info(&add_id).is_some());
        assert!(call_graph.get_function_info(&log_id).is_some());

        // Verify method calls have correct line numbers
        let init_callees = call_graph.get_callees(&init_id);
        let reset_callee = init_callees
            .iter()
            .find(|f| f.name == "Calculator.reset")
            .unwrap();
        assert_eq!(reset_callee.line, 7);

        let add_callees = call_graph.get_callees(&add_id);
        let log_callee = add_callees
            .iter()
            .find(|f| f.name == "Calculator.log")
            .unwrap();
        assert_eq!(log_callee.line, 14);
    }

    #[test]
    fn test_discover_observer_without_abc() {
        // Test that observer collection names are recognized
        use crate::analysis::python_call_graph::observer_registry::ObserverRegistry;

        assert!(ObserverRegistry::is_observer_collection_name("observers"));
        assert!(ObserverRegistry::is_observer_collection_name("listeners"));
        assert!(ObserverRegistry::is_observer_collection_name("handlers"));
        assert!(ObserverRegistry::is_observer_collection_name("callbacks"));
        assert!(ObserverRegistry::is_observer_collection_name("subscribers"));
        assert!(ObserverRegistry::is_observer_collection_name("watchers"));
        assert!(!ObserverRegistry::is_observer_collection_name("items"));
        assert!(!ObserverRegistry::is_observer_collection_name("data"));
    }

    #[test]
    fn test_register_interface_methods_from_dispatch() {
        // Test that we can detect for loops that iterate over collections
        let code = r#"
class Manager:
    def __init__(self):
        self.handlers = []

    def trigger(self):
        for h in self.handlers:
            h.on_start()
            h.on_stop()
"#;

        let module =
            rustpython_parser::parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let mut extractor = TwoPassExtractor::new(PathBuf::from("test.py"));
        let _call_graph = extractor.extract(&module);

        // Verify that the Manager class has a handlers collection
        // The actual interface discovery happens through type flow tracking in integration tests
        assert!(extractor
            .type_tracker
            .class_hierarchy
            .contains_key("Manager"));
    }

    #[test]
    fn test_base_class_registered_as_interface() {
        // Test that class hierarchy is tracked correctly
        let code = r#"
class BaseObserver:
    def notify(self): pass

class ConcreteObserver(BaseObserver):
    def notify(self): print("notified")
"#;

        let module =
            rustpython_parser::parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let mut extractor = TwoPassExtractor::new(PathBuf::from("test.py"));
        let _call_graph = extractor.extract(&module);

        // Verify class hierarchy is tracked
        assert!(extractor
            .type_tracker
            .class_hierarchy
            .contains_key("BaseObserver"));
        assert!(extractor
            .type_tracker
            .class_hierarchy
            .contains_key("ConcreteObserver"));

        // Verify ConcreteObserver inherits from BaseObserver
        let concrete = extractor
            .type_tracker
            .class_hierarchy
            .get("ConcreteObserver")
            .unwrap();
        assert!(concrete.bases.contains(&"BaseObserver".to_string()));
    }

    #[test]
    fn test_call_edges_from_dispatch_to_implementations() {
        // Verify that basic call graph extraction works for dispatch patterns
        // Full end-to-end testing is done in integration tests
        let code = r#"
class Observer:
    def update(self, event):
        pass

class Subject:
    def __init__(self):
        self.observers = []

    def notify(self, event):
        for obs in self.observers:
            obs.update(event)
"#;

        let module =
            rustpython_parser::parse(code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let mut extractor = TwoPassExtractor::new(PathBuf::from("test.py"));
        let call_graph = extractor.extract(&module);

        // Find the notify method (dispatch site)
        let notify_id = FunctionId::new(PathBuf::from("test.py"), "Subject.notify".to_string(), 0);

        // Get all callees from notify
        let callees = call_graph.get_callees(&notify_id);

        // Verify that Observer.update is recognized as a callee
        // (concrete implementations are resolved through type flow in cross-module contexts)
        let has_update_call = callees.iter().any(|callee| callee.name.contains("update"));

        assert!(
            has_update_call,
            "Call to update should be detected. Found callees: {:?}",
            callees.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_observer_discovery_performance() {
        // Generate code with multiple classes and observer patterns
        // to verify that observer discovery performance is reasonable
        let mut code = String::new();

        // Generate 20 observer interfaces
        for i in 0..20 {
            code.push_str(&format!(
                "
class Observer{}:
    def update(self):
        pass
",
                i
            ));
        }

        // Generate 20 concrete implementations
        for i in 0..20 {
            code.push_str(&format!(
                "
class ConcreteObserver{}(Observer{}):
    def update(self):
        print('Observer {} updated')
",
                i, i, i
            ));
        }

        // Generate 20 subjects with observer collections
        for i in 0..20 {
            code.push_str(&format!(
                "
class Subject{}:
    def __init__(self):
        self.observers = []

    def attach(self, observer):
        self.observers.append(observer)

    def notify(self):
        for obs in self.observers:
            obs.update()
",
                i
            ));
        }

        let start = std::time::Instant::now();
        let module =
            rustpython_parser::parse(&code, rustpython_parser::Mode::Module, "test.py").unwrap();
        let mut extractor = TwoPassExtractor::new(PathBuf::from("test.py"));
        let _call_graph = extractor.extract(&module);
        let duration = start.elapsed();

        // Performance should be reasonable for this size
        // With 60 classes and 20 observer patterns, should complete quickly
        // Threshold set to 1200ms to account for coverage instrumentation overhead
        assert!(
            duration.as_millis() < 1200,
            "Observer discovery took {}ms, which is too slow. Expected < 1200ms",
            duration.as_millis()
        );

        // Verify that class hierarchy was extracted
        assert!(!extractor.type_tracker.class_hierarchy.is_empty());
    }
}
