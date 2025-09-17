use crate::core::{DebtItem, DebtType, Priority};
use crate::debt::suppression::SuppressionContext;
use rustpython_parser::ast::{self, StmtTry, WithItem};
use std::path::Path;

/// Analyzer for detecting problematic error handling patterns in Python code
pub struct PythonErrorHandlingAnalyzer<'a> {
    current_file: &'a Path,
    suppression: Option<&'a SuppressionContext>,
    in_test_function: bool,
    current_line: usize,
}

impl<'a> PythonErrorHandlingAnalyzer<'a> {
    pub fn new(file_path: &'a Path, suppression: Option<&'a SuppressionContext>) -> Self {
        Self {
            current_file: file_path,
            suppression,
            in_test_function: false,
            current_line: 1,
        }
    }

    pub fn analyze(&mut self, module: &ast::Mod) -> Vec<DebtItem> {
        let mut items = Vec::new();

        if let ast::Mod::Module(m) = module {
            for stmt in &m.body {
                items.extend(self.analyze_statement(stmt));
            }
        }

        items
    }

    fn analyze_statement(&mut self, stmt: &ast::Stmt) -> Vec<DebtItem> {
        let mut items = Vec::new();

        // Update line tracking (approximation)
        self.current_line += 1;

        match stmt {
            ast::Stmt::Try(try_stmt) => {
                items.extend(self.analyze_try_statement(try_stmt));
            }
            ast::Stmt::With(with_stmt) => {
                items.extend(self.analyze_with_statement(with_stmt));
            }
            ast::Stmt::FunctionDef(func) => {
                let was_test = self.in_test_function;
                self.in_test_function = func.name.starts_with("test_");

                for stmt in &func.body {
                    items.extend(self.analyze_statement(stmt));
                }

                self.in_test_function = was_test;
            }
            ast::Stmt::AsyncFunctionDef(func) => {
                let was_test = self.in_test_function;
                self.in_test_function = func.name.starts_with("test_");

                for stmt in &func.body {
                    items.extend(self.analyze_statement(stmt));
                }

                self.in_test_function = was_test;
            }
            ast::Stmt::ClassDef(class) => {
                for stmt in &class.body {
                    items.extend(self.analyze_statement(stmt));
                }
            }
            ast::Stmt::If(if_stmt) => {
                for stmt in &if_stmt.body {
                    items.extend(self.analyze_statement(stmt));
                }
                for stmt in &if_stmt.orelse {
                    items.extend(self.analyze_statement(stmt));
                }
            }
            ast::Stmt::While(while_stmt) => {
                for stmt in &while_stmt.body {
                    items.extend(self.analyze_statement(stmt));
                }
                for stmt in &while_stmt.orelse {
                    items.extend(self.analyze_statement(stmt));
                }
            }
            ast::Stmt::For(for_stmt) => {
                for stmt in &for_stmt.body {
                    items.extend(self.analyze_statement(stmt));
                }
                for stmt in &for_stmt.orelse {
                    items.extend(self.analyze_statement(stmt));
                }
            }
            ast::Stmt::AsyncFor(for_stmt) => {
                for stmt in &for_stmt.body {
                    items.extend(self.analyze_statement(stmt));
                }
                for stmt in &for_stmt.orelse {
                    items.extend(self.analyze_statement(stmt));
                }
            }
            _ => {}
        }

        items
    }

    fn analyze_try_statement(&mut self, try_stmt: &StmtTry) -> Vec<DebtItem> {
        let mut items = Vec::new();

        // Check nested try statements
        for stmt in &try_stmt.body {
            items.extend(self.analyze_statement(stmt));
        }

        // Analyze exception handlers
        for handler in &try_stmt.handlers {
            items.extend(self.analyze_exception_handler(handler));

            // Check for nested try in handler
            let ast::ExceptHandler::ExceptHandler(h) = handler;
            for stmt in &h.body {
                items.extend(self.analyze_statement(stmt));
            }
        }

        // Check finally block
        for stmt in &try_stmt.finalbody {
            items.extend(self.analyze_statement(stmt));
        }

        // Check orelse block
        for stmt in &try_stmt.orelse {
            items.extend(self.analyze_statement(stmt));
        }

        items
    }

