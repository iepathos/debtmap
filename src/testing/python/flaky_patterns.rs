use super::{FlakinessType, Severity, TestIssueType, TestQualityIssue};
use rustpython_parser::ast::{self, Expr, Stmt};

pub struct FlakyPatternDetector;

impl Default for FlakyPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl FlakyPatternDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_test_function(&self, func_def: &ast::StmtFunctionDef) -> Vec<TestQualityIssue> {
        let mut issues = Vec::new();

        // Check for timing dependencies
        if let Some(issue) = self.check_timing_dependencies(func_def) {
            issues.push(issue);
        }

        // Check for random values
        if let Some(issue) = self.check_random_usage(func_def) {
            issues.push(issue);
        }

        // Check for external dependencies
        if let Some(issue) = self.check_external_dependencies(func_def) {
            issues.push(issue);
        }

        // Check for filesystem dependencies
        if let Some(issue) = self.check_filesystem_dependencies(func_def) {
            issues.push(issue);
        }

        // Check for network dependencies
        if let Some(issue) = self.check_network_dependencies(func_def) {
            issues.push(issue);
        }

        // Check for threading issues
        if let Some(issue) = self.check_threading_issues(func_def) {
            issues.push(issue);
        }

        issues
    }

    fn check_timing_dependencies(
        &self,
        func_def: &ast::StmtFunctionDef,
    ) -> Option<TestQualityIssue> {
        if self.contains_timing_calls(&func_def.body) {
            Some(TestQualityIssue {
                issue_type: TestIssueType::FlakyPattern(FlakinessType::TimingDependency),
                test_name: func_def.name.to_string(),
                line: 1, // TODO: Extract actual line number from range
                severity: Severity::High,
                suggestion:
                    "Use mock time or freeze time libraries instead of sleep/time-based assertions"
                        .to_string(),
            })
        } else {
            None
        }
    }

    fn contains_timing_calls(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            if self.is_timing_call(stmt) {
                return true;
            }
            if self.contains_timing_calls_in_stmt(stmt) {
                return true;
            }
        }
        false
    }

    fn contains_timing_calls_in_stmt(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::If(if_stmt) => {
                self.contains_timing_calls(&if_stmt.body)
                    || self.contains_timing_calls(&if_stmt.orelse)
            }
            Stmt::For(for_stmt) => {
                self.contains_timing_calls(&for_stmt.body)
                    || self.contains_timing_calls(&for_stmt.orelse)
            }
            Stmt::While(while_stmt) => {
                self.contains_timing_calls(&while_stmt.body)
                    || self.contains_timing_calls(&while_stmt.orelse)
            }
            Stmt::With(with_stmt) => self.contains_timing_calls(&with_stmt.body),
            Stmt::Try(try_stmt) => {
                self.contains_timing_calls(&try_stmt.body)
                    || try_stmt.handlers.iter().any(|h| {
                        let ast::ExceptHandler::ExceptHandler(handler) = h;
                        self.contains_timing_calls(&handler.body)
                    })
                    || self.contains_timing_calls(&try_stmt.orelse)
                    || self.contains_timing_calls(&try_stmt.finalbody)
            }
            _ => false,
        }
    }

    fn is_timing_call(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Expr(expr_stmt) => {
                if let Expr::Call(call) = &*expr_stmt.value {
                    return self.is_timing_function(&call.func);
                }
                false
            }
            Stmt::Assign(assign_stmt) => {
                // Check if the assigned value is a timing call
                if let Expr::Call(call) = &*assign_stmt.value {
                    return self.is_timing_function(&call.func);
                }
                false
            }
            _ => false,
        }
    }

    fn is_timing_function(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Attribute(attr) => {
                let method = attr.attr.as_str();
                if method == "sleep" || method == "time" || method == "perf_counter" {
                    return true;
                }
                if let Expr::Name(name) = &*attr.value {
                    let module = name.id.as_str();
                    (module == "time" || module == "datetime")
                        && (method == "now" || method == "today" || method == "utcnow")
                } else {
                    false
                }
            }
            Expr::Name(name) => {
                let func_name = name.id.as_str();
                func_name == "sleep" || func_name == "time"
            }
            _ => false,
        }
    }

    fn check_random_usage(&self, func_def: &ast::StmtFunctionDef) -> Option<TestQualityIssue> {
        if self.contains_random_calls(&func_def.body) {
            Some(TestQualityIssue {
                issue_type: TestIssueType::FlakyPattern(FlakinessType::RandomValues),
                test_name: func_def.name.to_string(),
                line: 1, // TODO: Extract actual line number from range
                severity: Severity::High,
                suggestion: "Use fixed seed for random values or deterministic test data"
                    .to_string(),
            })
        } else {
            None
        }
    }

    fn contains_random_calls(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            if self.is_random_call(stmt) {
                return true;
            }
            if self.contains_random_calls_in_stmt(stmt) {
                return true;
            }
        }
        false
    }

    fn contains_random_calls_in_stmt(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::If(if_stmt) => {
                self.contains_random_calls(&if_stmt.body)
                    || self.contains_random_calls(&if_stmt.orelse)
            }
            Stmt::For(for_stmt) => {
                self.contains_random_calls(&for_stmt.body)
                    || self.contains_random_calls(&for_stmt.orelse)
            }
            Stmt::While(while_stmt) => {
                self.contains_random_calls(&while_stmt.body)
                    || self.contains_random_calls(&while_stmt.orelse)
            }
            Stmt::With(with_stmt) => self.contains_random_calls(&with_stmt.body),
            Stmt::Try(try_stmt) => {
                self.contains_random_calls(&try_stmt.body)
                    || try_stmt.handlers.iter().any(|h| {
                        let ast::ExceptHandler::ExceptHandler(handler) = h;
                        self.contains_random_calls(&handler.body)
                    })
                    || self.contains_random_calls(&try_stmt.orelse)
                    || self.contains_random_calls(&try_stmt.finalbody)
            }
            _ => false,
        }
    }

    fn is_random_call(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Expr(expr_stmt) => {
                if let Expr::Call(call) = &*expr_stmt.value {
                    return self.is_random_function(&call.func);
                }
                false
            }
            Stmt::Assign(assign_stmt) => {
                // Check if the assigned value is a random call
                if let Expr::Call(call) = &*assign_stmt.value {
                    return self.is_random_function(&call.func);
                }
                false
            }
            _ => false,
        }
    }

    fn is_random_function(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Attribute(attr) => {
                let method = attr.attr.as_str();
                // Check for uuid methods
                if method == "uuid4" || method == "uuid1" {
                    return true;
                }

                if let Expr::Name(name) = &*attr.value {
                    let module = name.id.as_str();
                    // Check for random and secrets modules
                    if module == "random" || module == "secrets" || module == "uuid" {
                        return true;
                    }
                }
                false
            }
            Expr::Name(name) => {
                let func_name = name.id.as_str();
                func_name.starts_with("rand") || func_name == "uuid4" || func_name == "uuid1"
            }
            _ => false,
        }
    }

    fn check_external_dependencies(
        &self,
        func_def: &ast::StmtFunctionDef,
    ) -> Option<TestQualityIssue> {
        if self.contains_external_calls(&func_def.body) {
            Some(TestQualityIssue {
                issue_type: TestIssueType::FlakyPattern(FlakinessType::ExternalDependency),
                test_name: func_def.name.to_string(),
                line: 1, // TODO: Extract actual line number from range
                severity: Severity::Medium,
                suggestion: "Mock external services or use test doubles".to_string(),
            })
        } else {
            None
        }
    }

    fn contains_external_calls(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            if self.is_external_call(stmt) {
                return true;
            }
            if self.contains_external_calls_in_stmt(stmt) {
                return true;
            }
        }
        false
    }

    fn contains_external_calls_in_stmt(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::If(if_stmt) => {
                self.contains_external_calls(&if_stmt.body)
                    || self.contains_external_calls(&if_stmt.orelse)
            }
            Stmt::For(for_stmt) => {
                self.contains_external_calls(&for_stmt.body)
                    || self.contains_external_calls(&for_stmt.orelse)
            }
            Stmt::While(while_stmt) => {
                self.contains_external_calls(&while_stmt.body)
                    || self.contains_external_calls(&while_stmt.orelse)
            }
            Stmt::With(with_stmt) => self.contains_external_calls(&with_stmt.body),
            Stmt::Try(try_stmt) => {
                self.contains_external_calls(&try_stmt.body)
                    || try_stmt.handlers.iter().any(|h| {
                        let ast::ExceptHandler::ExceptHandler(handler) = h;
                        self.contains_external_calls(&handler.body)
                    })
                    || self.contains_external_calls(&try_stmt.orelse)
                    || self.contains_external_calls(&try_stmt.finalbody)
            }
            _ => false,
        }
    }

    fn is_external_call(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Expr(expr_stmt) => {
                if let Expr::Call(call) = &*expr_stmt.value {
                    return self.is_external_function(&call.func);
                }
                false
            }
            Stmt::Assign(assign_stmt) => {
                // Check if the assigned value is an external call
                if let Expr::Call(call) = &*assign_stmt.value {
                    // For chained calls like urllib.urlopen().read(), check the inner call
                    if let Expr::Attribute(attr) = &*call.func {
                        if let Expr::Call(inner_call) = &*attr.value {
                            if self.is_external_function(&inner_call.func) {
                                return true;
                            }
                        }
                    }
                    return self.is_external_function(&call.func);
                }
                false
            }
            _ => false,
        }
    }

    fn is_external_function(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Attribute(attr) => {
                let method = attr.attr.as_str();
                if method == "get"
                    || method == "post"
                    || method == "put"
                    || method == "delete"
                    || method == "request"
                    || method == "urlopen"
                {
                    return true;
                }
                if let Expr::Name(name) = &*attr.value {
                    let module = name.id.as_str();
                    module == "requests" || module == "urllib" || module == "httpx"
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn check_filesystem_dependencies(
        &self,
        func_def: &ast::StmtFunctionDef,
    ) -> Option<TestQualityIssue> {
        if self.contains_filesystem_calls(&func_def.body)
            && !self.has_temp_directory(&func_def.body)
        {
            Some(TestQualityIssue {
                issue_type: TestIssueType::FlakyPattern(FlakinessType::FilesystemDependency),
                test_name: func_def.name.to_string(),
                line: 1, // TODO: Extract actual line number from range
                severity: Severity::Medium,
                suggestion: "Use temporary directories or mock filesystem operations".to_string(),
            })
        } else {
            None
        }
    }

    fn contains_filesystem_calls(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            if self.is_filesystem_call(stmt) {
                return true;
            }
        }
        false
    }

    fn is_filesystem_call(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::With(with_stmt) => {
                // Check for file open
                for item in &with_stmt.items {
                    if let Expr::Call(call) = &item.context_expr {
                        if let Expr::Name(name) = &*call.func {
                            if name.id.as_str() == "open" {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            Stmt::Expr(expr_stmt) => {
                if let Expr::Call(call) = &*expr_stmt.value {
                    self.is_filesystem_function(&call.func)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn is_filesystem_function(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Attribute(attr) => {
                let method = attr.attr.as_str();
                if method == "mkdir"
                    || method == "makedirs"
                    || method == "remove"
                    || method == "rmdir"
                    || method == "rmtree"
                    || method == "rename"
                    || method == "exists"
                    || method == "isfile"
                    || method == "isdir"
                {
                    return true;
                }
                if let Expr::Name(name) = &*attr.value {
                    let module = name.id.as_str();
                    module == "os" || module == "shutil" || module == "pathlib"
                } else {
                    false
                }
            }
            Expr::Name(name) => {
                let func_name = name.id.as_str();
                func_name == "open"
            }
            _ => false,
        }
    }

    fn has_temp_directory(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            if self.is_temp_directory_usage(stmt) {
                return true;
            }
        }
        false
    }

    fn is_temp_directory_usage(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::With(with_stmt) => {
                for item in &with_stmt.items {
                    if let Expr::Call(call) = &item.context_expr {
                        if self.is_temp_directory_function(&call.func) {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn is_temp_directory_function(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Attribute(attr) => {
                let method = attr.attr.as_str();
                method == "TemporaryDirectory"
                    || method == "NamedTemporaryFile"
                    || method == "mkdtemp"
                    || method == "mkstemp"
            }
            Expr::Name(name) => {
                let func_name = name.id.as_str();
                func_name == "TemporaryDirectory" || func_name == "NamedTemporaryFile"
            }
            _ => false,
        }
    }

    fn check_network_dependencies(
        &self,
        func_def: &ast::StmtFunctionDef,
    ) -> Option<TestQualityIssue> {
        if self.contains_network_calls(&func_def.body) {
            Some(TestQualityIssue {
                issue_type: TestIssueType::FlakyPattern(FlakinessType::NetworkDependency),
                test_name: func_def.name.to_string(),
                line: 1, // TODO: Extract actual line number from range
                severity: Severity::High,
                suggestion: "Mock network calls or use test servers".to_string(),
            })
        } else {
            None
        }
    }

    fn contains_network_calls(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            // Check with statements specially to avoid double-checking
            if let Stmt::With(with_stmt) = stmt {
                for item in &with_stmt.items {
                    if let Expr::Call(call) = &item.context_expr {
                        if self.is_network_function(&call.func) {
                            return true;
                        }
                    }
                }
                // Also check the body of the with statement
                if self.contains_network_calls(&with_stmt.body) {
                    return true;
                }
            } else if let Stmt::AsyncWith(async_with_stmt) = stmt {
                // Handle async with statements
                for item in &async_with_stmt.items {
                    if let Expr::Call(call) = &item.context_expr {
                        if self.is_network_function(&call.func) {
                            return true;
                        }
                    }
                }
                // Also check the body of the async with statement
                if self.contains_network_calls(&async_with_stmt.body) {
                    return true;
                }
            } else if self.is_network_call(stmt) {
                return true;
            } else if self.contains_network_calls_in_stmt(stmt) {
                return true;
            }
        }
        false
    }

    fn contains_network_calls_in_stmt(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::If(if_stmt) => {
                self.contains_network_calls(&if_stmt.body)
                    || self.contains_network_calls(&if_stmt.orelse)
            }
            Stmt::For(for_stmt) => {
                self.contains_network_calls(&for_stmt.body)
                    || self.contains_network_calls(&for_stmt.orelse)
            }
            Stmt::While(while_stmt) => {
                self.contains_network_calls(&while_stmt.body)
                    || self.contains_network_calls(&while_stmt.orelse)
            }
            Stmt::With(with_stmt) => self.contains_network_calls(&with_stmt.body),
            Stmt::Try(try_stmt) => {
                self.contains_network_calls(&try_stmt.body)
                    || try_stmt.handlers.iter().any(|h| {
                        let ast::ExceptHandler::ExceptHandler(handler) = h;
                        self.contains_network_calls(&handler.body)
                    })
                    || self.contains_network_calls(&try_stmt.orelse)
                    || self.contains_network_calls(&try_stmt.finalbody)
            }
            Stmt::AsyncWith(async_with_stmt) => self.contains_network_calls(&async_with_stmt.body),
            _ => false,
        }
    }

    fn is_network_call(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Expr(expr_stmt) => {
                // Check direct call
                if let Expr::Call(call) = &*expr_stmt.value {
                    return self.is_network_function(&call.func);
                }
                // Check await expressions
                if let Expr::Await(await_expr) = &*expr_stmt.value {
                    if let Expr::Call(call) = &*await_expr.value {
                        return self.is_network_function(&call.func);
                    }
                }
                false
            }
            Stmt::Assign(assign_stmt) => {
                // Check if the assigned value is a network call
                if let Expr::Call(call) = &*assign_stmt.value {
                    return self.is_network_function(&call.func);
                }
                // Check await expressions
                if let Expr::Await(await_expr) = &*assign_stmt.value {
                    if let Expr::Call(call) = &*await_expr.value {
                        return self.is_network_function(&call.func);
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn is_network_function(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Attribute(attr) => {
                let method = attr.attr.as_str();
                // Common network methods
                if method == "connect"
                    || method == "send"
                    || method == "recv"
                    || method == "listen"
                    || method == "accept"
                    || method == "bind"
                    || method == "get"
                    || method == "post"
                    || method == "put"
                    || method == "delete"
                    || method == "ClientSession"
                {
                    return true;
                }

                // Check for module.method patterns
                if let Expr::Name(name) = &*attr.value {
                    let module = name.id.as_str();
                    if module == "socket" || module == "asyncio" || module == "aiohttp" {
                        return true;
                    }
                }

                // Check for session.get() and similar patterns
                if let Expr::Name(name) = &*attr.value {
                    let var_name = name.id.as_str();
                    if var_name == "session"
                        && (method == "get"
                            || method == "post"
                            || method == "put"
                            || method == "delete")
                    {
                        return true;
                    }
                }

                false
            }
            Expr::Name(name) => {
                let func_name = name.id.as_str();
                func_name == "socket" || func_name == "urlopen"
            }
            _ => false,
        }
    }

    fn check_threading_issues(&self, func_def: &ast::StmtFunctionDef) -> Option<TestQualityIssue> {
        if self.contains_threading_calls(&func_def.body)
            && !self.has_proper_synchronization(&func_def.body)
        {
            Some(TestQualityIssue {
                issue_type: TestIssueType::FlakyPattern(FlakinessType::ThreadingIssue),
                test_name: func_def.name.to_string(),
                line: 1, // TODO: Extract actual line number from range
                severity: Severity::Critical,
                suggestion: "Use proper synchronization or avoid threading in tests".to_string(),
            })
        } else {
            None
        }
    }

    fn contains_threading_calls(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            // Check with statements for threading executors
            if let Stmt::With(with_stmt) = stmt {
                for item in &with_stmt.items {
                    if let Expr::Call(call) = &item.context_expr {
                        if self.is_threading_function(&call.func) {
                            return true;
                        }
                    }
                }
                // Also check the body of the with statement
                if self.contains_threading_calls(&with_stmt.body) {
                    return true;
                }
            } else if let Stmt::AsyncWith(async_with_stmt) = stmt {
                for item in &async_with_stmt.items {
                    if let Expr::Call(call) = &item.context_expr {
                        if self.is_threading_function(&call.func) {
                            return true;
                        }
                    }
                }
                // Also check the body of the async with statement
                if self.contains_threading_calls(&async_with_stmt.body) {
                    return true;
                }
            } else if self.is_threading_call(stmt) {
                return true;
            } else if self.contains_threading_calls_in_stmt(stmt) {
                return true;
            }
        }
        false
    }

    fn contains_threading_calls_in_stmt(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::If(if_stmt) => {
                self.contains_threading_calls(&if_stmt.body)
                    || self.contains_threading_calls(&if_stmt.orelse)
            }
            Stmt::For(for_stmt) => {
                self.contains_threading_calls(&for_stmt.body)
                    || self.contains_threading_calls(&for_stmt.orelse)
            }
            Stmt::While(while_stmt) => {
                self.contains_threading_calls(&while_stmt.body)
                    || self.contains_threading_calls(&while_stmt.orelse)
            }
            Stmt::Try(try_stmt) => {
                self.contains_threading_calls(&try_stmt.body)
                    || try_stmt.handlers.iter().any(|h| {
                        let ast::ExceptHandler::ExceptHandler(handler) = h;
                        self.contains_threading_calls(&handler.body)
                    })
                    || self.contains_threading_calls(&try_stmt.orelse)
                    || self.contains_threading_calls(&try_stmt.finalbody)
            }
            _ => false,
        }
    }

    fn is_threading_call(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Expr(expr_stmt) => {
                if let Expr::Call(call) = &*expr_stmt.value {
                    return self.is_threading_function(&call.func);
                }
                false
            }
            Stmt::Assign(assign_stmt) => {
                // Check if the assigned value is a threading call
                if let Expr::Call(call) = &*assign_stmt.value {
                    return self.is_threading_function(&call.func);
                }
                false
            }
            _ => false,
        }
    }

    fn is_threading_function(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Attribute(attr) => {
                let method = attr.attr.as_str();
                if method == "Thread"
                    || method == "start"
                    || method == "join"
                    || method == "Process"
                    || method == "Pool"
                    || method == "ThreadPoolExecutor"
                    || method == "ProcessPoolExecutor"
                    || method == "submit"
                {
                    return true;
                }
                // Check for module.something patterns
                if let Expr::Name(name) = &*attr.value {
                    let module = name.id.as_str();
                    if module == "threading"
                        || module == "multiprocessing"
                        || module == "concurrent"
                    {
                        return true;
                    }
                }
                // Check for nested module patterns like concurrent.futures
                if let Expr::Attribute(inner_attr) = &*attr.value {
                    if let Expr::Name(name) = &*inner_attr.value {
                        let module = name.id.as_str();
                        let submodule = inner_attr.attr.as_str();
                        if module == "concurrent" && submodule == "futures" {
                            return true;
                        }
                    }
                }
                false
            }
            Expr::Name(name) => {
                let func_name = name.id.as_str();
                func_name == "Thread" || func_name == "Process"
            }
            _ => false,
        }
    }

    fn has_proper_synchronization(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            if self.is_synchronization_primitive(stmt) {
                return true;
            }
            // Check nested statements
            match stmt {
                Stmt::If(if_stmt) => {
                    if self.has_proper_synchronization(&if_stmt.body)
                        || self.has_proper_synchronization(&if_stmt.orelse)
                    {
                        return true;
                    }
                }
                Stmt::For(for_stmt) => {
                    if self.has_proper_synchronization(&for_stmt.body)
                        || self.has_proper_synchronization(&for_stmt.orelse)
                    {
                        return true;
                    }
                }
                Stmt::While(while_stmt) => {
                    if self.has_proper_synchronization(&while_stmt.body)
                        || self.has_proper_synchronization(&while_stmt.orelse)
                    {
                        return true;
                    }
                }
                Stmt::With(with_stmt) => {
                    if self.has_proper_synchronization(&with_stmt.body) {
                        return true;
                    }
                }
                Stmt::Try(try_stmt) => {
                    if self.has_proper_synchronization(&try_stmt.body)
                        || try_stmt.handlers.iter().any(|h| {
                            let ast::ExceptHandler::ExceptHandler(handler) = h;
                            self.has_proper_synchronization(&handler.body)
                        })
                        || self.has_proper_synchronization(&try_stmt.orelse)
                        || self.has_proper_synchronization(&try_stmt.finalbody)
                    {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn is_synchronization_primitive(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::With(with_stmt) => {
                for item in &with_stmt.items {
                    // Check if it's a direct Lock() call
                    if let Expr::Call(call) = &item.context_expr {
                        if self.is_lock_function(&call.func) {
                            return true;
                        }
                    }
                    // Check if it's a variable that might be a lock (common pattern)
                    if let Expr::Name(name) = &item.context_expr {
                        let var_name = name.id.as_str();
                        if var_name == "lock"
                            || var_name == "sem"
                            || var_name == "mutex"
                            || var_name == "semaphore"
                            || var_name == "event"
                        {
                            return true;
                        }
                    }
                }
                false
            }
            // Also check assignments of lock primitives
            Stmt::Assign(assign_stmt) => {
                if let Expr::Call(call) = &*assign_stmt.value {
                    self.is_lock_function(&call.func)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn is_lock_function(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Attribute(attr) => {
                let method = attr.attr.as_str();
                method == "Lock"
                    || method == "RLock"
                    || method == "Semaphore"
                    || method == "Event"
                    || method == "Condition"
            }
            Expr::Name(name) => {
                let func_name = name.id.as_str();
                func_name == "Lock" || func_name == "RLock" || func_name == "Semaphore"
            }
            _ => false,
        }
    }
}
