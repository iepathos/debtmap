//! Asyncio pattern detection for Python
//!
//! Detects asyncio-specific error patterns that can lead to silent failures,
//! resource leaks, and subtle bugs in async applications.

use rustpython_parser::ast;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::core::{DebtItem, DebtType, Priority};

/// Main detector for asyncio patterns
#[derive(Debug)]
pub struct AsyncioPatternDetector {
    task_registry: HashMap<String, TaskInfo>,
    resource_registry: HashMap<String, AsyncResourceInfo>,
    blocking_operations: HashSet<&'static str>,
    file_path: PathBuf,
    in_async_context: bool,
    current_line: usize,
}

/// Information about a tracked task
#[derive(Debug, Clone)]
struct TaskInfo {
    creation_line: usize,
    is_awaited: bool,
    has_exception_handling: bool,
    has_timeout: bool,
    task_type: TaskType,
}

/// Type of async task
#[derive(Debug, Clone, Copy, PartialEq)]
enum TaskType {
    CreateTask,
    Gather,
    WaitFor,
    RunInExecutor,
    Shield,
}

/// Information about an async resource
#[derive(Debug, Clone)]
struct AsyncResourceInfo {
    resource_type: AsyncResourceType,
    creation_line: usize,
    has_cleanup: bool,
    context_manager_used: bool,
    lifecycle: ResourceLifecycle,
}

/// Type of async resource
#[derive(Debug, Clone, Copy, PartialEq)]
enum AsyncResourceType {
    ClientSession,
    StreamWriter,
    StreamReader,
    Database,
    WebSocket,
    AsyncGenerator,
    Lock,
}

/// Lifecycle state of a resource
#[derive(Debug, Clone, Copy, PartialEq)]
enum ResourceLifecycle {
    Created,
    InUse,
    Closed,
    Leaked,
}

/// Detected async error pattern
#[derive(Debug, Clone)]
pub struct AsyncErrorPattern {
    pattern_type: AsyncErrorType,
    pub line: usize,
    pub confidence: f32,
    pub explanation: String,
    pub fix_suggestion: String,
}

/// Types of async errors
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AsyncErrorType {
    UnhandledTaskException,
    FireAndForget,
    AsyncResourceLeak,
    BlockingInAsyncContext,
    MissingTimeout,
    ImproperCancellation,
    UnboundedTaskCreation,
    MissingAwait,
    AwaitableNotAwaited,
    EventLoopMisuse,
    CancelledErrorSwallowed,
}

impl AsyncioPatternDetector {
    /// Create a new detector for a file
    pub fn new(file_path: PathBuf) -> Self {
        let mut blocking_operations = HashSet::new();
        blocking_operations.insert("time.sleep");
        blocking_operations.insert("open");
        blocking_operations.insert("urllib.request");
        blocking_operations.insert("requests.get");
        blocking_operations.insert("requests.post");
        blocking_operations.insert("os.system");
        blocking_operations.insert("subprocess.run");
        blocking_operations.insert("json.load");

        Self {
            task_registry: HashMap::new(),
            resource_registry: HashMap::new(),
            blocking_operations,
            file_path,
            in_async_context: false,
            current_line: 1,
        }
    }

    /// Analyze a Python module for asyncio patterns
    pub fn analyze_module(&mut self, module: &ast::Mod) -> Vec<DebtItem> {
        let ast::Mod::Module(mod_ast) = module else {
            return Vec::new();
        };

        let mut patterns = Vec::new();

        for stmt in &mod_ast.body {
            patterns.extend(self.analyze_stmt(stmt));
        }

        convert_patterns_to_debt_items(patterns, &self.file_path)
    }

    /// Analyze a statement
    fn analyze_stmt(&mut self, stmt: &ast::Stmt) -> Vec<AsyncErrorPattern> {
        match stmt {
            ast::Stmt::AsyncFunctionDef(func_def) => self.analyze_async_function(func_def),
            ast::Stmt::FunctionDef(func_def) => self.analyze_sync_function(func_def),
            ast::Stmt::ClassDef(class_def) => self.analyze_class(class_def),
            _ => Vec::new(),
        }
    }

