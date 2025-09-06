/// Async resource management detection for Python
use super::{
    AffectedScope, ImpactLevel, PythonResourceDetector, PythonResourceIssueType, ResourceImpact,
    ResourceIssue, ResourceLocation, ResourceSeverity,
};
use rustpython_parser::ast::{self, Expr, Stmt};
use std::collections::HashSet;
use std::path::Path;

pub struct PythonAsyncResourceDetector {
    async_resource_types: HashSet<String>,
    _async_context_managers: HashSet<String>,
}

impl Default for PythonAsyncResourceDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonAsyncResourceDetector {
    pub fn new() -> Self {
        let mut async_resource_types = HashSet::new();
        async_resource_types.insert("aiohttp.ClientSession".to_string());
        async_resource_types.insert("asyncio.Lock".to_string());
        async_resource_types.insert("asyncio.Semaphore".to_string());
        async_resource_types.insert("asyncio.Queue".to_string());
        async_resource_types.insert("asyncpg.Connection".to_string());
        async_resource_types.insert("aiofiles.open".to_string());
        async_resource_types.insert("asyncio.create_task".to_string());
        async_resource_types.insert("asyncio.create_subprocess".to_string());

        let mut async_context_managers = HashSet::new();
        async_context_managers.insert("async with".to_string());
        async_context_managers.insert("aenter".to_string());
        async_context_managers.insert("aexit".to_string());

        Self {
            async_resource_types,
            _async_context_managers: async_context_managers,
        }
    }

    fn check_async_function(&self, func: &ast::StmtFunctionDef, issues: &mut Vec<ResourceIssue>) {
        // Check all functions for async resources, not just those with async decorators
        let mut async_resources = Vec::new();
        let mut resource_variables = std::collections::HashMap::new();
        let mut has_async_with = false;
        let mut has_finally = false;
        let mut closed_resources = std::collections::HashSet::new();

        for stmt in &func.body {
            self.analyze_async_statement_enhanced(
                stmt,
                &mut async_resources,
                &mut resource_variables,
                &mut has_async_with,
                &mut has_finally,
                &mut closed_resources,
            );
        }

        // Check if async resources are properly managed
        for resource in async_resources {
            // Check if this specific resource was closed
            let var_name = resource_variables
                .get(&resource)
                .cloned()
                .unwrap_or_default();
            let is_closed =
                closed_resources.contains(&var_name) || closed_resources.contains(&resource);

            if !has_async_with && !has_finally && !is_closed {
                issues.push(ResourceIssue {
                    issue_type: PythonResourceIssueType::AsyncResourceLeak {
                        function_name: func.name.to_string(),
                        resource_type: resource,
                    },
                    severity: ResourceSeverity::High,
                    location: ResourceLocation {
                        line: 1, // TODO: Track actual line numbers
                        column: 0,
                        end_line: None,
                        end_column: None,
                    },
                    suggestion:
                        "Use 'async with' for async resources or ensure cleanup in finally block"
                            .to_string(),
                });
            }
        }
    }

