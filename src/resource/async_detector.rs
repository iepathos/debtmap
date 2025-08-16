use super::{
    AsyncResourceIssueType, CancellationSafety, ResourceDetector, ResourceImpact,
    ResourceManagementIssue, ResourceType, SourceLocation,
};
use std::path::Path;
use syn::{visit::Visit, Expr, ExprAwait, ExprCall, ExprMethodCall, ItemFn, Stmt};

pub struct AsyncResourceDetector {
    cancellation_analyzer: CancellationAnalyzer,
}

impl Default for AsyncResourceDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncResourceDetector {
    pub fn new() -> Self {
        Self {
            cancellation_analyzer: CancellationAnalyzer::new(),
        }
    }

    fn analyze_async_resource_usage(&self, async_fn: &AsyncFunction) -> AsyncResourceUsage {
        let mut usage = AsyncResourceUsage::default();

        // Track resource acquisition and cleanup across await points
        let await_points = self.find_await_points(async_fn);
        let resource_operations = self.find_resource_operations(async_fn);

        for resource_op in resource_operations {
            let cancellation_analysis = self
                .cancellation_analyzer
                .analyze_resource_cancellation_safety(&resource_op, &await_points);

            if !cancellation_analysis.is_safe {
                usage.issues.push(AsyncResourceIssueInfo {
                    issue_type: AsyncResourceIssueType::CancellationUnsafe,
                    cancellation_safety: CancellationSafety::Unsafe,
                    mitigation_strategy: self.suggest_cancellation_mitigation(&resource_op),
                    location: resource_op.location.clone(),
                });
            }
        }

        // Check for Drop implementations in async context
        let drop_calls = self.find_drop_calls_in_async(async_fn);
        for drop_call in drop_calls {
            usage.issues.push(AsyncResourceIssueInfo {
                issue_type: AsyncResourceIssueType::DropInAsync,
                cancellation_safety: CancellationSafety::Unknown,
                mitigation_strategy: "Move resource cleanup outside async context".to_string(),
                location: drop_call.location,
            });
        }

        usage
    }

    fn find_await_points(&self, async_fn: &AsyncFunction) -> Vec<AwaitPoint> {
        let mut await_points = Vec::new();
        let mut visitor = AwaitVisitor::new();

        for stmt in &async_fn.stmts {
            visitor.visit_stmt(stmt);
        }

        for (expr, line) in visitor.await_exprs {
            await_points.push(AwaitPoint {
                location: SourceLocation {
                    file: String::new(),
                    line,
                    column: 0,
                },
                expression: format!("{:?}", expr),
                is_resource_operation: self.is_resource_operation_expr(&expr),
            });
        }

        await_points
    }

    fn find_resource_operations(&self, async_fn: &AsyncFunction) -> Vec<ResourceOperation> {
        let mut operations = Vec::new();
        let mut visitor = ResourceOpVisitor::new();

        for stmt in &async_fn.stmts {
            visitor.visit_stmt(stmt);
        }

        for (op_type, expr, line) in visitor.resource_ops {
            operations.push(ResourceOperation {
                operation_type: op_type,
                resource_type: self.infer_resource_type_from_expr(&expr),
                location: SourceLocation {
                    file: String::new(),
                    line,
                    column: 0,
                },
                variable_name: None,
            });
        }

        operations
    }

    fn find_drop_calls_in_async(&self, async_fn: &AsyncFunction) -> Vec<DropCall> {
        let mut drop_calls = Vec::new();
        let mut visitor = DropCallVisitor::new();

        for stmt in &async_fn.stmts {
            visitor.visit_stmt(stmt);
        }

        for line in visitor.drop_calls {
            drop_calls.push(DropCall {
                location: SourceLocation {
                    file: String::new(),
                    line,
                    column: 0,
                },
            });
        }

        drop_calls
    }

