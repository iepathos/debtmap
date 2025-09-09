//! Event Tracking and Method Reference Module
//!
//! This module handles detection of event binding patterns and method references
//! that are commonly missed in static call graph analysis.

use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use anyhow::Result;
use rustpython_parser::ast;
use std::collections::HashMap;
use std::path::Path;

/// Pure function to get known event binding methods
pub fn get_event_binding_methods() -> &'static [&'static str] {
    &[
        "Bind",             // wxPython: obj.Bind(wx.EVT_PAINT, self.on_paint)
        "bind",             // Tkinter: widget.bind("<Button-1>", self.on_click)
        "connect",          // PyQt/PySide: signal.connect(self.slot)
        "on",               // Some frameworks
        "addEventListener", // Web frameworks
        "addListener",      // Event systems
        "subscribe",        // Observer patterns
        "observe",          // Observer patterns
        "listen",           // Event systems
    ]
}

/// Pure function to check if method name is an event binding method
pub fn is_event_binding_method_name(method_name: &str) -> bool {
    get_event_binding_methods().contains(&method_name)
}

/// Pure function to extract method name from attribute call
pub fn extract_method_name_from_call(call_expr: &ast::ExprCall) -> Option<&str> {
    match &*call_expr.func {
        ast::Expr::Attribute(attr_expr) => Some(&attr_expr.attr),
        _ => None,
    }
}

/// Pure function to check if argument is a self/cls method reference
pub fn is_self_or_cls_method_reference(arg: &ast::Expr) -> Option<&str> {
    match arg {
        ast::Expr::Attribute(handler_attr) => match &*handler_attr.value {
            ast::Expr::Name(obj_name)
                if obj_name.id.as_str() == "self" || obj_name.id.as_str() == "cls" =>
            {
                Some(&handler_attr.attr)
            }
            _ => None,
        },
        _ => None,
    }
}

/// Pure function to find self/cls method references in arguments
pub fn find_method_references_in_args(args: &[ast::Expr]) -> Vec<&str> {
    args.iter()
        .filter_map(is_self_or_cls_method_reference)
        .collect()
}

/// Event tracking functionality
pub struct EventTracker<'a> {
    pub current_class: Option<&'a str>,
    pub current_function: Option<&'a str>,
    pub function_lines: &'a HashMap<String, usize>,
}

impl<'a> EventTracker<'a> {
    pub fn new(
        current_class: Option<&'a str>,
        current_function: Option<&'a str>,
        function_lines: &'a HashMap<String, usize>,
    ) -> Self {
        Self {
            current_class,
            current_function,
            function_lines,
        }
    }

    /// Check for event handler binding patterns like obj.Bind(event, self.method)
    pub fn check_for_event_binding(
        &self,
        call_expr: &ast::ExprCall,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        // Check if this is a method call like obj.Bind(...)
        if let Some(method_name) = extract_method_name_from_call(call_expr) {
            if is_event_binding_method_name(method_name) {
                let method_refs = find_method_references_in_args(&call_expr.args);
                for method_ref in method_refs {
                    self.add_event_handler_reference(method_ref, file_path, call_graph)?;
                }
            }
        }
        Ok(())
    }

    /// Add an event handler reference to the call graph
    pub fn add_event_handler_reference(
        &self,
        method_name: &str,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        if let (Some(class_name), Some(caller_name)) = (self.current_class, self.current_function) {
            let handler_name = format!("{}.{}", class_name, method_name);

            // Use the actual line numbers we collected, or fall back to 0
            let caller_line = self.function_lines.get(caller_name).copied().unwrap_or(0);
            let handler_line = self.function_lines.get(&handler_name).copied().unwrap_or(0);

            let caller_id = FunctionId {
                name: caller_name.to_string(),
                file: file_path.to_path_buf(),
                line: caller_line,
            };

            let handler_id = FunctionId {
                name: handler_name,
                file: file_path.to_path_buf(),
                line: handler_line,
            };

            // Add the call edge to indicate the handler is referenced by the caller
            let call = FunctionCall {
                caller: caller_id,
                callee: handler_id,
                call_type: CallType::Direct, // Event binding is a direct reference
            };

            call_graph.add_call(call);
        }

        Ok(())
    }

    /// Add an instance method call to the call graph
    pub fn add_instance_method_call(
        &self,
        method_name: &str,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        if let (Some(class_name), Some(caller_name)) = (self.current_class, self.current_function) {
            let callee_name = format!("{}.{}", class_name, method_name);

            // Use the actual line numbers we collected, or fall back to 0
            let caller_line = self.function_lines.get(caller_name).copied().unwrap_or(0);
            let callee_line = self.function_lines.get(&callee_name).copied().unwrap_or(0);

            let caller_id = FunctionId {
                name: caller_name.to_string(),
                file: file_path.to_path_buf(),
                line: caller_line,
            };

            let callee_id = FunctionId {
                name: callee_name,
                file: file_path.to_path_buf(),
                line: callee_line,
            };

            // Add the call edge
            let call = FunctionCall {
                caller: caller_id,
                callee: callee_id,
                call_type: CallType::Direct,
            };

            call_graph.add_call(call);
        }

        Ok(())
    }

    /// Track method references for potential indirect calls
    pub fn track_method_reference(
        &self,
        _attr_expr: &ast::ExprAttribute,
        _file_path: &Path,
        _call_graph: &mut CallGraph,
    ) -> Result<()> {
        // Track method references for potential indirect calls
        // This would handle cases where methods are passed as arguments
        Ok(())
    }
}
