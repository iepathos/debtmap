//! Observer Pattern Analysis Module
//!
//! This module handles detection and analysis of observer/listener patterns in Python code.
//! It tracks observer interfaces, implementations, collections, and dispatch patterns.

use crate::analysis::python_call_graph::cross_module::CrossModuleContext;
use crate::analysis::python_call_graph::observer_dispatch::ObserverDispatchDetector;
use crate::analysis::python_call_graph::observer_registry::ObserverRegistry;
use crate::analysis::type_flow_tracker::TypeId;
use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use rustpython_parser::ast;
use std::sync::{Arc, RwLock};

use super::PythonTypeTracker;

/// Observer pattern analyzer
///
/// Handles detection of observer interfaces, implementations, and dispatch patterns.
pub struct ObserverAnalyzer {
    /// Observer registry for tracking observer patterns (shared across files via RwLock)
    observer_registry: Arc<RwLock<ObserverRegistry>>,
    /// Pending for loops that may contain observer dispatches
    /// Format: (for_stmt, caller_id, current_class)
    pending_observer_dispatches: Vec<(ast::StmtFor, FunctionId, Option<String>)>,
}

impl ObserverAnalyzer {
    /// Create a new observer analyzer
    pub fn new(observer_registry: Arc<RwLock<ObserverRegistry>>) -> Self {
        Self {
            observer_registry,
            pending_observer_dispatches: Vec::new(),
        }
    }

    /// Get the observer registry
    pub fn observer_registry(&self) -> Arc<RwLock<ObserverRegistry>> {
        self.observer_registry.clone()
    }

    /// Add a pending observer dispatch for later resolution
    pub fn add_pending_dispatch(
        &mut self,
        for_stmt: ast::StmtFor,
        caller: FunctionId,
        current_class: Option<String>,
    ) {
        self.pending_observer_dispatches
            .push((for_stmt, caller, current_class));
    }