    /// Analyze an async function
    fn analyze_async_function(
        &mut self,
        func_def: &ast::StmtAsyncFunctionDef,
    ) -> Vec<AsyncErrorPattern> {
        let was_in_async = self.in_async_context;
        self.in_async_context = true;

        let mut patterns = Vec::new();
        patterns.extend(self.analyze_function_body(&func_def.body));

        self.in_async_context = was_in_async;
        patterns
    }

    /// Analyze a sync function (should not have async calls)
    fn analyze_sync_function(&mut self, func_def: &ast::StmtFunctionDef) -> Vec<AsyncErrorPattern> {
        let was_in_async = self.in_async_context;
        self.in_async_context = false;

        let patterns = self.analyze_function_body(&func_def.body);

        self.in_async_context = was_in_async;
        patterns
    }

    /// Analyze a class definition
    fn analyze_class(&mut self, class_def: &ast::StmtClassDef) -> Vec<AsyncErrorPattern> {
        let mut patterns = Vec::new();

        for stmt in &class_def.body {
            patterns.extend(self.analyze_stmt(stmt));
        }

        patterns
    }

    /// Analyze function body statements
    fn analyze_function_body(&mut self, body: &[ast::Stmt]) -> Vec<AsyncErrorPattern> {
        let mut patterns = Vec::new();

        for stmt in body {
            patterns.extend(self.analyze_body_stmt(stmt));
        }

        patterns
    }

    /// Analyze a statement in a function body
    fn analyze_body_stmt(&mut self, stmt: &ast::Stmt) -> Vec<AsyncErrorPattern> {
        let mut patterns = Vec::new();
        self.current_line += 1; // Track line numbers

        match stmt {
            ast::Stmt::Expr(expr_stmt) => {
                patterns.extend(self.analyze_expr(&expr_stmt.value));
            }
            ast::Stmt::Assign(assign) => {
                patterns.extend(self.analyze_assign(assign));
            }
            ast::Stmt::For(for_stmt) => {
                patterns.extend(self.analyze_for_loop(for_stmt));
                patterns.extend(self.analyze_function_body(&for_stmt.body));
            }
            ast::Stmt::While(while_stmt) => {
                patterns.extend(self.analyze_function_body(&while_stmt.body));
            }
            ast::Stmt::If(if_stmt) => {
                patterns.extend(self.analyze_function_body(&if_stmt.body));
                patterns.extend(self.analyze_function_body(&if_stmt.orelse));
            }
            ast::Stmt::With(with_stmt) => {
                patterns.extend(self.analyze_with_stmt(with_stmt));
            }
            ast::Stmt::AsyncWith(async_with) => {
                patterns.extend(self.analyze_async_with(async_with));
            }
            ast::Stmt::Try(try_stmt) => {
                patterns.extend(self.analyze_try_stmt(try_stmt));
            }
            _ => {}
        }

        patterns
    }

    /// Analyze an expression
    fn analyze_expr(&mut self, expr: &ast::Expr) -> Vec<AsyncErrorPattern> {
        match expr {
            ast::Expr::Call(call) => self.analyze_call(call),
            ast::Expr::Await(await_expr) => self.analyze_await(await_expr),
            _ => Vec::new(),
        }
    }

    /// Analyze a function call
    fn analyze_call(&mut self, call: &ast::ExprCall) -> Vec<AsyncErrorPattern> {
        let mut patterns = Vec::new();

        if let Some(call_name) = extract_call_name(&call.func) {
            // Detect asyncio.create_task without exception handling
            if call_name == "asyncio.create_task" || call_name == "create_task" {
                patterns.extend(self.detect_unhandled_task(call));
            }

            // Detect asyncio.gather without return_exceptions
            if call_name == "asyncio.gather" || call_name == "gather" {
                patterns.extend(self.detect_gather_without_exception_handling(call));
            }

            // Detect blocking operations in async context
            if self.in_async_context && self.is_blocking_operation(&call_name) {
                patterns.push(AsyncErrorPattern {
                    pattern_type: AsyncErrorType::BlockingInAsyncContext,
                    line: self.current_line,
                    confidence: 0.9,
                    explanation: format!(
                        "Blocking operation '{}' called in async function",
                        call_name
                    ),
                    fix_suggestion: format!(
                        "Use async alternative or run in executor: await asyncio.to_thread({})",
                        call_name
                    ),
                });
            }

            // Detect resource creation without context manager
            if let Some(resource_type) = self.identify_async_resource(&call_name) {
                patterns.extend(self.detect_resource_without_context_manager(call, resource_type));
            }
        }

        patterns
    }