    fn is_resource_operation_expr(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Call(call) => self.is_resource_function_call(call),
            Expr::MethodCall(method) => self.is_resource_method_call(method),
            _ => false,
        }
    }

    fn is_resource_function_call(&self, call: &ExprCall) -> bool {
        // Check if this is a resource-related function call
        if let Expr::Path(path) = &*call.func {
            let path_str = path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            RESOURCE_FUNCTIONS.iter().any(|rf| path_str.contains(rf))
        } else {
            false
        }
    }

    fn is_resource_method_call(&self, method: &ExprMethodCall) -> bool {
        let method_name = method.method.to_string();
        RESOURCE_METHODS.iter().any(|rm| method_name == *rm)
    }

    fn infer_resource_type_from_expr(&self, expr: &Expr) -> ResourceType {
        match expr {
            Expr::Call(call) => {
                if let Expr::Path(path) = &*call.func {
                    let path_str = path
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");

                    if path_str.contains("File") {
                        ResourceType::FileHandle
                    } else if path_str.contains("TcpStream") || path_str.contains("Socket") {
                        ResourceType::NetworkConnection
                    } else if path_str.contains("Connection") || path_str.contains("Database") {
                        ResourceType::DatabaseConnection
                    } else if path_str.contains("Thread") {
                        ResourceType::ThreadHandle
                    } else if path_str.contains("Mutex") {
                        ResourceType::Mutex
                    } else if path_str.contains("Channel") {
                        ResourceType::Channel
                    } else {
                        ResourceType::SystemHandle
                    }
                } else {
                    ResourceType::SystemHandle
                }
            }
            _ => ResourceType::SystemHandle,
        }
    }

    fn suggest_cancellation_mitigation(&self, resource_op: &ResourceOperation) -> String {
        match resource_op.resource_type {
            ResourceType::FileHandle => {
                "Use tokio::fs or async-std::fs for cancellation-safe file operations".to_string()
            }
            ResourceType::NetworkConnection => {
                "Use connection pools or ensure proper cleanup in Drop implementation".to_string()
            }
            ResourceType::DatabaseConnection => {
                "Use async database drivers with proper cancellation handling".to_string()
            }
            _ => "Ensure resource cleanup in cancellation scenarios using RAII or finally blocks"
                .to_string(),
        }
    }
}

impl ResourceDetector for AsyncResourceDetector {
    fn detect_issues(&self, file: &syn::File, _path: &Path) -> Vec<ResourceManagementIssue> {
        let mut visitor = AsyncFnVisitor::new();
        visitor.visit_file(file);

        let mut issues = Vec::new();

        for async_fn in visitor.async_functions {
            let resource_usage = self.analyze_async_resource_usage(&async_fn);

            for issue in resource_usage.issues {
                issues.push(ResourceManagementIssue::AsyncResourceIssue {
                    function_name: async_fn.name.clone(),
                    issue_type: issue.issue_type,
                    cancellation_safety: issue.cancellation_safety,
                    mitigation_strategy: issue.mitigation_strategy,
                });
            }
        }

        issues
    }

    fn detector_name(&self) -> &'static str {
        "AsyncResourceDetector"
    }

    fn assess_resource_impact(&self, issue: &ResourceManagementIssue) -> ResourceImpact {
        match issue {
            ResourceManagementIssue::AsyncResourceIssue { issue_type, .. } => match issue_type {
                AsyncResourceIssueType::ResourceNotCleaned => ResourceImpact::High,
                AsyncResourceIssueType::CancellationUnsafe => ResourceImpact::Critical,
                AsyncResourceIssueType::SharedResourceRace => ResourceImpact::Critical,
                AsyncResourceIssueType::DropInAsync => ResourceImpact::Medium,
            },
            _ => ResourceImpact::Medium,
        }
    }
}

struct AsyncFnVisitor {
    async_functions: Vec<AsyncFunction>,
}

