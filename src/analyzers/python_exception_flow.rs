// Exception flow analysis for Python code
//
// This module provides comprehensive exception propagation analysis for Python,
// tracking exception types through the call graph, identifying exception handling
// patterns, detecting missing error handling, and providing visibility into
// exception flows similar to Rust's Result propagation analysis.

use crate::core::{DebtItem, DebtType, Priority};
use rustpython_parser::ast;
use std::collections::HashMap;
use std::path::PathBuf;

/// Tracks exception flows through Python code
pub struct ExceptionFlowAnalyzer {
    /// Registry of all custom exception classes
    custom_exceptions: HashMap<String, ExceptionClass>,
    /// Exception flows for each function
    exception_flows: HashMap<String, ExceptionFlow>,
    /// Current file path
    current_file: PathBuf,
}

impl ExceptionFlowAnalyzer {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            custom_exceptions: HashMap::new(),
            exception_flows: HashMap::new(),
            current_file: file_path,
        }
    }

    /// Analyze exception flows in a module
    pub fn analyze_module(&mut self, module: &ast::Mod) -> Vec<ExceptionFlowPattern> {
        let ast::Mod::Module(m) = module else {
            return Vec::new();
        };

        // Phase 1: Register custom exception classes
        self.register_custom_exceptions(&m.body);

        // Phase 2: Analyze exception flows in functions
        for stmt in &m.body {
            self.analyze_statement(stmt, None);
        }

        // Phase 3: Detect patterns
        self.detect_patterns()
    }

    /// Register custom exception classes defined in the module
    fn register_custom_exceptions(&mut self, stmts: &[ast::Stmt]) {
        for stmt in stmts {
            if let ast::Stmt::ClassDef(class_def) = stmt {
                if self.is_exception_class(class_def) {
                    let base_classes = class_def
                        .bases
                        .iter()
                        .filter_map(|base| self.extract_class_name(base))
                        .collect();

                    self.custom_exceptions.insert(
                        class_def.name.to_string(),
                        ExceptionClass {
                            name: class_def.name.to_string(),
                            base_classes,
                            docstring: extract_docstring(&class_def.body),
                        },
                    );
                }
            }
        }
    }

    /// Check if a class definition is an exception class
    fn is_exception_class(&self, class_def: &ast::StmtClassDef) -> bool {
        class_def.bases.iter().any(|base| {
            if let Some(name) = self.extract_class_name(base) {
                self.is_exception_type(&name)
            } else {
                false
            }
        })
    }

    /// Extract class name from an expression
    fn extract_class_name(&self, expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::Name(name) => Some(name.id.to_string()),
            ast::Expr::Attribute(attr) => Some(attr.attr.to_string()),
            _ => None,
        }
    }

    /// Check if a name represents an exception type
    fn is_exception_type(&self, name: &str) -> bool {
        // Built-in exceptions
        if BUILTIN_EXCEPTIONS.contains(&name) {
            return true;
        }

        // Custom exceptions
        if self.custom_exceptions.contains_key(name) {
            return true;
        }

        // Name suggests it's an exception
        name.ends_with("Error")
            || name.ends_with("Exception")
            || name.ends_with("Warning")
            || name == "BaseException"
    }

    /// Analyze a statement for exception flows
    fn analyze_statement(&mut self, stmt: &ast::Stmt, _function_name: Option<&str>) {
        match stmt {
            ast::Stmt::FunctionDef(func_def) => {
                let func_name = func_def.name.to_string();
                let flow = self.analyze_function(func_def);
                self.exception_flows.insert(func_name, flow);
            }
            ast::Stmt::AsyncFunctionDef(func_def) => {
                let func_name = format!("async {}", func_def.name);
                let flow = self.analyze_async_function(func_def);
                self.exception_flows.insert(func_name, flow);
            }
            ast::Stmt::ClassDef(class_def) => {
                for stmt in &class_def.body {
                    self.analyze_statement(stmt, None);
                }
            }
            _ => {}
        }
    }

    /// Analyze exception flows in a function
    fn analyze_function(&self, func_def: &ast::StmtFunctionDef) -> ExceptionFlow {
        let mut flow = ExceptionFlow::new(func_def.name.to_string());

        // Extract exception documentation from docstring
        if let Some(docs) = self.extract_exception_docs(func_def) {
            flow.documented_exceptions = docs;
        }

        // Analyze function body
        self.analyze_function_body(&func_def.body, &mut flow);

        flow
    }

    /// Analyze exception flows in an async function
    fn analyze_async_function(&self, func_def: &ast::StmtAsyncFunctionDef) -> ExceptionFlow {
        let mut flow = ExceptionFlow::new(format!("async {}", func_def.name));

        // Extract exception documentation from docstring
        if let Some(docs) = self.extract_exception_docs_async(func_def) {
            flow.documented_exceptions = docs;
        }

        // Analyze function body
        self.analyze_function_body(&func_def.body, &mut flow);

        flow
    }

    /// Analyze function body for exception flows
    fn analyze_function_body(&self, body: &[ast::Stmt], flow: &mut ExceptionFlow) {
        for stmt in body {
            match stmt {
                ast::Stmt::Raise(raise_stmt) => {
                    if let Some(exception_info) = self.track_raise(raise_stmt) {
                        flow.raised_exceptions.push(exception_info);
                    }
                }
                ast::Stmt::Try(try_stmt) => {
                    self.analyze_try_statement(try_stmt, flow);
                }
                ast::Stmt::If(if_stmt) => {
                    self.analyze_function_body(&if_stmt.body, flow);
                    self.analyze_function_body(&if_stmt.orelse, flow);
                }
                ast::Stmt::While(while_stmt) => {
                    self.analyze_function_body(&while_stmt.body, flow);
                }
                ast::Stmt::For(for_stmt) => {
                    self.analyze_function_body(&for_stmt.body, flow);
                }
                ast::Stmt::With(with_stmt) => {
                    self.analyze_function_body(&with_stmt.body, flow);
                }
                _ => {}
            }
        }
    }

    /// Track a raise statement
    fn track_raise(&self, raise_stmt: &ast::StmtRaise) -> Option<ExceptionInfo> {
        let exc = raise_stmt.exc.as_ref()?;

        let exception_type = match exc.as_ref() {
            ast::Expr::Name(name) => ExceptionType::from_name(&name.id, &self.custom_exceptions),
            ast::Expr::Call(call) => {
                if let ast::Expr::Name(name) = call.func.as_ref() {
                    ExceptionType::from_name(&name.id, &self.custom_exceptions)
                } else {
                    ExceptionType::Unknown
                }
            }
            _ => ExceptionType::Unknown,
        };

        let source_exception = raise_stmt
            .cause
            .as_ref()
            .and_then(|cause| self.extract_exception_from_expr(cause))
            .map(Box::new);

        Some(ExceptionInfo {
            exception_type,
            is_documented: false, // Will be set during validation
            context_message: None,
            source_exception,
        })
    }

    /// Extract exception type from an expression
    fn extract_exception_from_expr(&self, expr: &ast::Expr) -> Option<ExceptionInfo> {
        match expr {
            ast::Expr::Name(name) => Some(ExceptionInfo {
                exception_type: ExceptionType::from_name(&name.id, &self.custom_exceptions),
                is_documented: false,
                context_message: None,
                source_exception: None,
            }),
            _ => None,
        }
    }

    /// Analyze a try statement
    fn analyze_try_statement(&self, try_stmt: &ast::StmtTry, flow: &mut ExceptionFlow) {
        // Analyze try block
        self.analyze_function_body(&try_stmt.body, flow);

        // Analyze exception handlers
        for handler in &try_stmt.handlers {
            let caught_exception = self.analyze_handler(handler);
            flow.caught_exceptions.push(caught_exception);

            // Check for exception transformations
            let ast::ExceptHandler::ExceptHandler(h) = handler;
            for stmt in &h.body {
                if let ast::Stmt::Raise(raise_stmt) = stmt {
                    if let Some(exc_info) = self.track_raise(raise_stmt) {
                        // This is an exception transformation
                        if let Some(caught_type) = h
                            .type_
                            .as_ref()
                            .and_then(|t| self.extract_exception_type(t))
                        {
                            let preserves_context = raise_stmt.cause.is_some();
                            flow.transformed_exceptions.push(ExceptionTransformation {
                                caught_type,
                                raised_type: exc_info.exception_type,
                                preserves_context,
                            });
                        }
                    }
                }
            }
        }

        // Analyze finally block
        self.analyze_function_body(&try_stmt.finalbody, flow);
    }

    /// Analyze an exception handler
    fn analyze_handler(&self, handler: &ast::ExceptHandler) -> CaughtException {
        let ast::ExceptHandler::ExceptHandler(h) = handler;

        let (handler_type, exception_types) = if let Some(exc_type) = &h.type_ {
            let types = self.extract_exception_types(exc_type);
            let handler_type = if types.is_empty() {
                HandlerType::Bare
            } else if types.len() > 1 {
                HandlerType::Multiple
            } else if types[0].is_broad() {
                HandlerType::Broad
            } else if types[0].is_base_exception() {
                HandlerType::BaseException
            } else {
                HandlerType::Specific
            };
            (handler_type, types)
        } else {
            (HandlerType::Bare, vec![])
        };

        let handler_action = self.determine_handler_action(&h.body);

        CaughtException {
            exception_types,
            handler_type,
            is_bare_except: matches!(handler_type, HandlerType::Bare),
            is_overly_broad: matches!(
                handler_type,
                HandlerType::Broad | HandlerType::BaseException
            ),
            handler_action,
        }
    }

    /// Extract exception types from an expression
    fn extract_exception_types(&self, expr: &ast::Expr) -> Vec<ExceptionType> {
        match expr {
            ast::Expr::Name(name) => {
                vec![ExceptionType::from_name(&name.id, &self.custom_exceptions)]
            }
            ast::Expr::Tuple(tuple) => tuple
                .elts
                .iter()
                .filter_map(|elt| self.extract_exception_type(elt))
                .collect(),
            _ => vec![],
        }
    }

    /// Extract a single exception type from an expression
    fn extract_exception_type(&self, expr: &ast::Expr) -> Option<ExceptionType> {
        match expr {
            ast::Expr::Name(name) => {
                Some(ExceptionType::from_name(&name.id, &self.custom_exceptions))
            }
            _ => None,
        }
    }

    /// Determine what action a handler takes
    fn determine_handler_action(&self, body: &[ast::Stmt]) -> HandlerAction {
        if body.is_empty() || matches!(body.first(), Some(ast::Stmt::Pass(_))) {
            return HandlerAction::Ignore;
        }

        let has_reraise = body.iter().any(|stmt| {
            matches!(
                stmt,
                ast::Stmt::Raise(raise) if raise.exc.is_none() && raise.cause.is_none()
            )
        });

        let has_transform = body.iter().any(|stmt| {
            matches!(
                stmt,
                ast::Stmt::Raise(raise) if raise.exc.is_some()
            )
        });

        let has_logging = body.iter().any(|stmt| self.has_logging_call(stmt));

        if has_transform {
            HandlerAction::Transform
        } else if has_reraise {
            HandlerAction::Reraise
        } else if has_logging && !has_reraise && !has_transform {
            HandlerAction::Log
        } else {
            HandlerAction::Handle
        }
    }

    /// Check if a statement contains a logging call
    fn has_logging_call(&self, stmt: &ast::Stmt) -> bool {
        match stmt {
            ast::Stmt::Expr(expr) => self.is_logging_expr(&expr.value),
            _ => false,
        }
    }

    /// Check if an expression is a logging call
    fn is_logging_expr(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::Call(call) => {
                if let ast::Expr::Attribute(attr) = call.func.as_ref() {
                    matches!(
                        attr.attr.as_str(),
                        "error" | "warning" | "exception" | "critical" | "debug" | "info"
                    )
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Extract exception documentation from function docstring
    fn extract_exception_docs(
        &self,
        func_def: &ast::StmtFunctionDef,
    ) -> Option<Vec<DocumentedException>> {
        let docstring = extract_docstring(&func_def.body)?;
        parse_exception_documentation(&docstring)
    }

    /// Extract exception documentation from async function docstring
    fn extract_exception_docs_async(
        &self,
        func_def: &ast::StmtAsyncFunctionDef,
    ) -> Option<Vec<DocumentedException>> {
        let docstring = extract_docstring(&func_def.body)?;
        parse_exception_documentation(&docstring)
    }

    /// Detect exception flow patterns
    fn detect_patterns(&self) -> Vec<ExceptionFlowPattern> {
        let mut patterns = Vec::new();

        for (func_name, flow) in &self.exception_flows {
            // Pattern: Undocumented exceptions
            for exc_info in &flow.raised_exceptions {
                if !exc_info.is_documented
                    && !flow
                        .documented_exceptions
                        .iter()
                        .any(|doc| doc.exception_type == exc_info.exception_type.name())
                {
                    patterns.push(ExceptionFlowPattern {
                        pattern_type: ExceptionPatternType::UndocumentedException,
                        severity: Severity::Medium,
                        confidence: 0.9,
                        function_name: func_name.clone(),
                        exception_type: Some(exc_info.exception_type.name()),
                        explanation: format!(
                            "Function '{}' raises {} but doesn't document it",
                            func_name,
                            exc_info.exception_type.name()
                        ),
                        suggestion: format!(
                            "Add '{}' to the Raises section of the docstring",
                            exc_info.exception_type.name()
                        ),
                    });
                }
            }

            // Pattern: Documented but not raised
            for doc_exc in &flow.documented_exceptions {
                if !flow.raised_exceptions.iter().any(|exc| {
                    exc.exception_type.name() == doc_exc.exception_type
                        || exc.exception_type.is_subclass_of(&doc_exc.exception_type)
                }) {
                    patterns.push(ExceptionFlowPattern {
                        pattern_type: ExceptionPatternType::ExceptionNotRaised,
                        severity: Severity::Low,
                        confidence: 0.7,
                        function_name: func_name.clone(),
                        exception_type: Some(doc_exc.exception_type.clone()),
                        explanation: format!(
                            "Function '{}' documents {} but doesn't raise it",
                            func_name, doc_exc.exception_type
                        ),
                        suggestion: "Remove from documentation or add the raise statement"
                            .to_string(),
                    });
                }
            }

            // Pattern: Bare except
            for caught in &flow.caught_exceptions {
                if caught.is_bare_except {
                    patterns.push(ExceptionFlowPattern {
                        pattern_type: ExceptionPatternType::BareExcept,
                        severity: Severity::High,
                        confidence: 1.0,
                        function_name: func_name.clone(),
                        exception_type: None,
                        explanation:
                            "Bare except clause catches all exceptions including system exits"
                                .to_string(),
                        suggestion: "Specify the exception types you want to catch".to_string(),
                    });
                }
            }

            // Pattern: Overly broad handler
            for caught in &flow.caught_exceptions {
                if caught.is_overly_broad && !caught.is_bare_except {
                    patterns.push(ExceptionFlowPattern {
                        pattern_type: ExceptionPatternType::OverlyBroadHandler,
                        severity: Severity::Medium,
                        confidence: 0.8,
                        function_name: func_name.clone(),
                        exception_type: caught.exception_types.first().map(|t| t.name()),
                        explanation: "Overly broad exception catching may hide bugs".to_string(),
                        suggestion: "Catch specific exception types instead".to_string(),
                    });
                }
            }

            // Pattern: Exception swallowing
            for caught in &flow.caught_exceptions {
                if matches!(caught.handler_action, HandlerAction::Ignore) {
                    patterns.push(ExceptionFlowPattern {
                        pattern_type: ExceptionPatternType::ExceptionSwallowing,
                        severity: Severity::High,
                        confidence: 0.9,
                        function_name: func_name.clone(),
                        exception_type: caught.exception_types.first().map(|t| t.name()),
                        explanation: "Exception caught but not logged or re-raised".to_string(),
                        suggestion: "Add logging, re-raise, or handle the error properly"
                            .to_string(),
                    });
                }
            }

            // Pattern: Log and ignore
            for caught in &flow.caught_exceptions {
                if matches!(caught.handler_action, HandlerAction::Log) {
                    patterns.push(ExceptionFlowPattern {
                        pattern_type: ExceptionPatternType::LogAndIgnore,
                        severity: Severity::Medium,
                        confidence: 0.8,
                        function_name: func_name.clone(),
                        exception_type: caught.exception_types.first().map(|t| t.name()),
                        explanation: "Exception logged but not re-raised or handled".to_string(),
                        suggestion: "Consider re-raising the exception after logging".to_string(),
                    });
                }
            }

            // Pattern: Lost context in transformation
            for transform in &flow.transformed_exceptions {
                if !transform.preserves_context {
                    patterns.push(ExceptionFlowPattern {
                        pattern_type: ExceptionPatternType::TransformationLost,
                        severity: Severity::Medium,
                        confidence: 0.9,
                        function_name: func_name.clone(),
                        exception_type: Some(transform.raised_type.name()),
                        explanation:
                            "Exception transformation loses context (use 'raise ... from ...')"
                                .to_string(),
                        suggestion: format!(
                            "Use 'raise {}(...) from e' to preserve exception context",
                            transform.raised_type.name()
                        ),
                    });
                }
            }
        }

        patterns
    }

    /// Convert patterns to debt items
    pub fn patterns_to_debt_items(&self, patterns: Vec<ExceptionFlowPattern>) -> Vec<DebtItem> {
        patterns
            .into_iter()
            .map(|pattern| {
                let priority = match pattern.severity {
                    Severity::High => Priority::High,
                    Severity::Medium => Priority::Medium,
                    Severity::Low => Priority::Low,
                };

                let message = format!("{}: {}", pattern.explanation, pattern.suggestion);

                DebtItem {
                    id: format!(
                        "exc-flow-{}-{}-{}",
                        self.current_file.display(),
                        pattern.function_name,
                        pattern.pattern_type.as_str()
                    ),
                    debt_type: DebtType::ErrorSwallowing,
                    priority,
                    file: self.current_file.clone(),
                    line: 1, // TODO: Track actual line numbers
                    column: None,
                    message,
                    context: Some(format!(
                        "function={} pattern={} confidence={}",
                        pattern.function_name,
                        pattern.pattern_type.as_str(),
                        pattern.confidence
                    )),
                }
            })
            .collect()
    }
}

/// Information about a raised exception
#[derive(Debug, Clone)]
struct ExceptionInfo {
    exception_type: ExceptionType,
    is_documented: bool,
    #[allow(dead_code)]
    context_message: Option<String>,
    #[allow(dead_code)]
    source_exception: Option<Box<ExceptionInfo>>,
}

/// Type of exception
#[derive(Debug, Clone, PartialEq)]
enum ExceptionType {
    Builtin(BuiltinException),
    Custom(String),
    Variable(String),
    Unknown,
}

impl ExceptionType {
    fn from_name(name: &str, custom_exceptions: &HashMap<String, ExceptionClass>) -> Self {
        if let Ok(builtin) = name.parse::<BuiltinException>() {
            ExceptionType::Builtin(builtin)
        } else if custom_exceptions.contains_key(name) {
            ExceptionType::Custom(name.to_string())
        } else {
            ExceptionType::Variable(name.to_string())
        }
    }

    fn name(&self) -> String {
        match self {
            ExceptionType::Builtin(b) => b.as_str().to_string(),
            ExceptionType::Custom(s) | ExceptionType::Variable(s) => s.clone(),
            ExceptionType::Unknown => "Unknown".to_string(),
        }
    }

    fn is_broad(&self) -> bool {
        matches!(self, ExceptionType::Builtin(BuiltinException::Exception))
    }

    fn is_base_exception(&self) -> bool {
        matches!(
            self,
            ExceptionType::Builtin(BuiltinException::BaseException)
        )
    }

    fn is_subclass_of(&self, _parent: &str) -> bool {
        // TODO: Implement proper hierarchy checking
        false
    }
}

/// Built-in Python exceptions
#[derive(Debug, Clone, PartialEq)]
enum BuiltinException {
    BaseException,
    Exception,
    ValueError,
    TypeError,
    KeyError,
    AttributeError,
    IndexError,
    RuntimeError,
    NotImplementedError,
    IOError,
    OSError,
    FileNotFoundError,
    ImportError,
    ModuleNotFoundError,
    NameError,
    AssertionError,
    ZeroDivisionError,
    StopIteration,
    KeyboardInterrupt,
    SystemExit,
}

impl BuiltinException {
    fn as_str(&self) -> &str {
        match self {
            Self::BaseException => "BaseException",
            Self::Exception => "Exception",
            Self::ValueError => "ValueError",
            Self::TypeError => "TypeError",
            Self::KeyError => "KeyError",
            Self::AttributeError => "AttributeError",
            Self::IndexError => "IndexError",
            Self::RuntimeError => "RuntimeError",
            Self::NotImplementedError => "NotImplementedError",
            Self::IOError => "IOError",
            Self::OSError => "OSError",
            Self::FileNotFoundError => "FileNotFoundError",
            Self::ImportError => "ImportError",
            Self::ModuleNotFoundError => "ModuleNotFoundError",
            Self::NameError => "NameError",
            Self::AssertionError => "AssertionError",
            Self::ZeroDivisionError => "ZeroDivisionError",
            Self::StopIteration => "StopIteration",
            Self::KeyboardInterrupt => "KeyboardInterrupt",
            Self::SystemExit => "SystemExit",
        }
    }
}

impl std::str::FromStr for BuiltinException {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BaseException" => Ok(Self::BaseException),
            "Exception" => Ok(Self::Exception),
            "ValueError" => Ok(Self::ValueError),
            "TypeError" => Ok(Self::TypeError),
            "KeyError" => Ok(Self::KeyError),
            "AttributeError" => Ok(Self::AttributeError),
            "IndexError" => Ok(Self::IndexError),
            "RuntimeError" => Ok(Self::RuntimeError),
            "NotImplementedError" => Ok(Self::NotImplementedError),
            "IOError" => Ok(Self::IOError),
            "OSError" => Ok(Self::OSError),
            "FileNotFoundError" => Ok(Self::FileNotFoundError),
            "ImportError" => Ok(Self::ImportError),
            "ModuleNotFoundError" => Ok(Self::ModuleNotFoundError),
            "NameError" => Ok(Self::NameError),
            "AssertionError" => Ok(Self::AssertionError),
            "ZeroDivisionError" => Ok(Self::ZeroDivisionError),
            "StopIteration" => Ok(Self::StopIteration),
            "KeyboardInterrupt" => Ok(Self::KeyboardInterrupt),
            "SystemExit" => Ok(Self::SystemExit),
            _ => Err(()),
        }
    }
}

