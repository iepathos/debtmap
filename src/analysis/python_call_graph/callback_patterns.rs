//! Callback Pattern Recognition Module
//!
//! This module provides pure functions for recognizing callback patterns in Python code.
//! It identifies functions that accept callbacks as arguments and tracks their position.

use rustpython_parser::ast;

/// Callback pattern configuration
#[derive(Debug, Clone)]
pub struct CallbackPattern {
    pub function_name: String,
    pub module_name: Option<String>,
    pub argument_position: usize, // Which argument is the callback (0-indexed)
}

/// Pure function to create wxPython callback patterns
fn create_wx_python_patterns() -> Vec<CallbackPattern> {
    vec![
        CallbackPattern {
            function_name: "CallAfter".to_string(),
            module_name: Some("wx".to_string()),
            argument_position: 0,
        },
        CallbackPattern {
            function_name: "CallLater".to_string(),
            module_name: Some("wx".to_string()),
            argument_position: 1,
        },
    ]
}

/// Pure function to create asyncio callback patterns
fn create_asyncio_patterns() -> Vec<CallbackPattern> {
    vec![
        CallbackPattern {
            function_name: "create_task".to_string(),
            module_name: Some("asyncio".to_string()),
            argument_position: 0,
        },
        CallbackPattern {
            function_name: "ensure_future".to_string(),
            module_name: Some("asyncio".to_string()),
            argument_position: 0,
        },
        CallbackPattern {
            function_name: "run_in_executor".to_string(),
            module_name: None,
            argument_position: 1,
        },
    ]
}

/// Pure function to create threading callback patterns
fn create_threading_patterns() -> Vec<CallbackPattern> {
    vec![
        CallbackPattern {
            function_name: "Timer".to_string(),
            module_name: Some("threading".to_string()),
            argument_position: 1,
        },
        CallbackPattern {
            function_name: "Thread".to_string(),
            module_name: Some("threading".to_string()),
            argument_position: 0,
        },
    ]
}

/// Pure function to create multiprocessing callback patterns
fn create_multiprocessing_patterns() -> Vec<CallbackPattern> {
    vec![
        CallbackPattern {
            function_name: "Process".to_string(),
            module_name: Some("multiprocessing".to_string()),
            argument_position: 0,
        },
        CallbackPattern {
            function_name: "apply_async".to_string(),
            module_name: None,
            argument_position: 0,
        },
    ]
}

/// Pure function to create generic callback patterns
fn create_generic_patterns() -> Vec<CallbackPattern> {
    vec![
        CallbackPattern {
            function_name: "schedule".to_string(),
            module_name: None,
            argument_position: 0,
        },
        CallbackPattern {
            function_name: "submit".to_string(),
            module_name: None,
            argument_position: 0,
        },
        CallbackPattern {
            function_name: "defer".to_string(),
            module_name: None,
            argument_position: 0,
        },
        CallbackPattern {
            function_name: "setTimeout".to_string(),
            module_name: None,
            argument_position: 0,
        },
    ]
}

/// Pure function to create functools callback patterns
fn create_functools_patterns() -> Vec<CallbackPattern> {
    vec![CallbackPattern {
        function_name: "partial".to_string(),
        module_name: Some("functools".to_string()),
        argument_position: 0,
    }]
}

/// Pure function to create web framework callback patterns
fn create_web_framework_patterns() -> Vec<CallbackPattern> {
    vec![
        // Flask/FastAPI route decorators - these are handled differently
        // but we list them for completeness
        CallbackPattern {
            function_name: "route".to_string(),
            module_name: None,
            argument_position: 0,
        },
        CallbackPattern {
            function_name: "get".to_string(),
            module_name: None,
            argument_position: 0,
        },
        CallbackPattern {
            function_name: "post".to_string(),
            module_name: None,
            argument_position: 0,
        },
        // Click command decorators
        CallbackPattern {
            function_name: "command".to_string(),
            module_name: None,
            argument_position: 0,
        },
    ]
}

/// Get callback patterns for a given function name
pub fn get_callback_patterns() -> Vec<CallbackPattern> {
    [
        create_wx_python_patterns(),
        create_asyncio_patterns(),
        create_threading_patterns(),
        create_multiprocessing_patterns(),
        create_generic_patterns(),
        create_functools_patterns(),
        create_web_framework_patterns(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Pure function to check if pattern matches function call
pub fn pattern_matches_call(
    pattern: &CallbackPattern,
    func_name: &str,
    module_name: Option<&str>,
) -> bool {
    if pattern.function_name != func_name {
        return false;
    }

    match (&pattern.module_name, module_name) {
        (Some(pattern_module), Some(actual_module)) => pattern_module == actual_module,
        (None, _) => true, // Pattern doesn't specify module, match on function name alone
        (Some(_), None) => false, // Pattern requires module but none provided
    }
}

/// Pure function to find callback position for function
pub fn find_callback_position(
    patterns: &[CallbackPattern],
    func_name: &str,
    module_name: Option<&str>,
) -> Option<usize> {
    patterns
        .iter()
        .find(|pattern| pattern_matches_call(pattern, func_name, module_name))
        .map(|pattern| pattern.argument_position)
}

/// Pure function to extract function and module name from call expression
pub fn extract_call_target(call_expr: &ast::ExprCall) -> Option<(String, Option<String>)> {
    match &*call_expr.func {
        ast::Expr::Name(name) => Some((name.id.to_string(), None)),
        ast::Expr::Attribute(attr_expr) => {
            let func_name = attr_expr.attr.to_string();
            let module_name = match &*attr_expr.value {
                ast::Expr::Name(module) => Some(module.id.to_string()),
                _ => None, // More complex expression like obj.method.CallAfter
            };
            Some((func_name, module_name))
        }
        _ => None,
    }
}

/// Pure function to get callback argument if valid callback position exists
pub fn get_callback_argument(
    call_expr: &ast::ExprCall,
    callback_position: usize,
) -> Option<&ast::Expr> {
    if call_expr.args.len() > callback_position {
        Some(&call_expr.args[callback_position])
    } else {
        None
    }
}

/// Pure function to check if an expression is a lambda
#[allow(dead_code)]
pub fn is_lambda_expr(expr: &ast::Expr) -> bool {
    matches!(expr, ast::Expr::Lambda(_))
}

/// Pure function to check if an expression is a partial call
#[allow(dead_code)]
pub fn is_partial_call(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Call(call_expr) => {
            if let Some((func_name, module_name)) = extract_call_target(call_expr) {
                func_name == "partial" && module_name.as_deref() == Some("functools")
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Pure function to extract function name from simple name expression
#[allow(dead_code)]
pub fn extract_function_name(expr: &ast::Expr) -> Option<String> {
    match expr {
        ast::Expr::Name(name) => Some(name.id.to_string()),
        ast::Expr::Attribute(attr) => {
            // For self.method or cls.method
            if let ast::Expr::Name(obj) = &*attr.value {
                Some(format!("{}.{}", obj.id, attr.attr))
            } else {
                Some(attr.attr.to_string())
            }
        }
        _ => None,
    }
}

/// Pure function to check if an expression is a method reference (self.method, cls.method)
#[allow(dead_code)]
pub fn is_method_reference(expr: &ast::Expr) -> Option<(String, String)> {
    if let ast::Expr::Attribute(attr) = expr {
        if let ast::Expr::Name(obj) = &*attr.value {
            if obj.id.as_str() == "self" || obj.id.as_str() == "cls" {
                return Some((obj.id.to_string(), attr.attr.to_string()));
            }
        }
    }
    None
}
