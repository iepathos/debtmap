//! Detection of async boundaries and async context analysis

use std::collections::HashSet;
use syn::{
    Block, Expr, ExprAsync, ExprAwait, ExprBlock, ExprCall, ExprClosure, ExprMethodCall, Item,
    ItemFn, Path, Stmt, UseTree, visit::Visit,
};

/// Detects async boundaries where blocking I/O would be problematic
pub struct AsyncBoundaryDetector {
    /// Stack of async contexts (true = in async context)
    async_stack: Vec<bool>,
    /// Whether we're currently in an async boundary
    pub in_async_boundary: bool,
    /// Detected blocking I/O calls in async context
    pub blocking_in_async: Vec<BlockingCall>,
    /// Track imports to disambiguate Command types
    imports: ImportTracker,
}

#[derive(Debug, Clone, Default)]
struct ImportTracker {
    /// Set of async command imports (tokio::process, async_std::process)
    has_async_command: bool,
    /// Set of blocking command imports (std::process)
    has_std_command: bool,
    /// Imported symbols and their sources
    imported_symbols: HashSet<String>,
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
            imports: ImportTracker::default(),
        }
    }

    /// Analyze file with import tracking
    pub fn analyze_file(&mut self, file: &syn::File) {
        // First pass: analyze imports
        self.analyze_imports(&file.items);
        // Second pass: visit the file normally
        self.visit_file(file);
    }

    /// Analyze imports before visiting the rest of the file
    fn analyze_imports(&mut self, items: &[Item]) {
        for item in items {
            if let Item::Use(use_item) = item {
                self.process_use_tree(&use_item.tree, String::new());
            }
        }
    }

    fn process_use_tree(&mut self, tree: &UseTree, prefix: String) {
        match tree {
            UseTree::Path(path) => {
                let new_prefix = join_import_path(&prefix, &path.ident.to_string());
                self.process_use_tree(&path.tree, new_prefix);
            }
            UseTree::Name(name) => {
                let symbol = name.ident.to_string();
                let full_path = join_import_path(&prefix, &symbol);
                self.imports.record_named_import(full_path, symbol);
            }
            UseTree::Glob(_) => {
                self.imports.record_glob_import(&prefix);
            }
            UseTree::Group(group) => {
                for tree in &group.items {
                    self.process_use_tree(tree, prefix.clone());
                }
            }
            UseTree::Rename(rename) => {
                let symbol = rename.ident.to_string();
                let alias = rename.rename.to_string();
                let full_path = join_import_path(&prefix, &symbol);
                self.imports.record_renamed_import(full_path, alias);
            }
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
    fn is_blocking_io(&self, path: &str, method: &str) -> bool {
        // First check if it's an async I/O library - these are NOT blocking
        let async_patterns = ["tokio::", "async_std::", "futures::", "smol::"];

        for pattern in &async_patterns {
            if path.starts_with(pattern) {
                return false; // Async I/O is not blocking
            }
        }

        // Special case: If method is "output" or "status" on Command,
        // check our import tracking to determine if it's blocking
        if (method == "output" || method == "status" || method == "spawn") && path == "Command" {
            // If we have import information, use it
            if self.imports.has_async_command && !self.imports.has_std_command {
                return false; // Definitely async
            } else if self.imports.has_std_command && !self.imports.has_async_command {
                return true; // Definitely blocking
            } else {
                // Ambiguous or no imports - in async context, assume async
                return false;
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
            // Process/Command - only flag if explicitly std::process
            ("std::process::Command", "output"),
            ("std::process::Command", "status"),
            ("std::process::Command", "spawn"),
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
            if (path == *module
                || path.starts_with(&format!("{}::", module))
                || path.ends_with(&format!("::{}", module)))
                && method == *func
            {
                return true;
            }
        }

        // Check for common blocking method names
        // But be careful - some of these could be async versions too
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

        // Only flag these if path suggests it's std library, not async library
        if blocking_methods.contains(&method) {
            // If path is empty or doesn't indicate async library, might be blocking
            return path.is_empty() || path.starts_with("std::");
        }

        false
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

    fn record_blocking_method_call(&mut self, node: &ExprMethodCall) {
        if !self.is_in_async() {
            return;
        }

        let method_name = node.method.to_string();
        let receiver_str = receiver_type_hint(&node.receiver);

        if self.is_blocking_io(&receiver_str, &method_name) {
            self.blocking_in_async.push(BlockingCall {
                function_name: format!("{receiver_str}.{method_name}"),
                is_blocking: true,
                in_async_context: true,
                line: 0,
            });
        }
    }
}

impl ImportTracker {
    fn record_named_import(&mut self, full_path: String, symbol: String) {
        self.record_command_import(&full_path, symbol);
        self.imported_symbols.insert(full_path);
    }

    fn record_renamed_import(&mut self, full_path: String, alias: String) {
        self.record_command_import(&full_path, alias);
    }

    fn record_glob_import(&mut self, prefix: &str) {
        if is_async_command_module(prefix) {
            self.has_async_command = true;
        } else if is_std_command_module(prefix) {
            self.has_std_command = true;
        }
    }

    fn record_command_import(&mut self, full_path: &str, imported_symbol: String) {
        if is_async_command_path(full_path) {
            self.has_async_command = true;
            self.imported_symbols.insert(imported_symbol);
        } else if is_std_command_path(full_path) {
            self.has_std_command = true;
            self.imported_symbols.insert(imported_symbol);
        }
    }
}

fn join_import_path(prefix: &str, symbol: &str) -> String {
    if prefix.is_empty() {
        symbol.to_string()
    } else {
        format!("{prefix}::{symbol}")
    }
}

fn is_async_command_path(path: &str) -> bool {
    matches!(
        path,
        "tokio::process::Command" | "async_std::process::Command"
    )
}

fn is_std_command_path(path: &str) -> bool {
    path == "std::process::Command"
}

fn is_async_command_module(path: &str) -> bool {
    matches!(path, "tokio::process" | "async_std::process")
}

fn is_std_command_module(path: &str) -> bool {
    path == "std::process"
}

fn path_to_string(path: &Path) -> String {
    path.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn receiver_type_hint(receiver: &Expr) -> String {
    match receiver {
        Expr::Path(path) => path_to_string(&path.path),
        Expr::Call(call) => call_receiver_type_hint(call),
        _ => String::new(),
    }
}

fn call_receiver_type_hint(call: &ExprCall) -> String {
    match &*call.func {
        Expr::Path(path) => constructor_receiver_type(&path_to_string(&path.path)),
        _ => String::new(),
    }
}

fn constructor_receiver_type(path: &str) -> String {
    path.strip_suffix("::new").unwrap_or(path).to_string()
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

                if self.is_blocking_io(&path_str, &last_segment) {
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
        self.record_blocking_method_call(node);
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
        let detector = AsyncBoundaryDetector::new();
        assert!(detector.is_blocking_io("std::fs", "read"));
        assert!(detector.is_blocking_io("std::fs::read_to_string", "read_to_string"));
        assert!(detector.is_blocking_io("File", "open"));
        assert!(!detector.is_blocking_io("tokio::fs", "read"));
        assert!(!detector.is_blocking_io("async_std::fs", "read"));
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

    #[test]
    fn grouped_std_command_import_is_tracked() {
        let file = syn::parse_file("use std::{fs, process::Command};").unwrap();
        let mut detector = AsyncBoundaryDetector::new();

        detector.analyze_imports(&file.items);

        assert!(detector.imports.has_std_command);
        assert!(detector.imports.imported_symbols.contains("Command"));
        assert!(
            detector
                .imports
                .imported_symbols
                .contains("std::process::Command")
        );
    }

    #[test]
    fn renamed_async_command_import_is_tracked_by_alias() {
        let file = syn::parse_file("use tokio::process::Command as TokioCommand;").unwrap();
        let mut detector = AsyncBoundaryDetector::new();

        detector.analyze_imports(&file.items);

        assert!(detector.imports.has_async_command);
        assert!(detector.imports.imported_symbols.contains("TokioCommand"));
    }

    #[test]
    fn glob_command_import_tracks_command_source() {
        let file = syn::parse_file("use async_std::process::*;").unwrap();
        let mut detector = AsyncBoundaryDetector::new();

        detector.analyze_imports(&file.items);

        assert!(detector.imports.has_async_command);
        assert!(!detector.imports.has_std_command);
    }

    #[test]
    fn method_receiver_path_is_normalized() {
        let expr: Expr = syn::parse_str("std::fs::File.open()").unwrap();

        let receiver = match expr {
            Expr::MethodCall(call) => call.receiver,
            _ => panic!("expected method call"),
        };

        assert_eq!(receiver_type_hint(&receiver), "std::fs::File");
    }

    #[test]
    fn constructor_method_receiver_is_normalized_to_type() {
        let expr: Expr = syn::parse_str("Command::new(\"echo\").output()").unwrap();

        let receiver = match expr {
            Expr::MethodCall(call) => call.receiver,
            _ => panic!("expected method call"),
        };

        assert_eq!(receiver_type_hint(&receiver), "Command");
    }
}