    fn analyze_with_statement(&mut self, with_stmt: &ast::StmtWith) -> Vec<DebtItem> {
        let mut items = Vec::new();

        // Check if it's contextlib.suppress usage (which can swallow errors)
        if self.is_contextlib_suppress(&with_stmt.items)
            && !self.is_suppressed(self.current_line, &DebtType::ErrorSwallowing) {
                items.push(self.create_debt_item(
                    self.current_line,
                    ErrorPattern::ContextlibSuppress,
                    "contextlib.suppress usage detected",
                ));
            }

        // Check body for nested error handling
        for stmt in &with_stmt.body {
            items.extend(self.analyze_statement(stmt));
        }

        items
    }

    fn analyze_exception_handler(&self, handler: &ast::ExceptHandler) -> Vec<DebtItem> {
        let ast::ExceptHandler::ExceptHandler(h) = handler;
        let mut items = Vec::new();

        // Check for bare except
        if h.type_.is_none() {
            items.push(self.create_debt_item(
                self.current_line,
                ErrorPattern::BareExcept,
                "Bare except clause catches all exceptions including system exits",
            ));
            // For bare except, still check if handler is empty
            if self.is_empty_handler(&h.body) {
                items.push(self.create_debt_item(
                    self.current_line,
                    ErrorPattern::EmptyHandler,
                    "Empty exception handler swallows errors without handling",
                ));
            }
            return items;
        }

        // Check for overly broad exception catching
        if let Some(typ) = &h.type_ {
            if let Some(pattern) = self.check_broad_exception(typ) {
                items.push(self.create_debt_item(
                    self.current_line,
                    pattern,
                    "Overly broad exception catching may hide bugs",
                ));
            }

            // Check for system exception suppression
            if let Some(pattern) = self.check_system_exception(typ) {
                items.push(self.create_debt_item(
                    self.current_line,
                    pattern,
                    "System exceptions should not be caught",
                ));
            }
        }

        // Check for empty handler (pass or ellipsis only)
        if self.is_empty_handler(&h.body) {
            items.push(self.create_debt_item(
                self.current_line,
                ErrorPattern::EmptyHandler,
                "Empty exception handler swallows errors without handling",
            ));
        }
        // Check for missing error context (no logging, re-raising, or context)
        else if !self.has_error_context(&h.body) {
            items.push(self.create_debt_item(
                self.current_line,
                ErrorPattern::NoErrorContext,
                "Exception caught without logging, re-raising, or context",
            ));
        }

        items
    }

    fn is_empty_handler(&self, body: &[ast::Stmt]) -> bool {
        body.len() == 1
            && match &body[0] {
                ast::Stmt::Pass(_) => true,
                ast::Stmt::Expr(expr_stmt) => {
                    matches!(&*expr_stmt.value, ast::Expr::Constant(_))
                }
                _ => false,
            }
    }

