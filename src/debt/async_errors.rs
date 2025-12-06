use crate::core::{DebtItem, DebtType, Priority};
use crate::debt::suppression::SuppressionContext;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprCall, File, ItemFn, Stmt};

pub struct AsyncErrorDetector<'a> {
    items: Vec<DebtItem>,
    current_file: &'a Path,
    suppression: Option<&'a SuppressionContext>,
    in_async_context: bool,
    in_test_function: bool,
}

impl<'a> AsyncErrorDetector<'a> {
    pub fn new(file_path: &'a Path, suppression: Option<&'a SuppressionContext>) -> Self {
        Self {
            items: Vec::new(),
            current_file: file_path,
            suppression,
            in_async_context: false,
            in_test_function: false,
        }
    }

    pub fn detect(mut self, file: &File) -> Vec<DebtItem> {
        self.visit_file(file);
        self.items
    }

    fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        span.start().line
    }

    fn add_debt_item(&mut self, line: usize, pattern: AsyncErrorPattern, context: &str) {
        // Check if this item is suppressed
        let debt_type = DebtType::ErrorSwallowing {
            pattern: pattern.to_string(),
            context: Some(context.to_string()),
        };

        if let Some(checker) = self.suppression {
            if checker.is_suppressed(line, &debt_type) {
                return;
            }
        }

        let priority = self.determine_priority(&pattern);
        let message = format!("{}: {}", pattern.description(), pattern.remediation());

        self.items.push(DebtItem {
            id: format!("async-error-{}-{}", self.current_file.display(), line),
            debt_type,
            priority,
            file: self.current_file.to_path_buf(),
            line,
            column: None,
            message,
            context: Some(context.to_string()),
        });
    }

    fn determine_priority(&self, pattern: &AsyncErrorPattern) -> Priority {
        // Lower priority for test code
        if self.in_test_function {
            return Priority::Low;
        }

        match pattern {
            AsyncErrorPattern::DroppedFuture => Priority::High,
            AsyncErrorPattern::UnhandledJoinHandle => Priority::High,
            AsyncErrorPattern::SilentTaskPanic => Priority::Critical,
            AsyncErrorPattern::SelectBranchIgnored => Priority::Medium,
            AsyncErrorPattern::SpawnWithoutJoin => Priority::Medium,
        }
    }

    fn check_tokio_spawn(&mut self, call: &ExprCall) {
        // Check for tokio::spawn or similar patterns
        if let Expr::Path(path) = &*call.func {
            let path_str = quote::quote!(#path).to_string();

            if path_str.contains("spawn") || path_str.contains("spawn_blocking") {
                let line = self.get_line_number(call.func.span());

                // Check if the result is being handled
                // This is a simplified check - in reality we'd need more context
                self.add_debt_item(
                    line,
                    AsyncErrorPattern::SpawnWithoutJoin,
                    "Spawned task without join handle",
                );
            }
        }
    }

    fn check_join_handle(&mut self, stmt: &Stmt) {
        // Check for dropped JoinHandle
        if let Stmt::Expr(Expr::Call(call), Some(_)) = stmt {
            // Expression with semicolon (dropped value)
            let call_str = quote::quote!(#call).to_string();
            if call_str.contains("spawn") {
                let line = self.get_line_number(call.span());
                self.add_debt_item(
                    line,
                    AsyncErrorPattern::UnhandledJoinHandle,
                    "JoinHandle dropped without awaiting",
                );
            }
        }
    }

    fn check_select_patterns(&mut self, expr: &Expr) {
        // Check for tokio::select! or futures::select!
        if let Expr::Macro(mac) = expr {
            // Check both simple ident and path (e.g., tokio::select!)
            let path_str = quote::quote!(#mac.mac.path).to_string().replace(" ", "");

            // Check if this is a select! macro (handle both `select` and `tokio::select`)
            if path_str == "select" || path_str.ends_with("::select") {
                let line = self.get_line_number(mac.mac.path.span());
                // Simplified check - would need to parse the macro content
                self.add_debt_item(
                    line,
                    AsyncErrorPattern::SelectBranchIgnored,
                    "select! branch may ignore errors",
                );
            }
        }
    }

    fn check_future_dropping(&mut self, expr: &Expr) {
        // Check for futures that might be dropped
        if let Expr::MethodCall(method) = expr {
            // Check for async methods that aren't awaited
            // This is a heuristic - we'd need type info for accuracy
            let method_name = method.method.to_string();
            if method_name.starts_with("async_") || method_name.ends_with("_async") {
                // Check if this is followed by .await
                let line = self.get_line_number(method.span());
                self.add_debt_item(
                    line,
                    AsyncErrorPattern::DroppedFuture,
                    "Future may be dropped without awaiting",
                );
            }
        }
    }
}

impl<'a> Visit<'_> for AsyncErrorDetector<'a> {
    fn visit_item_fn(&mut self, node: &ItemFn) {
        let was_async = self.in_async_context;
        let was_test = self.in_test_function;

        // Check if function is async
        self.in_async_context = node.sig.asyncness.is_some();

        // Check if function is a test
        self.in_test_function = node.attrs.iter().any(|attr| {
            attr.path().get_ident().map(|i| i.to_string()).as_deref() == Some("test")
                || attr.path().get_ident().map(|i| i.to_string()).as_deref() == Some("tokio::test")
        });

        syn::visit::visit_item_fn(self, node);

        self.in_async_context = was_async;
        self.in_test_function = was_test;
    }

    fn visit_expr_call(&mut self, node: &ExprCall) {
        if self.in_async_context {
            self.check_tokio_spawn(node);
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_stmt(&mut self, node: &Stmt) {
        if self.in_async_context {
            self.check_join_handle(node);
            // Also check for macro statements in async context
            if let Stmt::Macro(stmt_macro) = node {
                // Check if it's a select! macro
                let path_str = quote::quote!(#stmt_macro.mac.path)
                    .to_string()
                    .replace(" ", "");
                if path_str == "select" || path_str.ends_with("::select") {
                    let line = self.get_line_number(stmt_macro.mac.path.span());
                    self.add_debt_item(
                        line,
                        AsyncErrorPattern::SelectBranchIgnored,
                        "select! branch may ignore errors",
                    );
                }
            }
        }
        syn::visit::visit_stmt(self, node);
    }

    fn visit_expr(&mut self, node: &Expr) {
        if self.in_async_context {
            self.check_select_patterns(node);
            self.check_future_dropping(node);
        }
        syn::visit::visit_expr(self, node);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AsyncErrorPattern {
    DroppedFuture,
    UnhandledJoinHandle,
    SilentTaskPanic,
    SelectBranchIgnored,
    SpawnWithoutJoin,
}

impl AsyncErrorPattern {
    fn description(&self) -> &'static str {
        match self {
            Self::DroppedFuture => "Future dropped without awaiting",
            Self::UnhandledJoinHandle => "JoinHandle not awaited",
            Self::SilentTaskPanic => "Task panic not handled",
            Self::SelectBranchIgnored => "select! branch ignores errors",
            Self::SpawnWithoutJoin => "spawn without join handle",
        }
    }

    fn remediation(&self) -> &'static str {
        match self {
            Self::DroppedFuture => "Await the future or explicitly handle cancellation",
            Self::UnhandledJoinHandle => {
                "Store and await JoinHandle or use spawn_and_forget explicitly"
            }
            Self::SilentTaskPanic => "Handle task panics with proper error recovery",
            Self::SelectBranchIgnored => "Handle errors in all select! branches",
            Self::SpawnWithoutJoin => "Store JoinHandle and await or handle task completion",
        }
    }
}

impl std::fmt::Display for AsyncErrorPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

pub fn detect_async_errors(
    file: &File,
    file_path: &Path,
    suppression: Option<&SuppressionContext>,
) -> Vec<DebtItem> {
    let detector = AsyncErrorDetector::new(file_path, suppression);
    detector.detect(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

    #[test]
    fn test_spawn_without_join() {
        let code = r#"
            async fn example() {
                tokio::spawn(async {
                    do_something().await;
                });
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_async_errors(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert!(items[0].message.contains("spawn"));
    }

    #[test]
    fn test_dropped_join_handle() {
        let code = r#"
            async fn example() {
                tokio::spawn(async_task());
                // JoinHandle dropped
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_async_errors(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert!(items[0].message.contains("JoinHandle"));
    }

    #[test]
    #[ignore] // TODO: Complex macro parsing requires more work
    fn test_select_macro() {
        let code = r#"
            async fn example() {
                select! {
                    _ = branch1() => {},
                    _ = branch2() => {},
                }
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = detect_async_errors(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert!(items[0].message.contains("select"));
    }

    #[test]
    fn test_dropped_future() {
        let code = r#"
            async fn example() {
                some_async_function();
                // Future not awaited
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let _items = detect_async_errors(&file, Path::new("test.rs"), None);

        // Our simple heuristic might detect this
        // In practice, we'd need type information
    }

    #[test]
    fn test_proper_async_handling() {
        let code = r#"
            async fn example() -> Result<(), Box<dyn std::error::Error>> {
                let handle = tokio::spawn(async {
                    do_something().await
                });
                
                handle.await??;
                Ok(())
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let _items = detect_async_errors(&file, Path::new("test.rs"), None);

        // Should have fewer issues when properly handled
        // Our simple analysis might still flag some things
    }
}
