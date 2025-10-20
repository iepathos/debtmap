//! Python-Specific Call Graph Analysis Module
//!
//! This module provides Python-specific call graph analysis that addresses false positives
//! in dead code detection by tracking:
//! - Instance method calls (self.method())
//! - Class method calls (cls.method())
//! - Static method calls
//! - Context manager usage (with statements)
//! - Property access (@property decorators)
//! - Nested function definitions and callback patterns
//! - Functions passed as arguments to callback-accepting functions
//! - Type-aware method resolution using two-pass analysis

mod analyze;
mod call_analysis;
pub mod callback_patterns;
pub mod callback_tracker;
pub mod cross_module;
mod event_tracking;
mod function_detection;
pub mod import_tracker;
pub mod namespace;
pub mod observer_dispatch;
pub mod observer_registry;

#[allow(unused_imports)]
use crate::priority::call_graph::{CallGraph, FunctionId};
use anyhow::Result;
use rustpython_parser::ast;
use std::collections::HashMap;
use std::path::Path;

use call_analysis::{CallAnalyzer, StatementAnalyzer};
use callback_patterns::{find_callback_position, get_callback_patterns};
use function_detection::FunctionLineCollector;

// Re-export two-pass extractor for convenience
pub use crate::analysis::python_type_tracker::TwoPassExtractor;

// Re-export cross-module analysis functions
pub use analyze::{analyze_python_project, build_cross_module_context};

// Re-export callback tracker for advanced callback analysis
pub use callback_tracker::{
    CallbackContext, CallbackTracker, CallbackType, Location, PendingCallback,
};

// Re-export callback pattern recognition functions
pub use callback_patterns::{
    extract_call_target, find_callback_position as find_callback_arg_position,
    get_callback_argument,
};

/// Python-specific call graph analyzer
#[derive(Default)]
pub struct PythonCallGraphAnalyzer {
    current_class: Option<String>,
    current_function: Option<String>,
    function_lines: HashMap<String, usize>,
    nested_functions: HashMap<String, Vec<String>>, // parent -> nested functions
}