    /// Detect unhandled task exceptions
    fn detect_unhandled_task(&self, _call: &ast::ExprCall) -> Vec<AsyncErrorPattern> {
        // This is a fire-and-forget pattern if the task is not assigned or awaited
        vec![AsyncErrorPattern {
            pattern_type: AsyncErrorType::UnhandledTaskException,
            line: self.current_line,
            confidence: 0.85,
            explanation: "asyncio.create_task() without exception handling can lead to silent failures".to_string(),
            fix_suggestion: "Store task reference and add exception handling: task = asyncio.create_task(...); task.add_done_callback(lambda t: t.result())".to_string(),
        }]
    }

    /// Detect asyncio.gather without return_exceptions=True
    fn detect_gather_without_exception_handling(
        &self,
        call: &ast::ExprCall,
    ) -> Vec<AsyncErrorPattern> {
        let has_return_exceptions = call.keywords.iter().any(|kw| {
            kw.arg
                .as_ref()
                .map_or(false, |arg| <ast::Identifier as AsRef<str>>::as_ref(arg) == "return_exceptions")
        });

        if !has_return_exceptions {
            vec![AsyncErrorPattern {
                pattern_type: AsyncErrorType::UnhandledTaskException,
                line: self.current_line,
                confidence: 0.75,
                explanation: "asyncio.gather() without return_exceptions=True will cancel all tasks if one fails".to_string(),
                fix_suggestion: "Add return_exceptions=True: await asyncio.gather(*tasks, return_exceptions=True)".to_string(),
            }]
        } else {
            Vec::new()
        }
    }

    /// Detect resource creation without context manager
    fn detect_resource_without_context_manager(
        &self,
        _call: &ast::ExprCall,
        resource_type: AsyncResourceType,
    ) -> Vec<AsyncErrorPattern> {
        let resource_name = match resource_type {
            AsyncResourceType::ClientSession => "aiohttp.ClientSession",
            AsyncResourceType::StreamWriter => "asyncio.StreamWriter",
            AsyncResourceType::Database => "Database connection",
            _ => "Async resource",
        };

        vec![AsyncErrorPattern {
            pattern_type: AsyncErrorType::AsyncResourceLeak,
            line: self.current_line,
            confidence: 0.8,
            explanation: format!(
                "{} created without async context manager may leak",
                resource_name
            ),
            fix_suggestion: format!(
                "Use async with: async with {}(...) as resource: ...",
                resource_name
            ),
        }]
    }

    /// Analyze assignment for task tracking
    fn analyze_assign(&mut self, assign: &ast::StmtAssign) -> Vec<AsyncErrorPattern> {
        // Track if assigned value is a task creation
        self.analyze_expr(&assign.value)
    }

    /// Analyze for loop for unbounded task creation
    fn analyze_for_loop(&self, for_stmt: &ast::StmtFor) -> Vec<AsyncErrorPattern> {
        let mut patterns = Vec::new();

        // Check if loop body creates tasks without limit
        for stmt in &for_stmt.body {
            if self.contains_task_creation(stmt) {
                patterns.push(AsyncErrorPattern {
                    pattern_type: AsyncErrorType::UnboundedTaskCreation,
                    line: self.current_line,
                    confidence: 0.7,
                    explanation: "Loop creates tasks without concurrency limit, may cause memory issues".to_string(),
                    fix_suggestion: "Use asyncio.Semaphore to limit concurrent tasks: sem = asyncio.Semaphore(10); async with sem: ...".to_string(),
                });
                break;
            }
        }

        patterns
    }