    fn check_broad_exception(&self, typ: &ast::Expr) -> Option<ErrorPattern> {
        match typ {
            ast::Expr::Name(name) => match name.id.as_str() {
                "Exception" => Some(ErrorPattern::OverlyBroad("Exception".to_string())),
                "BaseException" => Some(ErrorPattern::OverlyBroad("BaseException".to_string())),
                _ => None,
            },
            ast::Expr::Tuple(tuple) => {
                // Check if tuple contains broad exceptions
                for elt in &tuple.elts {
                    if let ast::Expr::Name(name) = elt {
                        match name.id.as_str() {
                            "Exception" | "BaseException" => {
                                return Some(ErrorPattern::OverlyBroad(format!(
                                    "{} in tuple",
                                    name.id
                                )));
                            }
                            _ => {}
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn check_system_exception(&self, typ: &ast::Expr) -> Option<ErrorPattern> {
        let check_name = |name: &str| -> Option<ErrorPattern> {
            match name {
                "KeyboardInterrupt" => Some(ErrorPattern::SystemExceptionSuppressed(
                    "KeyboardInterrupt".to_string(),
                )),
                "SystemExit" => Some(ErrorPattern::SystemExceptionSuppressed(
                    "SystemExit".to_string(),
                )),
                "GeneratorExit" => Some(ErrorPattern::SystemExceptionSuppressed(
                    "GeneratorExit".to_string(),
                )),
                _ => None,
            }
        };

        match typ {
            ast::Expr::Name(name) => check_name(&name.id),
            ast::Expr::Tuple(tuple) => {
                for elt in &tuple.elts {
                    if let ast::Expr::Name(name) = elt {
                        if let Some(pattern) = check_name(&name.id) {
                            return Some(pattern);
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn has_error_context(&self, body: &[ast::Stmt]) -> bool {
        for stmt in body {
            if self.statement_handles_error(stmt) {
                return true;
            }
        }
        false
    }

    fn statement_handles_error(&self, stmt: &ast::Stmt) -> bool {
        match stmt {
            // Check for raise statement (re-raising)
            ast::Stmt::Raise(_) => true,

            // Check for logging calls
            ast::Stmt::Expr(expr_stmt) => self.is_logging_call(&expr_stmt.value),

            // Check for return/yield that might propagate error info
            ast::Stmt::Return(ret) => {
                ret.value.is_some() // Non-empty return might be handling the error
            }

            _ => false,
        }
    }

    fn is_logging_call(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::Call(call) => {
                match &*call.func {
                    ast::Expr::Attribute(attr) => {
                        // Check for logger.error, logger.warning, etc.
                        matches!(
                            attr.attr.as_str(),
                            "error" | "warning" | "exception" | "critical" | "debug" | "info"
                        )
                    }
                    ast::Expr::Name(name) => {
                        // Check for print (basic logging)
                        name.id.as_str() == "print"
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn is_contextlib_suppress(&self, items: &[WithItem]) -> bool {
        for item in items {
            if let ast::Expr::Call(call) = &item.context_expr {
                if let ast::Expr::Attribute(attr) = &*call.func {
                    if attr.attr.as_str() == "suppress" {
                        if let ast::Expr::Name(name) = &*attr.value {
                            if name.id.as_str() == "contextlib" {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn is_suppressed(&self, line: usize, debt_type: &DebtType) -> bool {
        if let Some(checker) = self.suppression {
            checker.is_suppressed(line, debt_type)
        } else {
            false
        }
    }

    fn create_debt_item(&self, line: usize, pattern: ErrorPattern, context: &str) -> DebtItem {
        let priority = if self.in_test_function {
            Priority::Low
        } else {
            pattern.priority()
        };

        DebtItem {
            id: format!("py-error-{}-{}", self.current_file.display(), line),
            debt_type: DebtType::ErrorSwallowing,
            priority,
            file: self.current_file.to_path_buf(),
            line,
            column: None,
            message: format!("{}: {}", pattern.description(), pattern.remediation()),
            context: Some(context.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
enum ErrorPattern {
    BareExcept,
    EmptyHandler,
    #[allow(dead_code)]
    OverlyBroad(String),
    #[allow(dead_code)]
    SystemExceptionSuppressed(String),
    NoErrorContext,
    ContextlibSuppress,
}

impl ErrorPattern {
    fn description(&self) -> &str {
        match self {
            Self::BareExcept => "Bare except clause",
            Self::EmptyHandler => "Empty exception handler",
            Self::OverlyBroad(_) => "Overly broad exception catching",
            Self::SystemExceptionSuppressed(_) => "System exception suppression",
            Self::NoErrorContext => "Missing error context",
            Self::ContextlibSuppress => "contextlib.suppress usage",
        }
    }

    fn remediation(&self) -> &str {
        match self {
            Self::BareExcept => "Specify the exception types you want to catch",
            Self::EmptyHandler => "Handle the exception or remove the handler",
            Self::OverlyBroad(_) => "Catch specific exception types instead",
            Self::SystemExceptionSuppressed(_) => "Allow system exceptions to propagate",
            Self::NoErrorContext => "Add logging, re-raise, or handle the error properly",
            Self::ContextlibSuppress => "Consider explicit error handling instead of suppression",
        }
    }

    fn priority(&self) -> Priority {
        match self {
            Self::BareExcept => Priority::High,
            Self::EmptyHandler => Priority::High,
            Self::OverlyBroad(_) => Priority::Medium,
            Self::SystemExceptionSuppressed(_) => Priority::High,
            Self::NoErrorContext => Priority::Medium,
            Self::ContextlibSuppress => Priority::Low,
        }
    }
}

/// Public API for Python error handling detection
pub fn detect_error_swallowing(
    module: &ast::Mod,
    path: &Path,
    suppression: Option<&SuppressionContext>,
) -> Vec<DebtItem> {
    let mut analyzer = PythonErrorHandlingAnalyzer::new(path, suppression);
    analyzer.analyze(module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_parser::ast;

    fn parse_python(source: &str) -> ast::Mod {
        rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>").unwrap()
    }

    #[test]
    fn test_bare_except_detection() {
        let source = r#"
try:
    risky_operation()
except:
    pass
"#;
        let module = parse_python(source);
        let items = detect_error_swallowing(&module, Path::new("test.py"), None);

        // Should detect two issues: bare except and empty handler
        assert_eq!(items.len(), 2);
        assert!(items
            .iter()
            .any(|item| item.message.contains("Bare except")));
        assert!(items.iter().any(|item| item.message.contains("Empty")));
    }

    #[test]
    fn test_empty_handler_detection() {
        let source = r#"
try:
    something()
except ValueError:
    pass
"#;
        let module = parse_python(source);
        let items = detect_error_swallowing(&module, Path::new("test.py"), None);

        assert_eq!(items.len(), 1);
        assert!(items[0].message.contains("Empty exception handler"));
    }

    #[test]
    fn test_broad_exception_detection() {
        let source = r#"
try:
    operation()
except Exception as e:
    pass
"#;
        let module = parse_python(source);
        let items = detect_error_swallowing(&module, Path::new("test.py"), None);

        // Should detect two issues: overly broad and empty handler
        assert_eq!(items.len(), 2);
        assert!(items
            .iter()
            .any(|item| item.message.contains("Overly broad")));
        assert!(items.iter().any(|item| item.message.contains("Empty")));
    }

    #[test]
    fn test_system_exception_detection() {
        let source = r#"
try:
    long_running()
except KeyboardInterrupt:
    pass
"#;
        let module = parse_python(source);
        let items = detect_error_swallowing(&module, Path::new("test.py"), None);

        // Should detect two issues: system exception and empty handler
        assert_eq!(items.len(), 2);
        assert!(items
            .iter()
            .any(|item| item.message.contains("System exception")));
        assert!(items.iter().any(|item| item.message.contains("Empty")));
    }

    #[test]
    fn test_proper_error_handling_no_debt() {
        let source = r#"
import logging

try:
    risky_operation()
except ValueError as e:
    logging.error(f"Operation failed: {e}")
    raise
"#;
        let module = parse_python(source);
        let items = detect_error_swallowing(&module, Path::new("test.py"), None);

        // Should not detect debt for proper handling
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_multiple_exceptions_in_tuple() {
        let source = r#"
try:
    operation()
except (ValueError, KeyboardInterrupt, Exception):
    pass
"#;
        let module = parse_python(source);
        let items = detect_error_swallowing(&module, Path::new("test.py"), None);

        // Should detect at least 3 issues: system exception, overly broad, and empty handler
        assert!(items.len() >= 3);
        assert!(items
            .iter()
            .any(|item| item.message.contains("System exception")));
        assert!(items
            .iter()
            .any(|item| item.message.contains("Overly broad")));
        assert!(items.iter().any(|item| item.message.contains("Empty")));
    }

    #[test]
    fn test_contextlib_suppress_detection() {
        let source = r#"
import contextlib

with contextlib.suppress(ValueError):
    risky_operation()
"#;
        let module = parse_python(source);
        let items = detect_error_swallowing(&module, Path::new("test.py"), None);

        assert!(items
            .iter()
            .any(|item| item.message.contains("contextlib.suppress")));
    }

    #[test]
    fn test_nested_try_blocks() {
        let source = r#"
try:
    try:
        operation()
    except:
        pass
except:
    pass
"#;
        let module = parse_python(source);
        let items = detect_error_swallowing(&module, Path::new("test.py"), None);

        // Should detect 4 issues: 2 bare except and 2 empty handlers
        assert_eq!(items.len(), 4);
        assert_eq!(
            items
                .iter()
                .filter(|item| item.message.contains("Bare except"))
                .count(),
            2
        );
        assert_eq!(
            items
                .iter()
                .filter(|item| item.message.contains("Empty"))
                .count(),
            2
        );
    }

    #[test]
    fn test_in_test_function_lower_priority() {
        let source = r#"
def test_something():
    try:
        operation()
    except:
        pass
"#;
        let module = parse_python(source);
        let items = detect_error_swallowing(&module, Path::new("test.py"), None);

        // Should detect two issues: bare except and empty handler, both with low priority
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|item| item.priority == Priority::Low));
    }
}