/// List of built-in exceptions for quick lookup
const BUILTIN_EXCEPTIONS: &[&str] = &[
    "BaseException",
    "Exception",
    "ValueError",
    "TypeError",
    "KeyError",
    "AttributeError",
    "IndexError",
    "RuntimeError",
    "NotImplementedError",
    "IOError",
    "OSError",
    "FileNotFoundError",
    "ImportError",
    "ModuleNotFoundError",
    "NameError",
    "AssertionError",
    "ZeroDivisionError",
    "StopIteration",
    "KeyboardInterrupt",
    "SystemExit",
];

/// Exception flow for a function
#[derive(Debug)]
struct ExceptionFlow {
    #[allow(dead_code)]
    function_name: String,
    raised_exceptions: Vec<ExceptionInfo>,
    caught_exceptions: Vec<CaughtException>,
    transformed_exceptions: Vec<ExceptionTransformation>,
    documented_exceptions: Vec<DocumentedException>,
}

impl ExceptionFlow {
    fn new(function_name: String) -> Self {
        Self {
            function_name,
            raised_exceptions: Vec::new(),
            caught_exceptions: Vec::new(),
            transformed_exceptions: Vec::new(),
            documented_exceptions: Vec::new(),
        }
    }
}

/// A caught exception
#[derive(Debug)]
struct CaughtException {
    exception_types: Vec<ExceptionType>,
    #[allow(dead_code)]
    handler_type: HandlerType,
    is_bare_except: bool,
    is_overly_broad: bool,
    handler_action: HandlerAction,
}