    #[allow(dead_code)]
    fn is_async_decorator(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Name(name) => &name.id == "async" || &name.id == "asyncio",
            _ => false,
        }
    }

    fn analyze_async_statement_enhanced(
        &self,
        stmt: &Stmt,
        resources: &mut Vec<String>,
        resource_variables: &mut std::collections::HashMap<String, String>,
        has_async_with: &mut bool,
        has_finally: &mut bool,
        closed_resources: &mut std::collections::HashSet<String>,
    ) {
        match stmt {
            Stmt::AsyncWith(_) => {
                *has_async_with = true;
            }
            Stmt::Try(try_stmt) => {
                if !try_stmt.finalbody.is_empty() {
                    *has_finally = true;
                }
                for body_stmt in &try_stmt.body {
                    self.analyze_async_statement_enhanced(
                        body_stmt,
                        resources,
                        resource_variables,
                        has_async_with,
                        has_finally,
                        closed_resources,
                    );
                }
            }
            Stmt::Assign(assign) => {
                // Check for async resource creation
                if let Some(resource_type) = self.detect_async_resource(&assign.value) {
                    resources.push(resource_type.clone());

                    // Track variable name for this resource
                    if let Some(Expr::Name(name)) = assign.targets.first() {
                        resource_variables.insert(resource_type, name.id.to_string());
                    }
                }
            }
            Stmt::Expr(expr_stmt) => {
                // Check for resource cleanup calls
                if let Expr::Call(call) = expr_stmt.value.as_ref() {
                    if let Expr::Attribute(attr) = call.func.as_ref() {
                        if &attr.attr == "close"
                            || &attr.attr == "aclose"
                            || &attr.attr == "shutdown"
                        {
                            // Resource is being closed
                            if let Expr::Name(name) = attr.value.as_ref() {
                                closed_resources.insert(name.id.to_string());
                            }
                        }
                    }
                }

                // Check for async resource creation without assignment
                if let Some(resource_type) = self.detect_async_resource(&expr_stmt.value) {
                    resources.push(resource_type);
                }
            }
            Stmt::For(for_stmt) => {
                for body_stmt in &for_stmt.body {
                    self.analyze_async_statement_enhanced(
                        body_stmt,
                        resources,
                        resource_variables,
                        has_async_with,
                        has_finally,
                        closed_resources,
                    );
                }
            }
            Stmt::While(while_stmt) => {
                for body_stmt in &while_stmt.body {
                    self.analyze_async_statement_enhanced(
                        body_stmt,
                        resources,
                        resource_variables,
                        has_async_with,
                        has_finally,
                        closed_resources,
                    );
                }
            }
            Stmt::If(if_stmt) => {
                for body_stmt in &if_stmt.body {
                    self.analyze_async_statement_enhanced(
                        body_stmt,
                        resources,
                        resource_variables,
                        has_async_with,
                        has_finally,
                        closed_resources,
                    );
                }
                for else_stmt in &if_stmt.orelse {
                    self.analyze_async_statement_enhanced(
                        else_stmt,
                        resources,
                        resource_variables,
                        has_async_with,
                        has_finally,
                        closed_resources,
                    );
                }
            }
            _ => {}
        }
    }

    #[allow(dead_code)]
    fn analyze_async_statement(
        &self,
        stmt: &Stmt,
        resources: &mut Vec<String>,
        has_async_with: &mut bool,
        has_finally: &mut bool,
    ) {
        let mut resource_vars = std::collections::HashMap::new();
        let mut closed = std::collections::HashSet::new();
        self.analyze_async_statement_enhanced(
            stmt,
            resources,
            &mut resource_vars,
            has_async_with,
            has_finally,
            &mut closed,
        );
    }

    fn detect_async_resource(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Call(call) => {
                // Check for async resource creation
                let func_name = self.extract_function_name(call.func.as_ref());

                for resource_type in &self.async_resource_types {
                    if func_name.contains(resource_type) || resource_type.contains(&func_name) {
                        return Some(resource_type.clone());
                    }
                }

                // Check for common async patterns
                if func_name.contains("create_task")
                    || func_name.contains("create_subprocess")
                    || func_name.contains("ClientSession")
                    || func_name.contains("aiofiles.open")
                {
                    return Some(func_name);
                }
            }
            Expr::Await(await_expr) => {
                // Check awaited expressions
                return self.detect_async_resource(await_expr.value.as_ref());
            }
            _ => {}
        }
        None
    }

    #[allow(clippy::only_used_in_recursion)]
    fn extract_function_name(&self, expr: &Expr) -> String {
        match expr {
            Expr::Name(name) => name.id.to_string(),
            Expr::Attribute(attr) => {
                let base = self.extract_function_name(attr.value.as_ref());
                format!("{}.{}", base, attr.attr)
            }
            _ => String::new(),
        }
    }
}

impl PythonResourceDetector for PythonAsyncResourceDetector {
    fn detect_issues(&self, module: &ast::Mod, _path: &Path) -> Vec<ResourceIssue> {
        let mut issues = Vec::new();

        if let ast::Mod::Module(module) = module {
            for stmt in &module.body {
                match stmt {
                    Stmt::FunctionDef(func) => {
                        // Check all functions, including those named async_* or containing async resources
                        if func.name.starts_with("async_")
                            || func.name.contains("fetch")
                            || func.name.contains("async")
                        {
                            self.check_async_function(func, &mut issues);
                        } else {
                            // Still check for async resources in regular functions
                            self.check_async_function(func, &mut issues);
                        }
                    }
                    Stmt::AsyncFunctionDef(async_func) => {
                        // Convert to regular function def for analysis
                        let func = ast::StmtFunctionDef {
                            name: async_func.name.clone(),
                            args: async_func.args.clone(),
                            body: async_func.body.clone(),
                            decorator_list: async_func.decorator_list.clone(),
                            returns: async_func.returns.clone(),
                            type_comment: async_func.type_comment.clone(),
                            type_params: vec![],
                            range: async_func.range,
                        };
                        self.check_async_function(&func, &mut issues);
                    }
                    Stmt::ClassDef(class_def) => {
                        // Check methods in classes
                        for class_stmt in &class_def.body {
                            if let Stmt::FunctionDef(func) = class_stmt {
                                self.check_async_function(func, &mut issues);
                            } else if let Stmt::AsyncFunctionDef(async_func) = class_stmt {
                                let func = ast::StmtFunctionDef {
                                    name: async_func.name.clone(),
                                    args: async_func.args.clone(),
                                    body: async_func.body.clone(),
                                    decorator_list: async_func.decorator_list.clone(),
                                    returns: async_func.returns.clone(),
                                    type_comment: async_func.type_comment.clone(),
                                    type_params: vec![],
                                    range: async_func.range,
                                };
                                self.check_async_function(&func, &mut issues);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        issues
    }

    fn assess_resource_impact(&self, _issue: &ResourceIssue) -> ResourceImpact {
        ResourceImpact {
            impact_level: ImpactLevel::High,
            affected_scope: AffectedScope::Function,
            estimated_severity: 0.8,
        }
    }
}