impl AsyncFnVisitor {
    fn new() -> Self {
        Self {
            async_functions: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for AsyncFnVisitor {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if node.sig.asyncness.is_some() {
            let name = node.sig.ident.to_string();
            let stmts = node.block.stmts.clone();

            self.async_functions.push(AsyncFunction { name, stmts });
        }
    }
}

struct AwaitVisitor {
    await_exprs: Vec<(Expr, usize)>,
    current_line: usize,
}

impl AwaitVisitor {
    fn new() -> Self {
        Self {
            await_exprs: Vec::new(),
            current_line: 1,
        }
    }
}

impl<'ast> Visit<'ast> for AwaitVisitor {
    fn visit_expr_await(&mut self, node: &'ast ExprAwait) {
        self.await_exprs
            .push((*node.base.clone(), self.current_line));
        self.current_line += 1;
    }
}

struct ResourceOpVisitor {
    resource_ops: Vec<(ResourceOperationType, Expr, usize)>,
    current_line: usize,
}

impl ResourceOpVisitor {
    fn new() -> Self {
        Self {
            resource_ops: Vec::new(),
            current_line: 1,
        }
    }
}

impl<'ast> Visit<'ast> for ResourceOpVisitor {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(path) = &*node.func {
            let path_str = path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            if RESOURCE_FUNCTIONS.iter().any(|rf| path_str.contains(rf)) {
                let op_type = if path_str.contains("open") || path_str.contains("new") {
                    ResourceOperationType::Acquisition
                } else if path_str.contains("close") || path_str.contains("drop") {
                    ResourceOperationType::Release
                } else {
                    ResourceOperationType::Transfer
                };

                self.resource_ops
                    .push((op_type, Expr::Call(node.clone()), self.current_line));
                self.current_line += 1;
            }
        }
    }
}

struct DropCallVisitor {
    drop_calls: Vec<usize>,
    current_line: usize,
}

impl DropCallVisitor {
    fn new() -> Self {
        Self {
            drop_calls: Vec::new(),
            current_line: 1,
        }
    }
}

impl<'ast> Visit<'ast> for DropCallVisitor {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(path) = &*node.func {
            if path.path.segments.last().is_some_and(|s| s.ident == "drop") {
                self.drop_calls.push(self.current_line);
            }
        }
        self.current_line += 1;
    }
}

pub struct CancellationAnalyzer;

impl CancellationAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_resource_cancellation_safety(
        &self,
        resource_op: &ResourceOperation,
        await_points: &[AwaitPoint],
    ) -> CancellationAnalysis {
        // Check if resource is acquired before an await point
        // and not properly cleaned up after
        let mut analysis = CancellationAnalysis {
            is_safe: true,
            reason: String::new(),
        };

        if resource_op.operation_type == ResourceOperationType::Acquisition {
            // Check if there's an await point after acquisition
            let has_await_after = await_points
                .iter()
                .any(|ap| ap.location.line > resource_op.location.line);

            if has_await_after {
                analysis.is_safe = false;
                analysis.reason =
                    "Resource acquired before await point without proper cleanup".to_string();
            }
        }

        analysis
    }
}

#[derive(Debug, Clone)]
struct AsyncFunction {
    name: String,
    stmts: Vec<Stmt>,
}

#[derive(Debug, Default)]
struct AsyncResourceUsage {
    issues: Vec<AsyncResourceIssueInfo>,
}

#[derive(Debug)]
struct AsyncResourceIssueInfo {
    issue_type: AsyncResourceIssueType,
    cancellation_safety: CancellationSafety,
    mitigation_strategy: String,
    location: SourceLocation,
}

#[derive(Debug)]
struct AwaitPoint {
    location: SourceLocation,
    expression: String,
    is_resource_operation: bool,
}

#[derive(Debug)]
struct ResourceOperation {
    operation_type: ResourceOperationType,
    resource_type: ResourceType,
    location: SourceLocation,
    variable_name: Option<String>,
}

#[derive(Debug)]
struct DropCall {
    location: SourceLocation,
}

#[derive(Debug, PartialEq)]
enum ResourceOperationType {
    Acquisition,
    Release,
    Transfer,
}

#[derive(Debug)]
pub struct CancellationAnalysis {
    pub is_safe: bool,
    pub reason: String,
}

const RESOURCE_FUNCTIONS: &[&str] = &[
    "File::open",
    "File::create",
    "TcpStream::connect",
    "TcpListener::bind",
    "Thread::spawn",
    "Connection::open",
    "Database::connect",
];

const RESOURCE_METHODS: &[&str] = &[
    "open", "create", "connect", "bind", "spawn", "close", "shutdown", "join",
];