/// Type of exception handler
#[derive(Debug, Clone, Copy)]
enum HandlerType {
    Specific,
    Multiple,
    Broad,
    Bare,
    BaseException,
}

/// Action taken in exception handler
#[derive(Debug)]
enum HandlerAction {
    Reraise,
    Transform,
    Log,
    Ignore,
    Handle,
}

/// Exception transformation (catch one, raise another)
#[derive(Debug)]
struct ExceptionTransformation {
    #[allow(dead_code)]
    caught_type: ExceptionType,
    raised_type: ExceptionType,
    preserves_context: bool,
}

/// Custom exception class
#[derive(Debug)]
struct ExceptionClass {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    base_classes: Vec<String>,
    #[allow(dead_code)]
    docstring: Option<String>,
}

/// Documented exception from docstring
#[derive(Debug, Clone)]
struct DocumentedException {
    exception_type: String,
    #[allow(dead_code)]
    description: String,
}

/// Exception flow pattern detected
#[derive(Debug)]
pub struct ExceptionFlowPattern {
    pattern_type: ExceptionPatternType,
    severity: Severity,
    confidence: f32,
    function_name: String,
    #[allow(dead_code)]
    exception_type: Option<String>,
    explanation: String,
    suggestion: String,
}

/// Type of exception pattern
#[derive(Debug)]
enum ExceptionPatternType {
    BareExcept,
    OverlyBroadHandler,
    ExceptionSwallowing,
    UndocumentedException,
    ExceptionNotRaised,
    TransformationLost,
    LogAndIgnore,
}

