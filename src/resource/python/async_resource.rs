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
    async_context_managers: HashSet<String>,
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
            async_context_managers,
        }
    }

    fn check_async_function(&self, func: &ast::StmtFunctionDef, issues: &mut Vec<ResourceIssue>) {
        // Only check async functions
        if func
            .decorator_list
            .iter()
            .any(|d| self.is_async_decorator(d))
            || func.name.starts_with("async_")
        {
            let mut async_resources = Vec::new();
            let mut has_async_with = false;
            let mut has_finally = false;

            for stmt in &func.body {
                self.analyze_async_statement(
                    stmt,
                    &mut async_resources,
                    &mut has_async_with,
                    &mut has_finally,
                );
            }

            // Check if async resources are properly managed
            for resource in async_resources {
                if !has_async_with && !has_finally {
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
                        suggestion: "Use 'async with' for async resources or ensure cleanup in finally block".to_string(),
                    });
                }
            }
        }
    }

    fn is_async_decorator(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Name(name) => &name.id == "async" || &name.id == "asyncio",
            _ => false,
        }
    }

    fn analyze_async_statement(
        &self,
        stmt: &Stmt,
        resources: &mut Vec<String>,
        has_async_with: &mut bool,
        has_finally: &mut bool,
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
                    self.analyze_async_statement(body_stmt, resources, has_async_with, has_finally);
                }
            }
            Stmt::Assign(assign) => {
                // Check for async resource creation
                if let Some(resource_type) = self.detect_async_resource(&assign.value) {
                    resources.push(resource_type);
                }
            }
            Stmt::Expr(expr_stmt) => {
                // Check for async resource creation without assignment
                if let Some(resource_type) = self.detect_async_resource(&expr_stmt.value) {
                    resources.push(resource_type);
                }
            }
            Stmt::For(for_stmt) => {
                for body_stmt in &for_stmt.body {
                    self.analyze_async_statement(body_stmt, resources, has_async_with, has_finally);
                }
            }
            Stmt::While(while_stmt) => {
                for body_stmt in &while_stmt.body {
                    self.analyze_async_statement(body_stmt, resources, has_async_with, has_finally);
                }
            }
            Stmt::If(if_stmt) => {
                for body_stmt in &if_stmt.body {
                    self.analyze_async_statement(body_stmt, resources, has_async_with, has_finally);
                }
                for else_stmt in &if_stmt.orelse {
                    self.analyze_async_statement(else_stmt, resources, has_async_with, has_finally);
                }
            }
            _ => {}
        }
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
                        self.check_async_function(func, &mut issues);
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
                            range: async_func.range.clone(),
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
                                    range: async_func.range.clone(),
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