    /// Analyze with statement
    fn analyze_with_stmt(&mut self, with_stmt: &ast::StmtWith) -> Vec<AsyncErrorPattern> {
        self.analyze_function_body(&with_stmt.body)
    }

    /// Analyze async with statement
    fn analyze_async_with(&mut self, async_with: &ast::StmtAsyncWith) -> Vec<AsyncErrorPattern> {
        // Async with is the correct pattern, analyze body
        self.analyze_function_body(&async_with.body)
    }

    /// Analyze try statement for cancellation handling
    fn analyze_try_stmt(&self, try_stmt: &ast::StmtTry) -> Vec<AsyncErrorPattern> {
        let mut patterns = Vec::new();

        // Check for swallowed CancelledError
        for handler in &try_stmt.handlers {
            if self.swallows_cancelled_error(handler) {
                patterns.push(AsyncErrorPattern {
                    pattern_type: AsyncErrorType::CancelledErrorSwallowed,
                    line: self.current_line,
                    confidence: 0.9,
                    explanation: "Bare except or Exception catch may swallow CancelledError".to_string(),
                    fix_suggestion: "Catch specific exceptions or re-raise CancelledError: except Exception: if isinstance(e, asyncio.CancelledError): raise".to_string(),
                });
            }
        }

        patterns
    }

    /// Analyze await expression
    fn analyze_await(&mut self, await_expr: &ast::ExprAwait) -> Vec<AsyncErrorPattern> {
        // Analyze the expression being awaited
        self.analyze_expr(&await_expr.value)
    }

    /// Check if a statement contains task creation
    fn contains_task_creation(&self, stmt: &ast::Stmt) -> bool {
        match stmt {
            ast::Stmt::Expr(expr_stmt) => self.is_task_creation_expr(&expr_stmt.value),
            ast::Stmt::Assign(assign) => self.is_task_creation_expr(&assign.value),
            _ => false,
        }
    }

    /// Check if an expression is a task creation
    fn is_task_creation_expr(&self, expr: &ast::Expr) -> bool {
        if let ast::Expr::Call(call) = expr {
            if let Some(name) = extract_call_name(&call.func) {
                return name == "asyncio.create_task" || name == "create_task";
            }
        }
        false
    }

    /// Check if operation is blocking
    fn is_blocking_operation(&self, call_name: &str) -> bool {
        self.blocking_operations.contains(call_name)
    }

    /// Identify async resource type from call name
    fn identify_async_resource(&self, call_name: &str) -> Option<AsyncResourceType> {
        if call_name.contains("ClientSession") {
            Some(AsyncResourceType::ClientSession)
        } else if call_name.contains("StreamWriter") {
            Some(AsyncResourceType::StreamWriter)
        } else if call_name.contains("asyncpg") || call_name.contains("aiomysql") {
            Some(AsyncResourceType::Database)
        } else {
            None
        }
    }

    /// Check if exception handler swallows CancelledError
    fn swallows_cancelled_error(&self, handler: &ast::ExceptHandler) -> bool {
        // Match on the exception handler variant
        let ast::ExceptHandler::ExceptHandler(h) = handler;

        // Bare except
        if h.type_.is_none() {
            return true;
        }

        // Check if catching Exception
        if let Some(exc_type) = &h.type_ {
            if let ast::Expr::Name(name) = exc_type.as_ref() {
                return <ast::Identifier as AsRef<str>>::as_ref(&name.id) == "Exception";
            }
        }

        false
    }
}

/// Extract call name from expression
fn extract_call_name(expr: &ast::Expr) -> Option<String> {
    match expr {
        ast::Expr::Name(name) => Some(name.id.to_string()),
        ast::Expr::Attribute(attr) => {
            let value_name = extract_call_name(&attr.value)?;
            Some(format!("{}.{}", value_name, attr.attr))
        }
        _ => None,
    }
}

