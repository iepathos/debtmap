//! Observer Dispatch Detection Module
//!
//! Detects observer pattern dispatch through iteration loops and creates
//! appropriate call graph edges to concrete implementations.

use super::observer_registry::ObserverRegistry;
use crate::priority::call_graph::FunctionId;
use rustpython_parser::ast;
use std::sync::Arc;

/// Information about an observer dispatch call site
#[derive(Debug, Clone)]
pub struct ObserverDispatch {
    /// ID of the function containing the dispatch loop
    pub caller_id: FunctionId,
    /// Name of the method being called on observers
    pub method_name: String,
    /// Observer interface type (if known)
    pub observer_interface: Option<String>,
    /// Expression being iterated over
    pub collection_expr: String,
    /// Confidence score for this dispatch (0.0-1.0)
    pub confidence: f32,
}

/// Detector for observer pattern dispatch in for loops
pub struct ObserverDispatchDetector {
    registry: Arc<ObserverRegistry>,
}

impl ObserverDispatchDetector {
    /// Create a new observer dispatch detector
    pub fn new(registry: Arc<ObserverRegistry>) -> Self {
        Self { registry }
    }

    /// Detect observer dispatch patterns in a for loop
    ///
    /// # Arguments
    /// * `for_stmt` - The for statement to analyze
    /// * `current_class` - Name of the current class (if any)
    /// * `current_function` - ID of the current function
    pub fn detect_in_for_loop(
        &self,
        for_stmt: &ast::StmtFor,
        current_class: Option<&str>,
        current_function: &FunctionId,
    ) -> Vec<ObserverDispatch> {
        let mut dispatches = Vec::new();

        // Extract the target variable name from the for loop
        let target_var = match &*for_stmt.target {
            ast::Expr::Name(name) => name.id.as_str(),
            _ => return dispatches, // Complex targets not supported yet
        };

        // Check if iterating over an observer collection
        let (collection_expr, interface_type) =
            match self.analyze_collection_expr(&for_stmt.iter, current_class) {
                Some(info) => info,
                None => return dispatches,
            };

        // Find method calls on the iteration variable
        let method_calls = extract_method_calls_on_target(&for_stmt.body, target_var);

        // Create observer dispatch info for each method call
        for method_name in method_calls {
            let confidence = self.calculate_confidence(&collection_expr, &interface_type);

            dispatches.push(ObserverDispatch {
                caller_id: current_function.clone(),
                method_name,
                observer_interface: interface_type.clone(),
                collection_expr: collection_expr.clone(),
                confidence,
            });
        }

        dispatches
    }