    /// Register observer interfaces (classes that inherit from ABC) in the first pass
    pub fn register_observer_interfaces(&mut self, class_def: &ast::StmtClassDef) {
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
    pub fn find_observer_collections(module: &ast::ModModule) -> Vec<(String, String)> {
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
    pub fn collect_type_ids_for_observers(
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
    pub fn register_observer_interfaces_from_usage(
        observer_registry: &Arc<RwLock<ObserverRegistry>>,
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
    /// * `type_tracker` - The type tracker for querying type information
    pub fn discover_observer_interfaces_from_usage(
        &mut self,
        module: &ast::ModModule,
        type_tracker: &PythonTypeTracker,
    ) {
        // Step 1: Find all observer collections
        let observer_collections = Self::find_observer_collections(module);

        // Step 2: For each observer collection, get types from type flow tracker
        let type_ids_by_collection =
            Self::collect_type_ids_for_observers(type_tracker, &observer_collections);

        // Step 3: Register these types as observer interfaces
        Self::register_observer_interfaces_from_usage(
            &self.observer_registry,
            type_tracker,
            type_ids_by_collection,
        );

        // Step 4: Analyze dispatch loops to find interface methods
        self.analyze_dispatch_loops_for_interface_methods(module, type_tracker);
    }

    /// Analyze for loops to discover which methods are part of observer interfaces
    pub fn analyze_dispatch_loops_for_interface_methods(
        &mut self,
        module: &ast::ModModule,
        type_tracker: &PythonTypeTracker,
    ) {
        for stmt in &module.body {
            if let ast::Stmt::ClassDef(class_def) = stmt {
                for method in &class_def.body {
                    if let ast::Stmt::FunctionDef(func_def) = method {
                        self.find_and_register_interface_methods_in_function(
                            &class_def.name,
                            func_def,
                            type_tracker,
                        );
                    }
                }
            }
        }
    }

    /// Find dispatch loops in a function and register the methods being called
    pub fn find_and_register_interface_methods_in_function(
        &mut self,
        class_name: &ast::Identifier,
        func_def: &ast::StmtFunctionDef,
        type_tracker: &PythonTypeTracker,
    ) {
        // Recursively search for for-loops in the function body
        for stmt in &func_def.body {
            self.process_stmt_for_dispatch_loops(class_name, stmt, type_tracker);
        }
    }

    /// Process a statement looking for dispatch loops
    fn process_stmt_for_dispatch_loops(
        &mut self,
        class_name: &ast::Identifier,
        stmt: &ast::Stmt,
        type_tracker: &PythonTypeTracker,
    ) {
        match stmt {
            ast::Stmt::For(for_stmt) => {
                // Check if this is an observer dispatch pattern
                if let Some((collection_name, method_calls)) =
                    extract_observer_dispatch_info(for_stmt)
                {
                    // Get the full collection path
                    let collection_path = format!("{}.{}", class_name, collection_name);

                    // Get types in the collection from type flow tracker
                    let type_infos = type_tracker.with_type_flow(|flow| {
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
                    self.process_stmt_for_dispatch_loops(class_name, body_stmt, type_tracker);
                }
                for else_stmt in &if_stmt.orelse {
                    self.process_stmt_for_dispatch_loops(class_name, else_stmt, type_tracker);
                }
            }
            ast::Stmt::While(while_stmt) => {
                for body_stmt in &while_stmt.body {
                    self.process_stmt_for_dispatch_loops(class_name, body_stmt, type_tracker);
                }
            }
            ast::Stmt::With(with_stmt) => {
                for body_stmt in &with_stmt.body {
                    self.process_stmt_for_dispatch_loops(class_name, body_stmt, type_tracker);
                }
            }
            ast::Stmt::Try(try_stmt) => {
                for body_stmt in &try_stmt.body {
                    self.process_stmt_for_dispatch_loops(class_name, body_stmt, type_tracker);
                }
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(except_handler) = handler;
                    for handler_stmt in &except_handler.body {
                        self.process_stmt_for_dispatch_loops(
                            class_name,
                            handler_stmt,
                            type_tracker,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    /// Populate observer registry from class definition
    pub fn populate_observer_registry(&mut self, class_def: &ast::StmtClassDef) {
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
                                                let interface_name =
                                                    infer_interface_from_field_name(field_name);

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

    /// Proactively register a base class as an observer interface if it matches patterns.
    ///
    /// Checks if the given name matches observer interface patterns (Observer, Listener, etc.)
    /// and registers it as an interface if so.
    ///
    /// # Arguments
    ///
    /// * `interface_name` - Name of the potential observer interface
    pub fn register_interface_if_observer(&mut self, interface_name: &str) {
        if is_observer_interface_name(interface_name) {
            self.observer_registry
                .write()
                .unwrap()
                .register_interface(interface_name);
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
    /// * `resolve_method_fn` - Function to resolve method name to FunctionId
    pub fn register_observer_methods<F>(
        &mut self,
        class_name: &str,
        interface_name: &str,
        class_def: &ast::StmtClassDef,
        mut resolve_method_fn: F,
    ) where
        F: FnMut(&str, &str) -> FunctionId,
    {
        // Register method implementations
        for stmt in &class_def.body {
            if let ast::Stmt::FunctionDef(method_def) = stmt {
                // Skip __init__ and other special methods
                if !method_def.name.starts_with("__") {
                    let func_id = resolve_method_fn(class_name, &method_def.name);

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
    ///
    /// # Arguments
    /// * `class_def` - The class definition AST node
    /// * `resolve_method_fn` - Function to resolve method name to FunctionId
    pub fn register_observer_implementations<F>(
        &mut self,
        class_def: &ast::StmtClassDef,
        mut resolve_method_fn: F,
    ) where
        F: FnMut(&str, &str) -> FunctionId,
    {
        let class_name = &class_def.name;

        // Check if any base class is a registered observer interface
        // Also proactively register base classes as potential interfaces
        for base in &class_def.bases {
            if let ast::Expr::Name(base_name) = base {
                let interface_name = base_name.id.to_string();

                // Proactively register the base class as an interface if it looks like an observer interface
                self.register_interface_if_observer(&interface_name);

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
                    self.register_observer_methods(
                        class_name,
                        &interface_name,
                        class_def,
                        &mut resolve_method_fn,
                    );
                }
            }
        }
    }

    /// Store pending observer dispatches in the shared cross-module context
    /// for resolution after all files have been analyzed
    ///
    /// # Arguments
    /// * `cross_module_context` - Optional cross-module context
    /// * `detect_fn` - Fallback function for immediate detection if no cross-module context
    pub fn store_pending_observer_dispatches<F>(
        &mut self,
        cross_module_context: &Option<CrossModuleContext>,
        mut detect_fn: F,
    ) where
        F: FnMut(&ast::StmtFor, &FunctionId, Option<String>),
    {
        use crate::analysis::python_call_graph::cross_module::PendingObserverDispatch;

        // If we have a cross-module context, store pending dispatches there
        if let Some(context) = cross_module_context {
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
                detect_fn(&for_stmt, &caller, current_class);
            }
        }
    }

    /// Detect observer dispatch patterns in for loops
    ///
    /// # Arguments
    /// * `for_stmt` - The for loop statement to analyze
    /// * `caller` - The function ID of the caller
    /// * `current_class` - Optional current class context
    /// * `call_graph` - The call graph to add calls to
    pub fn detect_observer_dispatch(
        &self,
        for_stmt: &ast::StmtFor,
        caller: &FunctionId,
        current_class: Option<&str>,
        call_graph: &mut CallGraph,
    ) {
        let detector = ObserverDispatchDetector::new(self.observer_registry.clone());
        let dispatches = detector.detect_in_for_loop(for_stmt, current_class, caller);

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
                    call_graph.add_call(FunctionCall {
                        caller: dispatch.caller_id.clone(),
                        callee: impl_func_id.clone(),
                        call_type: CallType::ObserverDispatch,
                    });
                }
            }
        }
    }
}

// Pure helper functions

/// Extract observer dispatch information from a for loop
/// Returns (collection_name, method_names) if this is a dispatch loop
pub fn extract_observer_dispatch_info(for_stmt: &ast::StmtFor) -> Option<(String, Vec<String>)> {
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
        extract_method_calls_on_var(stmt, loop_var, &mut method_calls);
    }

    if method_calls.is_empty() {
        None
    } else {
        Some((collection_name, method_calls))
    }
}

/// Extract method calls on a specific variable from a statement
fn extract_method_calls_on_var(stmt: &ast::Stmt, var_name: &str, method_calls: &mut Vec<String>) {
    match stmt {
        ast::Stmt::Expr(expr_stmt) => {
            extract_method_calls_from_expr(&expr_stmt.value, var_name, method_calls);
        }
        ast::Stmt::If(if_stmt) => {
            for body_stmt in &if_stmt.body {
                extract_method_calls_on_var(body_stmt, var_name, method_calls);
            }
            for else_stmt in &if_stmt.orelse {
                extract_method_calls_on_var(else_stmt, var_name, method_calls);
            }
        }
        _ => {}
    }
}

/// Extract method calls from an expression
fn extract_method_calls_from_expr(
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

/// Infer interface name from field name
///
/// Converts plural observer collection names to singular interface names.
/// E.g., "observers" -> "Observer", "listeners" -> "Listener"
fn infer_interface_from_field_name(field_name: &str) -> String {
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

/// Check if a name looks like an observer interface
///
/// Returns true if the name ends with Observer, Listener, Handler, or Callback.
///
/// # Examples
///
/// ```ignore
/// assert!(is_observer_interface_name("ClickListener"));
/// assert!(is_observer_interface_name("EventObserver"));
/// assert!(is_observer_interface_name("RequestHandler"));
/// assert!(is_observer_interface_name("SuccessCallback"));
/// assert!(!is_observer_interface_name("MyClass"));
/// ```
pub fn is_observer_interface_name(name: &str) -> bool {
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
    }

    #[test]
    fn test_infer_interface_from_field_name() {
        assert_eq!(infer_interface_from_field_name("observers"), "Observer");
        assert_eq!(infer_interface_from_field_name("listeners"), "Listener");
        assert_eq!(infer_interface_from_field_name("handlers"), "Handler");
        assert_eq!(infer_interface_from_field_name("callbacks"), "Callback");

        // Edge cases
        assert_eq!(infer_interface_from_field_name("observer"), "Observer");
        assert_eq!(infer_interface_from_field_name(""), "Observer"); // Fallback
    }
}