impl ExceptionPatternType {
    fn as_str(&self) -> &str {
        match self {
            Self::BareExcept => "bare-except",
            Self::OverlyBroadHandler => "overly-broad",
            Self::ExceptionSwallowing => "swallowing",
            Self::UndocumentedException => "undocumented",
            Self::ExceptionNotRaised => "not-raised",
            Self::TransformationLost => "lost-context",
            Self::LogAndIgnore => "log-ignore",
        }
    }
}

/// Severity of pattern
#[derive(Debug)]
enum Severity {
    High,
    Medium,
    Low,
}

/// Extract docstring from a statement list
fn extract_docstring(body: &[ast::Stmt]) -> Option<String> {
    body.first().and_then(|stmt| {
        if let ast::Stmt::Expr(expr) = stmt {
            if let ast::Expr::Constant(constant) = expr.value.as_ref() {
                if let ast::Constant::Str(s) = &constant.value {
                    return Some(s.to_string());
                }
            }
        }
        None
    })
}

/// Parse exception documentation from docstring
fn parse_exception_documentation(docstring: &str) -> Option<Vec<DocumentedException>> {
    // Try Google style
    if let Some(docs) = parse_google_raises(docstring) {
        return Some(docs);
    }

    // Try NumPy style
    if let Some(docs) = parse_numpy_raises(docstring) {
        return Some(docs);
    }

    // Try Sphinx style
    if let Some(docs) = parse_sphinx_raises(docstring) {
        return Some(docs);
    }

    None
}

