/// Thread and process resource tracking for Python
use super::{
    AffectedScope, ImpactLevel, PythonResourceDetector, PythonResourceIssueType, ResourceImpact,
    ResourceIssue, ResourceLocation, ResourceSeverity,
};
use rustpython_parser::ast::{self, Expr, Stmt};
use std::collections::HashSet;
use std::path::Path;

pub struct PythonResourceTracker {
    thread_functions: HashSet<String>,
    process_functions: HashSet<String>,
    cleanup_methods: HashSet<String>,
}

impl PythonResourceTracker {
    pub fn new() -> Self {
        let mut thread_functions = HashSet::new();
        thread_functions.insert("threading.Thread".to_string());
        thread_functions.insert("concurrent.futures.ThreadPoolExecutor".to_string());
        thread_functions.insert("Thread".to_string());
        thread_functions.insert("start_new_thread".to_string());

        let mut process_functions = HashSet::new();
        process_functions.insert("multiprocessing.Process".to_string());
        process_functions.insert("subprocess.Popen".to_string());
        process_functions.insert("concurrent.futures.ProcessPoolExecutor".to_string());
        process_functions.insert("Process".to_string());
        process_functions.insert("Popen".to_string());
        // Add more multiprocessing patterns
        process_functions.insert("Pool".to_string());
        process_functions.insert("multiprocessing.Pool".to_string());

        let mut cleanup_methods = HashSet::new();
        cleanup_methods.insert("join".to_string());
        cleanup_methods.insert("close".to_string());
        cleanup_methods.insert("terminate".to_string());
        cleanup_methods.insert("kill".to_string());
        cleanup_methods.insert("shutdown".to_string());
        cleanup_methods.insert("__del__".to_string());
        cleanup_methods.insert("__exit__".to_string());
        cleanup_methods.insert("cleanup".to_string());
        cleanup_methods.insert("wait".to_string());

        Self {
            thread_functions,
            process_functions,
            cleanup_methods,
        }
    }

    fn analyze_class_resources(&self, class_def: &ast::StmtClassDef) -> Vec<ResourceIssue> {
        let mut issues = Vec::new();
        let mut managed_resources = HashSet::new();
        let mut has_cleanup = false;
        let mut has_del = false;
        let mut has_exit = false;

        // Scan class for resource management
        for stmt in &class_def.body {
            match stmt {
                Stmt::FunctionDef(func) => {
                    if &func.name == "__init__" {
                        // Check for resource allocation in __init__
                        for init_stmt in &func.body {
                            if let Some(resources) = self.detect_resource_allocation(init_stmt) {
                                for resource in resources {
                                    managed_resources.insert(resource);
                                }
                            }
                        }
                    } else if &func.name == "__del__" {
                        has_del = true;
                    } else if &func.name == "__exit__" {
                        has_exit = true;
                    } else if self.cleanup_methods.contains(&func.name.to_string()) {
                        has_cleanup = true;
                    }
                }
                _ => {}
            }
        }

        // Check if resources need cleanup
        if !managed_resources.is_empty() && !has_del && !has_exit && !has_cleanup {
            issues.push(ResourceIssue {
                issue_type: PythonResourceIssueType::MissingCleanup {
                    class_name: class_def.name.to_string(),
                    resources: managed_resources.into_iter().collect(),
                },
                severity: ResourceSeverity::Medium,
                location: ResourceLocation {
                    line: 1, // TODO: Track actual line numbers
                    column: 0,
                    end_line: None,
                    end_column: None,
                },
                suggestion: format!(
                    "Class '{}' manages resources but lacks cleanup. Add __del__, __exit__, or cleanup method.",
                    class_def.name
                ),
            });
        }

        issues
    }

    fn detect_resource_allocation(&self, stmt: &Stmt) -> Option<Vec<String>> {
        match stmt {
            Stmt::Assign(assign) => {
                if let Some(resource_type) = self.detect_resource_type(&assign.value) {
                    let var_name = self.extract_variable_name(&assign.targets);
                    return Some(vec![format!("{}: {}", var_name, resource_type)]);
                }
            }
            _ => {}
        }
        None
    }