    /// Analyze the collection expression to determine if it's an observer collection
    ///
    /// Returns (collection_expression, interface_type) if it's an observer collection
    fn analyze_collection_expr(
        &self,
        iter_expr: &ast::Expr,
        current_class: Option<&str>,
    ) -> Option<(String, Option<String>)> {
        match iter_expr {
            // self.observers, self.listeners, etc.
            ast::Expr::Attribute(attr) => {
                if let ast::Expr::Name(name) = &*attr.value {
                    if name.id.as_str() == "self" {
                        let field_name = attr.attr.as_str();
                        let collection_expr = format!("self.{}", field_name);

                        // Check if this is a registered observer collection
                        if let Some(class_name) = current_class {
                            if let Some(interface) = self
                                .registry
                                .get_collection_interface(class_name, field_name)
                            {
                                return Some((collection_expr, Some(interface.clone())));
                            }
                        }

                        // Fall back to heuristic check
                        if ObserverRegistry::is_observer_collection_name(field_name) {
                            return Some((collection_expr, None));
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Calculate confidence score for an observer dispatch
    fn calculate_confidence(&self, collection_expr: &str, interface_type: &Option<String>) -> f32 {
        let mut confidence: f32 = 0.85; // Base confidence

        // Extract field name from collection expression
        let field_name = collection_expr.split('.').next_back().unwrap_or("");

        // Higher confidence for known observer collection names
        if ObserverRegistry::is_observer_collection_name(field_name) {
            confidence += 0.05;
        }

        // Higher confidence if interface explicitly identified
        if interface_type.is_some() {
            confidence += 0.05;
        }

        confidence.clamp(0.70, 0.95)
    }
}

/// Extract method calls on a target variable from a list of statements
fn extract_method_calls_on_target(stmts: &[ast::Stmt], target_var: &str) -> Vec<String> {
    let mut method_calls = Vec::new();

    for stmt in stmts {
        extract_method_calls_recursive(stmt, target_var, &mut method_calls);
    }

    method_calls
}

/// Recursively extract method calls from a statement
fn extract_method_calls_recursive(
    stmt: &ast::Stmt,
    target_var: &str,
    method_calls: &mut Vec<String>,
) {
    match stmt {
        ast::Stmt::Expr(expr_stmt) => {
            extract_method_calls_from_expr(&expr_stmt.value, target_var, method_calls);
        }
        ast::Stmt::If(if_stmt) => {
            // Check condition
            extract_method_calls_from_expr(&if_stmt.test, target_var, method_calls);
            // Check body
            for s in &if_stmt.body {
                extract_method_calls_recursive(s, target_var, method_calls);
            }
            // Check else
            for s in &if_stmt.orelse {
                extract_method_calls_recursive(s, target_var, method_calls);
            }
        }
        ast::Stmt::Assign(assign_stmt) => {
            extract_method_calls_from_expr(&assign_stmt.value, target_var, method_calls);
        }
        _ => {}
    }
}

/// Extract method calls from an expression
fn extract_method_calls_from_expr(
    expr: &ast::Expr,
    target_var: &str,
    method_calls: &mut Vec<String>,
) {
    match expr {
        ast::Expr::Call(call_expr) => {
            // Check if this is target_var.method()
            if let ast::Expr::Attribute(attr) = &*call_expr.func {
                if let ast::Expr::Name(name) = &*attr.value {
                    if name.id.as_str() == target_var {
                        method_calls.push(attr.attr.to_string());
                    }
                }
            }
            // Recursively check arguments
            for arg in &call_expr.args {
                extract_method_calls_from_expr(arg, target_var, method_calls);
            }
        }
        ast::Expr::Attribute(attr) => {
            extract_method_calls_from_expr(&attr.value, target_var, method_calls);
        }
        ast::Expr::BinOp(binop) => {
            extract_method_calls_from_expr(&binop.left, target_var, method_calls);
            extract_method_calls_from_expr(&binop.right, target_var, method_calls);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_parser::parse;
    use std::path::PathBuf;

    fn create_test_function_id() -> FunctionId {
        FunctionId::new(PathBuf::from("test.py"), "Subject.notify".to_string(), 10)
    }

    #[test]
    fn test_detect_simple_observer_loop() {
        let python_code = r#"
for observer in self.observers:
    observer.on_event()
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        if let ast::Mod::Module(m) = &module {
            if let Some(ast::Stmt::For(for_stmt)) = m.body.first() {
                let registry = Arc::new(ObserverRegistry::new());
                let detector = ObserverDispatchDetector::new(registry);

                let dispatches = detector.detect_in_for_loop(
                    for_stmt,
                    Some("Subject"),
                    &create_test_function_id(),
                );

                assert_eq!(dispatches.len(), 1);
                assert_eq!(dispatches[0].method_name, "on_event");
                assert_eq!(dispatches[0].collection_expr, "self.observers");
                assert!(dispatches[0].confidence >= 0.85);
            } else {
                panic!("Expected for statement");
            }
        }
    }

    #[test]
    fn test_detect_multiple_method_calls() {
        let python_code = r#"
for listener in self.listeners:
    listener.on_start()
    listener.on_update()
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        if let ast::Mod::Module(m) = &module {
            if let Some(ast::Stmt::For(for_stmt)) = m.body.first() {
                let registry = Arc::new(ObserverRegistry::new());
                let detector = ObserverDispatchDetector::new(registry);

                let dispatches = detector.detect_in_for_loop(
                    for_stmt,
                    Some("Subject"),
                    &create_test_function_id(),
                );

                assert_eq!(dispatches.len(), 2);
                assert!(dispatches.iter().any(|d| d.method_name == "on_start"));
                assert!(dispatches.iter().any(|d| d.method_name == "on_update"));
            } else {
                panic!("Expected for statement");
            }
        }
    }

    #[test]
    fn test_detect_with_registered_interface() {
        let python_code = r#"
for observer in self.observers:
    observer.on_event()
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        if let ast::Mod::Module(m) = &module {
            if let Some(ast::Stmt::For(for_stmt)) = m.body.first() {
                let mut registry = ObserverRegistry::new();
                registry.register_collection("Subject", "observers", "Observer");
                let registry = Arc::new(registry);
                let detector = ObserverDispatchDetector::new(registry);

                let dispatches = detector.detect_in_for_loop(
                    for_stmt,
                    Some("Subject"),
                    &create_test_function_id(),
                );

                assert_eq!(dispatches.len(), 1);
                assert_eq!(
                    dispatches[0].observer_interface,
                    Some("Observer".to_string())
                );
                assert!(dispatches[0].confidence >= 0.90); // Higher confidence with interface
            } else {
                panic!("Expected for statement");
            }
        }
    }

    #[test]
    fn test_conditional_notification() {
        let python_code = r#"
for observer in self.observers:
    if condition:
        observer.on_event()
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        if let ast::Mod::Module(m) = &module {
            if let Some(ast::Stmt::For(for_stmt)) = m.body.first() {
                let registry = Arc::new(ObserverRegistry::new());
                let detector = ObserverDispatchDetector::new(registry);

                let dispatches = detector.detect_in_for_loop(
                    for_stmt,
                    Some("Subject"),
                    &create_test_function_id(),
                );

                assert_eq!(dispatches.len(), 1);
                assert_eq!(dispatches[0].method_name, "on_event");
            } else {
                panic!("Expected for statement");
            }
        }
    }

    #[test]
    fn test_no_detection_for_non_observer_collection() {
        let python_code = r#"
for item in self.items:
    item.process()
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        if let ast::Mod::Module(m) = &module {
            if let Some(ast::Stmt::For(for_stmt)) = m.body.first() {
                let registry = Arc::new(ObserverRegistry::new());
                let detector = ObserverDispatchDetector::new(registry);

                let dispatches = detector.detect_in_for_loop(
                    for_stmt,
                    Some("Subject"),
                    &create_test_function_id(),
                );

                assert_eq!(dispatches.len(), 0); // Should not detect non-observer collections
            } else {
                panic!("Expected for statement");
            }
        }
    }
}