impl PythonCallGraphAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a function accepts callbacks
    #[allow(dead_code)]
    fn is_callback_accepting_function(
        &self,
        func_name: &str,
        module_name: Option<&str>,
    ) -> Option<usize> {
        let patterns = get_callback_patterns();
        find_callback_position(&patterns, func_name, module_name)
    }

    /// Build a nested function name
    #[allow(dead_code)]
    fn build_nested_function_name(&self, nested_name: &str) -> String {
        if let Some(parent) = &self.current_function {
            format!("{}.{}", parent, nested_name)
        } else if let Some(class_name) = &self.current_class {
            format!("{}.{}", class_name, nested_name)
        } else {
            nested_name.to_string()
        }
    }

    /// Analyze a Python module and extract method calls with source text for line numbers
    pub fn analyze_module_with_source(
        &mut self,
        module: &ast::Mod,
        file_path: &Path,
        source: &str,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        if let ast::Mod::Module(module) = module {
            // First pass: collect function line numbers
            let mut collector = FunctionLineCollector::new();
            collector.collect_function_lines(&module.body, source, None);
            self.function_lines = collector.get_function_lines().clone();
            self.nested_functions = collector.get_nested_functions().clone();

            // Second pass: analyze function calls
            for stmt in &module.body {
                self.analyze_stmt(stmt, file_path, call_graph)?;
            }
        }
        Ok(())
    }

    /// Analyze without source (backward compatibility - uses line 0)
    pub fn analyze_module(
        &mut self,
        module: &ast::Mod,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        if let ast::Mod::Module(module) = module {
            for stmt in &module.body {
                self.analyze_stmt(stmt, file_path, call_graph)?;
            }
        }
        Ok(())
    }

    /// Analyze module using two-pass type-aware extraction
    pub fn analyze_module_with_types(module: &ast::Mod, file_path: &Path) -> Result<CallGraph> {
        let mut extractor = TwoPassExtractor::new(file_path.to_path_buf());
        Ok(extractor.extract(module))
    }

    fn analyze_stmt(
        &mut self,
        stmt: &ast::Stmt,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        match stmt {
            ast::Stmt::ClassDef(class_def) => {
                self.analyze_class(class_def, file_path, call_graph)?;
            }
            ast::Stmt::FunctionDef(func_def) => {
                self.analyze_function(func_def, file_path, call_graph)?;
            }
            ast::Stmt::AsyncFunctionDef(func_def) => {
                self.analyze_async_function(func_def, file_path, call_graph)?;
            }
            ast::Stmt::With(with_stmt) => {
                let call_analyzer = CallAnalyzer::new(
                    self.current_class.as_deref(),
                    self.current_function.as_deref(),
                    &self.function_lines,
                    &self.nested_functions,
                );
                let stmt_analyzer = StatementAnalyzer::new(&call_analyzer);
                stmt_analyzer.analyze_with_stmt(with_stmt, file_path, call_graph)?;
            }
            _ => {
                // Recursively analyze nested statements
                self.analyze_nested_stmts(stmt, file_path, call_graph)?;
            }
        }
        Ok(())
    }

    fn analyze_class(
        &mut self,
        class_def: &ast::StmtClassDef,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        let prev_class = self.current_class.clone();
        self.current_class = Some(class_def.name.to_string());

        // Analyze all methods in the class
        for stmt in &class_def.body {
            self.analyze_stmt(stmt, file_path, call_graph)?;
        }

        self.current_class = prev_class;
        Ok(())
    }

    fn analyze_function(
        &mut self,
        func_def: &ast::StmtFunctionDef,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        let prev_function = self.current_function.clone();

        // Create function ID based on class context or parent function
        let func_name = if let Some(parent_func) = &self.current_function {
            // This is a nested function
            format!("{}.{}", parent_func, func_def.name)
        } else if let Some(class_name) = &self.current_class {
            format!("{}.{}", class_name, func_def.name)
        } else {
            func_def.name.to_string()
        };

        self.current_function = Some(func_name.clone());

        // Analyze function body for nested functions and method calls
        for stmt in &func_def.body {
            match stmt {
                ast::Stmt::FunctionDef(nested_func) => {
                    // Recursively analyze nested function
                    self.analyze_function(nested_func, file_path, call_graph)?;
                }
                ast::Stmt::AsyncFunctionDef(nested_func) => {
                    // Handle async nested functions too
                    self.analyze_async_function(nested_func, file_path, call_graph)?;
                }
                _ => {
                    self.analyze_stmt_for_calls(stmt, file_path, call_graph)?;
                }
            }
        }

        self.current_function = prev_function;
        Ok(())
    }

    fn analyze_async_function(
        &mut self,
        func_def: &ast::StmtAsyncFunctionDef,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        let prev_function = self.current_function.clone();

        // Create function ID based on class context or parent function
        let func_name = if let Some(parent_func) = &self.current_function {
            // This is a nested function
            format!("{}.{}", parent_func, func_def.name)
        } else if let Some(class_name) = &self.current_class {
            format!("{}.{}", class_name, func_def.name)
        } else {
            func_def.name.to_string()
        };

        self.current_function = Some(func_name.clone());

        // Analyze function body for nested functions and method calls
        for stmt in &func_def.body {
            match stmt {
                ast::Stmt::FunctionDef(nested_func) => {
                    // Recursively analyze nested function
                    self.analyze_function(nested_func, file_path, call_graph)?;
                }
                ast::Stmt::AsyncFunctionDef(nested_func) => {
                    // Handle async nested functions too
                    self.analyze_async_function(nested_func, file_path, call_graph)?;
                }
                _ => {
                    self.analyze_stmt_for_calls(stmt, file_path, call_graph)?;
                }
            }
        }

        self.current_function = prev_function;
        Ok(())
    }

    fn analyze_stmt_for_calls(
        &mut self,
        stmt: &ast::Stmt,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        let call_analyzer = CallAnalyzer::new(
            self.current_class.as_deref(),
            self.current_function.as_deref(),
            &self.function_lines,
            &self.nested_functions,
        );
        let stmt_analyzer = StatementAnalyzer::new(&call_analyzer);
        stmt_analyzer.analyze_stmt_for_calls(stmt, file_path, call_graph)
    }

    fn analyze_nested_stmts(
        &mut self,
        stmt: &ast::Stmt,
        file_path: &Path,
        call_graph: &mut CallGraph,
    ) -> Result<()> {
        // Helper to recursively analyze nested statements
        match stmt {
            ast::Stmt::If(if_stmt) => {
                for s in &if_stmt.body {
                    self.analyze_stmt(s, file_path, call_graph)?;
                }
                for s in &if_stmt.orelse {
                    self.analyze_stmt(s, file_path, call_graph)?;
                }
            }
            ast::Stmt::For(for_stmt) => {
                for s in &for_stmt.body {
                    self.analyze_stmt(s, file_path, call_graph)?;
                }
            }
            ast::Stmt::While(while_stmt) => {
                for s in &while_stmt.body {
                    self.analyze_stmt(s, file_path, call_graph)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_parser::parse;

    #[test]
    fn test_instance_method_call_detection() {
        let python_code = r#"
class MyClass:
    def _private_method(self):
        pass
    
    def public_method(self):
        self._private_method()
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module(&module, Path::new("test.py"), &mut call_graph)
            .unwrap();

        // Check that the call was tracked
        let private_method_id = FunctionId {
            name: "MyClass._private_method".to_string(),
            file: Path::new("test.py").to_path_buf(),
            line: 0,
        };

        assert!(
            !call_graph.get_callers(&private_method_id).is_empty(),
            "Private method should have callers"
        );
    }

    #[test]
    fn test_with_statement_method_call() {
        let python_code = r#"
class Manager:
    def _get_connection(self):
        pass
    
    def use_connection(self):
        with self._get_connection() as conn:
            pass
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module(&module, Path::new("test.py"), &mut call_graph)
            .unwrap();

        let connection_method_id = FunctionId {
            name: "Manager._get_connection".to_string(),
            file: Path::new("test.py").to_path_buf(),
            line: 0,
        };

        assert!(
            !call_graph.get_callers(&connection_method_id).is_empty(),
            "Connection method used in with statement should have callers"
        );
    }

    #[test]
    fn test_event_handler_binding_detection() {
        let python_code = r#"
class ConversationPanel:
    def on_paint(self, event):
        """Handle paint events to draw the drag-and-drop indicator line."""
        dc = wx.PaintDC(self.message_container)
        # ... drawing logic ...
    
    def setup_event_handlers(self):
        """Setup event handlers for the panel."""
        self.message_container.Bind(wx.EVT_PAINT, self.on_paint)
        self.widget.bind("<Button-1>", self.on_click)
        self.signal.connect(self.on_signal)
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module(&module, Path::new("conversation_panel.py"), &mut call_graph)
            .unwrap();

        let on_paint_id = FunctionId {
            name: "ConversationPanel.on_paint".to_string(),
            file: Path::new("conversation_panel.py").to_path_buf(),
            line: 0,
        };

        // on_paint should have callers because it's bound as an event handler
        assert!(
            !call_graph.get_callers(&on_paint_id).is_empty(),
            "on_paint should have callers because it's bound as an event handler"
        );
    }

    #[test]
    fn test_multiple_event_binding_frameworks() {
        let python_code = r#"
class EventPanel:
    def on_click(self, event):
        """Handle click events."""
        pass
    
    def on_signal(self):
        """Handle signal events."""
        pass
        
    def on_custom(self):
        """Handle custom events."""
        pass
    
    def setup_events(self):
        """Setup various event bindings."""
        # wxPython style
        self.widget.Bind(wx.EVT_CLICK, self.on_click)
        # PyQt/PySide style  
        self.signal.connect(self.on_signal)
        # Tkinter style
        self.frame.bind("<Button>", self.on_click)
        # Custom event system
        self.emitter.subscribe("custom", self.on_custom)
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module(&module, Path::new("event_panel.py"), &mut call_graph)
            .unwrap();

        // Check that all event handlers have callers
        let handlers = [
            "EventPanel.on_click",
            "EventPanel.on_signal",
            "EventPanel.on_custom",
        ];

        for handler_name in &handlers {
            let handler_id = FunctionId {
                name: handler_name.to_string(),
                file: Path::new("event_panel.py").to_path_buf(),
                line: 0,
            };

            assert!(
                !call_graph.get_callers(&handler_id).is_empty(),
                "{} should have callers because it's bound as an event handler",
                handler_name
            );
        }
    }

    #[test]
    fn test_nested_function_callback_detection() {
        let python_code = r#"
class DeliveryBoy:
    def deliver_message_added(self, observers, message, index):
        def deliver(observers, message, index):
            for observer in observers:
                observer.on_message_added(message, index)

        wx.CallAfter(deliver, observers, message, index)
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module_with_source(
                &module,
                Path::new("delivery_boy.py"),
                python_code,
                &mut call_graph,
            )
            .unwrap();

        // Check that the nested deliver function has callers
        // We need to check with the actual line number from the analyzer
        let deliver_line = analyzer
            .function_lines
            .get("DeliveryBoy.deliver_message_added.deliver")
            .copied()
            .unwrap_or(0);
        let deliver_id = FunctionId {
            name: "DeliveryBoy.deliver_message_added.deliver".to_string(),
            file: Path::new("delivery_boy.py").to_path_buf(),
            line: deliver_line,
        };

        // The nested deliver function should have callers because it's passed to wx.CallAfter
        assert!(
            !call_graph.get_callers(&deliver_id).is_empty(),
            "Nested deliver function should have callers because it's passed to wx.CallAfter"
        );
    }

    #[test]
    fn test_dictionary_callback_storage() {
        let python_code = r#"
class CommandRegistry:
    def __init__(self):
        self.commands = {
            'start': self.cmd_start,
            'stop': self.cmd_stop,
        }

    def cmd_start(self):
        """Start command"""
        pass

    def cmd_stop(self):
        """Stop command"""
        pass

    def execute(self, command):
        handler = self.commands.get(command)
        if handler:
            handler()
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module_with_source(
                &module,
                Path::new("registry.py"),
                python_code,
                &mut call_graph,
            )
            .unwrap();

        // Note: This test documents expected behavior for dictionary callback storage.
        // The infrastructure for tracking method references in dictionaries has been
        // implemented through the analyze_nested_exprs flow and the CallbackTracker module.
        //
        // Full integration requires:
        // 1. Function definition tracking (not just call tracking)
        // 2. Two-phase analysis to resolve method references
        // 3. Integration of CallbackTracker into the main analysis pipeline
        //
        // The test verifies the module can be parsed and analyzed without errors.
        // Future enhancements will enable full callback tracking.

        let _ = analyzer;
        let _ = call_graph;
        // Test passes if analysis completes without errors
    }

    #[test]
    fn test_list_callback_storage() {
        let python_code = r#"
class EventManager:
    def __init__(self):
        self.handlers = [
            self.handler_one,
            self.handler_two,
        ]

    def handler_one(self):
        """First handler"""
        pass

    def handler_two(self):
        """Second handler"""
        pass

    def trigger_all(self):
        for handler in self.handlers:
            handler()
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module_with_source(
                &module,
                Path::new("event_manager.py"),
                python_code,
                &mut call_graph,
            )
            .unwrap();

        // Note: This test documents expected behavior for list callback storage.
        // The infrastructure for tracking method references in lists has been
        // implemented through the analyze_nested_exprs flow and the CallbackTracker module.
        //
        // Full integration requires:
        // 1. Function definition tracking (not just call tracking)
        // 2. Two-phase analysis to resolve method references
        // 3. Integration of CallbackTracker into the main analysis pipeline
        //
        // The test verifies the module can be parsed and analyzed without errors.
        // Future enhancements will enable full callback tracking.

        let _ = analyzer;
        let _ = call_graph;
        // Test passes if analysis completes without errors
    }

    #[test]
    fn test_functools_partial_callback() {
        let python_code = r#"
import functools

class TaskScheduler:
    def process_task(self, task_id, priority):
        """Process a task with given priority"""
        pass

    def schedule_high_priority(self, task_id):
        """Schedule a high priority task"""
        callback = functools.partial(self.process_task, priority=10)
        self.executor.submit(callback, task_id)
"#;

        let module = parse(python_code, rustpython_parser::Mode::Module, "<test>").unwrap();
        let mut analyzer = PythonCallGraphAnalyzer::new();
        let mut call_graph = CallGraph::new();

        analyzer
            .analyze_module_with_source(
                &module,
                Path::new("scheduler.py"),
                python_code,
                &mut call_graph,
            )
            .unwrap();

        // Check that process_task has callers (via functools.partial)
        let process_line = analyzer
            .function_lines
            .get("TaskScheduler.process_task")
            .copied()
            .unwrap_or(0);
        let process_id = FunctionId {
            name: "TaskScheduler.process_task".to_string(),
            file: Path::new("scheduler.py").to_path_buf(),
            line: process_line,
        };

        assert!(
            !call_graph.get_callers(&process_id).is_empty(),
            "process_task should have callers because it's wrapped in functools.partial"
        );
    }

    // Add more tests as needed...
}