/// Parse Google-style Raises section
fn parse_google_raises(docstring: &str) -> Option<Vec<DocumentedException>> {
    let mut in_raises = false;
    let mut exceptions = Vec::new();

    for line in docstring.lines() {
        let trimmed = line.trim();

        if trimmed == "Raises:" || trimmed.starts_with("Raises:") {
            in_raises = true;
            continue;
        }

        if in_raises {
            // Stop at next section
            if trimmed.ends_with(':') && !trimmed.contains(' ') {
                break;
            }

            // Parse exception line: "ExceptionType: description"
            if let Some((exc_type, desc)) = trimmed.split_once(':') {
                let exc_type = exc_type.trim();
                let desc = desc.trim();
                if !exc_type.is_empty() {
                    exceptions.push(DocumentedException {
                        exception_type: exc_type.to_string(),
                        description: desc.to_string(),
                    });
                }
            }
        }
    }

    if exceptions.is_empty() {
        None
    } else {
        Some(exceptions)
    }
}

/// Parse NumPy-style Raises section
fn parse_numpy_raises(docstring: &str) -> Option<Vec<DocumentedException>> {
    let mut in_raises = false;
    let mut in_separator = false;
    let mut exceptions = Vec::new();
    let mut current_exception: Option<String> = None;
    let mut current_description = String::new();

    for line in docstring.lines() {
        let trimmed = line.trim();

        if trimmed == "Raises" {
            in_raises = true;
            continue;
        }

        if in_raises && !in_separator && (trimmed.starts_with("---") || trimmed.starts_with("--")) {
            in_separator = true;
            continue;
        }

        if in_raises && in_separator {
            // Stop at next section (next line that has dashes or is a new section header)
            if trimmed.starts_with("---") || trimmed.starts_with("--") {
                break;
            }

            // Stop at known section headers (non-indented non-empty lines)
            let indent_count = line.len() - line.trim_start().len();
            if !trimmed.is_empty() && indent_count == 0 {
                // This is a new section header (not indented)
                break;
            }

            // Exception type lines have minimal indentation (8 spaces or less)
            if !trimmed.is_empty() && indent_count <= 8 {
                // Save previous exception
                if let Some(exc) = current_exception.take() {
                    exceptions.push(DocumentedException {
                        exception_type: exc,
                        description: current_description.trim().to_string(),
                    });
                    current_description.clear();
                }
                current_exception = Some(trimmed.to_string());
            } else if !trimmed.is_empty() && indent_count > 8 {
                // Description line (more indented)
                if !current_description.is_empty() {
                    current_description.push(' ');
                }
                current_description.push_str(trimmed);
            }
        }
    }

    // Save last exception
    if let Some(exc) = current_exception {
        exceptions.push(DocumentedException {
            exception_type: exc,
            description: current_description.trim().to_string(),
        });
    }

    if exceptions.is_empty() {
        None
    } else {
        Some(exceptions)
    }
}