/// Convert detected patterns to debt items
fn convert_patterns_to_debt_items(patterns: Vec<AsyncErrorPattern>, path: &Path) -> Vec<DebtItem> {
    patterns
        .into_iter()
        .map(|pattern| DebtItem {
            id: format!("asyncio-{:?}-{}", pattern.pattern_type, pattern.line),
            file: path.to_path_buf(),
            line: pattern.line,
            column: None,
            debt_type: match pattern.pattern_type {
                AsyncErrorType::UnhandledTaskException
                | AsyncErrorType::FireAndForget
                | AsyncErrorType::ImproperCancellation
                | AsyncErrorType::CancelledErrorSwallowed => DebtType::ErrorSwallowing,
                AsyncErrorType::AsyncResourceLeak => DebtType::ResourceManagement,
                AsyncErrorType::BlockingInAsyncContext | AsyncErrorType::EventLoopMisuse => {
                    DebtType::CodeSmell
                }
                AsyncErrorType::UnboundedTaskCreation => DebtType::Complexity,
                AsyncErrorType::MissingTimeout
                | AsyncErrorType::MissingAwait
                | AsyncErrorType::AwaitableNotAwaited => DebtType::CodeSmell,
            },
            message: format!("{}: {}", pattern.explanation, pattern.fix_suggestion),
            priority: if pattern.confidence > 0.8 {
                Priority::High
            } else if pattern.confidence > 0.6 {
                Priority::Medium
            } else {
                Priority::Low
            },
            context: Some(format!(
                "asyncio-pattern (confidence: {:.0}%)",
                pattern.confidence * 100.0
            )),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze_code(code: &str) -> Vec<DebtItem> {
        let module = rustpython_parser::parse(code, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse test code");
        let mut detector = AsyncioPatternDetector::new(PathBuf::from("test.py"));
        detector.analyze_module(&module)
    }

    #[test]
    fn test_detect_unhandled_task_exception() {
        let code = r#"
async def test():
    asyncio.create_task(risky_operation())
"#;
        let patterns = analyze_code(code);
        assert!(
            patterns
                .iter()
                .any(|p| p.debt_type == DebtType::ErrorSwallowing),
            "Should detect unhandled task exception"
        );
    }

    #[test]
    fn test_detect_blocking_in_async() {
        let code = r#"
async def test():
    time.sleep(5)
"#;
        let patterns = analyze_code(code);
        assert!(
            patterns.iter().any(|p| p.debt_type == DebtType::CodeSmell),
            "Should detect blocking operation in async context"
        );
    }

    #[test]
    fn test_detect_gather_without_return_exceptions() {
        let code = r#"
async def test():
    await asyncio.gather(task1(), task2())
"#;
        let patterns = analyze_code(code);
        assert!(
            patterns
                .iter()
                .any(|p| p.message.contains("return_exceptions")),
            "Should detect gather without return_exceptions"
        );
    }

    #[test]
    fn test_proper_async_with_no_detection() {
        let code = r#"
async def test():
    async with aiohttp.ClientSession() as session:
        await session.get("https://example.com")
"#;
        let patterns = analyze_code(code);
        // Should not detect resource leak when using context manager
        assert!(
            !patterns
                .iter()
                .any(|p| p.debt_type == DebtType::ResourceManagement),
            "Should not detect resource leak with async context manager"
        );
    }

    #[test]
    fn test_unbounded_task_creation() {
        let code = r#"
async def test():
    for item in large_list:
        asyncio.create_task(process(item))
"#;
        let patterns = analyze_code(code);
        assert!(
            patterns
                .iter()
                .any(|p| p.message.contains("unbounded") || p.message.contains("Semaphore")),
            "Should detect unbounded task creation in loop"
        );
    }

    #[test]
    fn test_swallowed_cancelled_error() {
        let code = r#"
async def test():
    try:
        await some_operation()
    except Exception:
        pass
"#;
        let patterns = analyze_code(code);
        assert!(
            patterns
                .iter()
                .any(|p| p.message.contains("CancelledError")),
            "Should detect swallowed CancelledError"
        );
    }
}
