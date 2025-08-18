//! Detection of async boundaries and async context analysis

use syn::{
    visit::Visit, Block, Expr, ExprAsync, ExprAwait, ExprBlock, ExprCall, ExprClosure,
    ExprMethodCall, ItemFn, Stmt,
};

/// Detects async boundaries where blocking I/O would be problematic
pub struct AsyncBoundaryDetector {
    /// Stack of async contexts (true = in async context)
    async_stack: Vec<bool>,
    /// Whether we're currently in an async boundary
    pub in_async_boundary: bool,
    /// Detected blocking I/O calls in async context
    pub blocking_in_async: Vec<BlockingCall>,
}

#[derive(Debug, Clone)]
pub struct BlockingCall {
    pub function_name: String,
    pub is_blocking: bool,
    pub in_async_context: bool,
    pub line: usize,
}

impl AsyncBoundaryDetector {
    pub fn new() -> Self {
        Self {
            async_stack: vec![false],
            in_async_boundary: false,
            blocking_in_async: Vec::new(),
        }
    }

    /// Check if we're in an async context
    fn is_in_async(&self) -> bool {
        self.async_stack.last().copied().unwrap_or(false)
    }

    /// Push async context
    fn push_async(&mut self, is_async: bool) {
        self.async_stack.push(is_async || self.is_in_async());
    }

    /// Pop async context
    fn pop_async(&mut self) {
        self.async_stack.pop();
    }

    /// Check if a function call is blocking I/O
    fn is_blocking_io(path: &str, method: &str) -> bool {
        // First check if it's an async I/O library - these are NOT blocking
        let async_patterns = [
            "tokio::",
            "async_std::",
            "futures::",
            "smol::",
        ];
        
        for pattern in &async_patterns {
            if path.starts_with(pattern) {
                return false; // Async I/O is not blocking
            }
        }
        
        // Common blocking I/O patterns
        let blocking_patterns = [
            // File I/O (standard library)
            ("std::fs", "read"),
            ("std::fs", "write"),
            ("std::fs", "read_to_string"),
            ("std::fs", "read_dir"),
            ("std::fs", "copy"),
            ("std::fs", "rename"),
            ("std::fs", "remove_file"),
            ("File", "open"),
            ("File", "create"),
            // Network I/O (synchronous)
            ("std::net", "TcpStream"),
            ("std::net", "TcpListener"),
            ("std::net", "UdpSocket"),
            // Process/Command
            ("std::process", "Command"),
            ("Command", "output"),
            ("Command", "status"),
            // Thread blocking
            ("std::thread", "sleep"),
            ("thread", "sleep"),
            // Synchronous HTTP clients
            ("reqwest", "blocking"),
            ("ureq", "get"),
            ("ureq", "post"),
        ];

        // Check for blocking patterns
        for (module, func) in &blocking_patterns {
            // Check if the path contains the module pattern
            // For exact module matching (e.g., "std::fs" but not "tokio::fs")
            if (path == *module || path.starts_with(&format!("{}::", module)) || path.ends_with(&format!("::{}", module))) 
                && method == *func {
                return true;
            }
        }

        // Check for common blocking method names
        let blocking_methods = [
            "read_to_string",
            "read_to_end",
            "read_exact",
            "write_all",
            "flush",
            "sync_all",
            "set_len",
            "sleep",
            "wait",
            "join",
        ];

        blocking_methods.contains(&method)
    }

    /// Detect if we're in an async block boundary
    fn detect_async_boundary(&mut self, block: &Block) -> bool {
        // Check if this block contains await expressions
        let mut has_await = false;
        for stmt in &block.stmts {
            match stmt {
                Stmt::Expr(expr, _) => {
                    has_await = contains_await(expr);
                    if has_await {
                        break;
                    }
                }
                Stmt::Local(local) => {
                    if let Some(init) = &local.init {
                        has_await = contains_await(&init.expr);
                        if has_await {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        has_await
    }
}

impl<'ast> Visit<'ast> for AsyncBoundaryDetector {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        // Check if function is async
        let is_async = node.sig.asyncness.is_some();
        self.push_async(is_async);

        if is_async {
            self.in_async_boundary = true;
        }

        syn::visit::visit_item_fn(self, node);

        self.pop_async();
        if is_async {
            self.in_async_boundary = false;
        }
    }

    fn visit_expr_async(&mut self, node: &'ast ExprAsync) {
        // Entering async block
        self.push_async(true);
        self.in_async_boundary = true;
        syn::visit::visit_expr_async(self, node);
        self.pop_async();
    }

    fn visit_expr_closure(&mut self, node: &'ast ExprClosure) {
        // Check if closure is async
        let is_async = node.asyncness.is_some();
        self.push_async(is_async);
        syn::visit::visit_expr_closure(self, node);
        self.pop_async();
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if self.is_in_async() {
            // Check if this is a blocking call
            if let Expr::Path(path) = &*node.func {
                let path_str = path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");

                let last_segment = path
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();

                if Self::is_blocking_io(&path_str, &last_segment) {
                    self.blocking_in_async.push(BlockingCall {
                        function_name: path_str,
                        is_blocking: true,
                        in_async_context: true,
                        line: 0, // Would need span info for actual line
                    });
                }
            }
        }

        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        if self.is_in_async() {
            let method_name = node.method.to_string();

            // Check receiver for type hints
            let receiver_str = match &*node.receiver {
                Expr::Path(path) => path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::"),
                _ => String::new(),
            };

            if Self::is_blocking_io(&receiver_str, &method_name) {
                self.blocking_in_async.push(BlockingCall {
                    function_name: format!("{}.{}", receiver_str, method_name),
                    is_blocking: true,
                    in_async_context: true,
                    line: 0,
                });
            }
        }

        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_block(&mut self, node: &'ast ExprBlock) {
        // Check if this block has async boundary characteristics
        let has_boundary = self.detect_async_boundary(&node.block);
        if has_boundary {
            self.in_async_boundary = true;
        }

        syn::visit::visit_expr_block(self, node);

        if has_boundary {
            self.in_async_boundary = false;
        }
    }
}

/// Check if an expression contains await
fn contains_await(expr: &Expr) -> bool {
    struct AwaitChecker {
        has_await: bool,
    }

    impl<'ast> Visit<'ast> for AwaitChecker {
        fn visit_expr_await(&mut self, _: &'ast ExprAwait) {
            self.has_await = true;
        }
    }

    let mut checker = AwaitChecker { has_await: false };
    checker.visit_expr(expr);
    checker.has_await
}

impl Default for AsyncBoundaryDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocking_io_detection() {
        assert!(AsyncBoundaryDetector::is_blocking_io("std::fs", "read"));
        assert!(AsyncBoundaryDetector::is_blocking_io(
            "std::fs::read_to_string",
            "read_to_string"
        ));
        assert!(AsyncBoundaryDetector::is_blocking_io("File", "open"));
        assert!(!AsyncBoundaryDetector::is_blocking_io("tokio::fs", "read"));
        assert!(!AsyncBoundaryDetector::is_blocking_io(
            "async_std::fs",
            "read"
        ));
    }

    #[test]
    fn test_async_context_detection() {
        let code = r#"
            async fn process_data() {
                let data = std::fs::read_to_string("file.txt").unwrap();
                process(data).await;
            }
        "#;

        let file = syn::parse_file(code).unwrap();
        let mut detector = AsyncBoundaryDetector::new();

        for item in file.items {
            if let syn::Item::Fn(func) = item {
                detector.visit_item_fn(&func);
            }
        }

        // Should detect blocking I/O in async context
        assert!(!detector.blocking_in_async.is_empty());
    }
}