    fn detect_resource_type(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Call(call) => {
                let func_name = self.extract_function_name(call.func.as_ref());

                if self
                    .thread_functions
                    .iter()
                    .any(|tf| func_name.contains(tf) || tf.contains(&func_name))
                {
                    return Some("Thread".to_string());
                }

                if self
                    .process_functions
                    .iter()
                    .any(|pf| func_name.contains(pf) || pf.contains(&func_name))
                {
                    return Some("Process".to_string());
                }

                // Enhanced multiprocessing.Process detection
                if func_name == "Process"
                    || func_name.ends_with(".Process")
                    || func_name.contains("multiprocessing") && func_name.contains("Process")
                {
                    return Some("Process".to_string());
                }

                // Check for file handles
                if func_name == "open" || func_name.ends_with(".open") {
                    return Some("File".to_string());
                }

                // Check for sockets
                if func_name.contains("socket") || func_name.contains("Socket") {
                    return Some("Socket".to_string());
                }

                // Check for database connections and pools
                if func_name.contains("connect") || func_name.contains("Connection") {
                    return Some("Connection".to_string());
                }

                // Check for connection pools
                if func_name.contains("create_engine")
                    || func_name.contains("Pool")
                    || func_name.contains("pool")
                        && (func_name.contains("create") || func_name.contains("get"))
                {
                    return Some("ConnectionPool".to_string());
                }
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
                if base.is_empty() {
                    attr.attr.to_string()
                } else {
                    format!("{}.{}", base, attr.attr)
                }
            }
            _ => String::new(),
        }
    }

    fn extract_variable_name(&self, targets: &[Expr]) -> String {
        if let Some(target) = targets.first() {
            match target {
                Expr::Name(name) => return name.id.to_string(),
                Expr::Attribute(attr) => {
                    if let Expr::Name(name) = attr.value.as_ref() {
                        if &name.id == "self" {
                            return format!("self.{}", attr.attr);
                        }
                    }
                }
                _ => {}
            }
        }
        "<unknown>".to_string()
    }

    fn check_thread_process_usage(&self, module: &ast::Mod) -> Vec<ResourceIssue> {
        let mut issues = Vec::new();

        if let ast::Mod::Module(module) = module {
            for stmt in &module.body {
                self.check_statement_for_resources(stmt, &mut issues);
            }
        }

        issues
    }

    fn check_statement_for_resources(&self, stmt: &Stmt, issues: &mut Vec<ResourceIssue>) {
        match stmt {
            Stmt::FunctionDef(func) => {
                let mut thread_process_created = Vec::new();
                let mut has_join_or_cleanup = false;
                let mut _for_loop_count = 0;
                let mut _has_loop_with_process = false;

                for func_stmt in &func.body {
                    // Check for loops that might create multiple processes
                    if let Stmt::For(for_stmt) = func_stmt {
                        _for_loop_count += 1;
                        for loop_stmt in &for_stmt.body {
                            if let Some(resources) = self.detect_resource_allocation(loop_stmt) {
                                for resource in &resources {
                                    if resource.contains("Process") {
                                        _has_loop_with_process = true;
                                        // Multiple processes in a loop
                                        thread_process_created
                                            .push(format!("{}[multiple]", resource));
                                    } else {
                                        thread_process_created.push(resource.clone());
                                    }
                                }
                            }
                        }
                    } else if let Some(resources) = self.detect_resource_allocation(func_stmt) {
                        thread_process_created.extend(resources);
                    }

                    // Check for cleanup calls
                    if let Stmt::Expr(expr_stmt) = func_stmt {
                        if let Expr::Call(call) = expr_stmt.value.as_ref() {
                            if let Expr::Attribute(attr) = call.func.as_ref() {
                                let attr_str = attr.attr.to_string();
                                if self.cleanup_methods.contains(&attr_str) {
                                    has_join_or_cleanup = true;
                                }
                            }
                        }
                    }
                }

                // Check if threads/processes are created but not joined
                for resource in thread_process_created {
                    if !has_join_or_cleanup
                        && (resource.contains("Thread")
                            || resource.contains("Process")
                            || resource.contains("ConnectionPool"))
                    {
                        let resource_type = if resource.contains("Thread") {
                            "Thread".to_string()
                        } else if resource.contains("Process") {
                            "Process".to_string()
                        } else {
                            "ConnectionPool".to_string()
                        };

                        let issue_type = if resource.contains("ConnectionPool") {
                            PythonResourceIssueType::UnclosedResource {
                                resource_type: resource_type.clone(),
                                variable_name: func.name.to_string(),
                            }
                        } else {
                            PythonResourceIssueType::ThreadOrProcessLeak {
                                resource_type: resource_type.clone(),
                                name: func.name.to_string(),
                            }
                        };

                        issues.push(ResourceIssue {
                            issue_type,
                            severity: ResourceSeverity::High,
                            location: ResourceLocation {
                                line: 1, // TODO: Track actual line numbers
                                column: 0,
                                end_line: None,
                                end_column: None,
                            },
                            suggestion: if resource.contains("ConnectionPool") {
                                format!("Connection pool created in '{}' but not closed. Call .close() or .dispose().", func.name)
                            } else {
                                format!(
                                    "{} created in '{}' but not joined. Call .join() or use with statement.",
                                    resource_type, func.name
                                )
                            },
                        });
                    }
                }
            }
            Stmt::ClassDef(_) => {
                // Handled separately
            }
            _ => {}
        }
    }
}

impl PythonResourceDetector for PythonResourceTracker {
    fn detect_issues(&self, module: &ast::Mod, _path: &Path) -> Vec<ResourceIssue> {
        let mut all_issues = Vec::new();

        // Check for class-level resource management
        if let ast::Mod::Module(module) = module {
            for stmt in &module.body {
                if let Stmt::ClassDef(class_def) = stmt {
                    all_issues.extend(self.analyze_class_resources(class_def));
                }
            }
        }

        // Check for thread/process leaks
        all_issues.extend(self.check_thread_process_usage(module));

        all_issues
    }

    fn assess_resource_impact(&self, issue: &ResourceIssue) -> ResourceImpact {
        let impact_level = match &issue.issue_type {
            PythonResourceIssueType::ThreadOrProcessLeak { .. } => ImpactLevel::High,
            PythonResourceIssueType::MissingCleanup { .. } => ImpactLevel::Medium,
            _ => ImpactLevel::Low,
        };

        ResourceImpact {
            impact_level,
            affected_scope: match &issue.issue_type {
                PythonResourceIssueType::ThreadOrProcessLeak { .. } => AffectedScope::Function,
                PythonResourceIssueType::MissingCleanup { .. } => AffectedScope::Class,
                _ => AffectedScope::Module,
            },
            estimated_severity: match impact_level {
                ImpactLevel::Critical => 1.0,
                ImpactLevel::High => 0.8,
                ImpactLevel::Medium => 0.5,
                ImpactLevel::Low => 0.3,
            },
        }
    }
}