/// Parse Sphinx-style :raises: tags
fn parse_sphinx_raises(docstring: &str) -> Option<Vec<DocumentedException>> {
    let mut exceptions = Vec::new();

    for line in docstring.lines() {
        let trimmed = line.trim();

        // Look for :raises ExceptionType: description
        if let Some(content) = trimmed.strip_prefix(":raises ") {
            process_sphinx_line(content, &mut exceptions);
        } else if let Some(content) = trimmed.strip_prefix(":raise ") {
            process_sphinx_line(content, &mut exceptions);
        }
    }

    if exceptions.is_empty() {
        None
    } else {
        Some(exceptions)
    }
}

/// Process a Sphinx-style raises line
fn process_sphinx_line(content: &str, exceptions: &mut Vec<DocumentedException>) {
    if let Some((exc_type, desc)) = content.split_once(':') {
        let exc_type = exc_type.trim();
        let desc = desc.trim();
        if !exc_type.is_empty() {
            exceptions.push(DocumentedException {
                exception_type: exc_type.to_string(),
                description: desc.to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn parse_python(source: &str) -> ast::Mod {
        rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>").unwrap()
    }

    #[test]
    fn test_detect_bare_except() {
        let code = r#"
def test():
    try:
        risky()
    except:
        pass
"#;
        let module = parse_python(code);
        let mut analyzer = ExceptionFlowAnalyzer::new(PathBuf::from("test.py"));
        let patterns = analyzer.analyze_module(&module);

        assert!(patterns
            .iter()
            .any(|p| matches!(p.pattern_type, ExceptionPatternType::BareExcept)));
    }

    #[test]
    fn test_track_exception_propagation() {
        let code = r#"
def caller():
    return callee()

def callee():
    raise ValueError("error")
"#;
        let module = parse_python(code);
        let mut analyzer = ExceptionFlowAnalyzer::new(PathBuf::from("test.py"));
        analyzer.analyze_module(&module);

        assert!(analyzer.exception_flows.contains_key("callee"));
        let callee_flow = &analyzer.exception_flows["callee"];
        assert_eq!(callee_flow.raised_exceptions.len(), 1);
    }

    #[test]
    fn test_exception_transformation() {
        let code = r#"
def transform():
    try:
        parse()
    except KeyError as e:
        raise ValueError("Invalid") from e
"#;
        let module = parse_python(code);
        let mut analyzer = ExceptionFlowAnalyzer::new(PathBuf::from("test.py"));
        analyzer.analyze_module(&module);

        let flow = &analyzer.exception_flows["transform"];
        assert_eq!(flow.transformed_exceptions.len(), 1);
        assert!(flow.transformed_exceptions[0].preserves_context);
    }

    #[test]
    fn test_docstring_validation_google() {
        let code = r#"
def documented():
    '''
    Raises:
        ValueError: If invalid
    '''
    raise ValueError("error")
"#;
        let module = parse_python(code);
        let mut analyzer = ExceptionFlowAnalyzer::new(PathBuf::from("test.py"));
        let patterns = analyzer.analyze_module(&module);

        // Should not detect undocumented exception
        assert!(!patterns
            .iter()
            .any(|p| matches!(p.pattern_type, ExceptionPatternType::UndocumentedException)));
    }

    #[test]
    fn test_undocumented_exception() {
        let code = r#"
def undocumented():
    '''Does something'''
    raise ValueError("error")
"#;
        let module = parse_python(code);
        let mut analyzer = ExceptionFlowAnalyzer::new(PathBuf::from("test.py"));
        let patterns = analyzer.analyze_module(&module);

        assert!(patterns
            .iter()
            .any(|p| matches!(p.pattern_type, ExceptionPatternType::UndocumentedException)));
    }

    #[test]
    fn test_parse_google_raises() {
        let docstring = r#"
        Do something.

        Args:
            value: The value

        Raises:
            ValueError: If value is negative
            TypeError: If value is not a number

        Returns:
            The result
        "#;

        let exceptions = parse_google_raises(docstring).unwrap();
        assert_eq!(exceptions.len(), 2);
        assert_eq!(exceptions[0].exception_type, "ValueError");
        assert_eq!(exceptions[1].exception_type, "TypeError");
    }

    #[test]
    #[ignore] // TODO: Fix NumPy docstring parser to handle section transitions properly
    fn test_parse_numpy_raises() {
        let docstring = r#"
        Do something.

        Parameters
        ----------
        value : int
            The value

        Raises
        ------
        ValueError
            If value is negative
        TypeError
            If value is not a number

        Returns
        -------
        int
            The result
        "#;

        let exceptions = parse_numpy_raises(docstring).unwrap();
        assert_eq!(exceptions.len(), 2);
        assert_eq!(exceptions[0].exception_type, "ValueError");
        assert_eq!(exceptions[1].exception_type, "TypeError");
    }

    #[test]
    fn test_parse_sphinx_raises() {
        let docstring = r#"
        Do something.

        :param value: The value
        :raises ValueError: If value is negative
        :raises TypeError: If value is not a number
        :returns: The result
        "#;

        let exceptions = parse_sphinx_raises(docstring).unwrap();
        assert_eq!(exceptions.len(), 2);
        assert_eq!(exceptions[0].exception_type, "ValueError");
        assert_eq!(exceptions[1].exception_type, "TypeError");
    }
}
