//! AST-based I/O operation detection for Rust code.
//!
//! This module implements comprehensive I/O detection by analyzing Rust syntax trees
//! to identify I/O operations based on actual code patterns rather than function names.
//!
//! # Supported Patterns
//!
//! ## File I/O
//! ```rust,ignore
//! File::open("path") // Detected as file_io
//! std::fs::read_to_string("path") // Detected as file_io
//! BufReader::new(file) // Detected as file_io
//! ```
//!
//! ## Console I/O
//! ```rust,ignore
//! println!("message") // Detected as console
//! eprintln!("error") // Detected as console
//! std::io::stdout() // Detected as console
//! ```
//!
//! ## Network I/O
//! ```rust,ignore
//! TcpStream::connect("addr") // Detected as network
//! client.get("url").send() // Detected as network
//! ```
//!
//! ## Database I/O
//! ```rust,ignore
//! conn.execute("query") // Detected as database
//! sqlx::query("SELECT") // Detected as database
//! ```
//!
//! ## Async I/O
//! ```rust,ignore
//! tokio::fs::read("path").await // Detected as file_io
//! async_std::fs::write("path", data).await // Detected as file_io
//! ```
//!
//! # Limitations
//!
//! - Indirect I/O (functions calling I/O functions) not detected
//! - Custom I/O wrappers may not be detected without explicit rules
//! - Type inference is limited to same-function scope

use crate::data_flow::IoOperation;
use syn::{
    punctuated::Punctuated, spanned::Spanned, token::Comma, visit::Visit, Expr, ExprCall,
    ExprMethodCall, ItemFn, Macro, Member,
};

/// Pattern for matching I/O operations
#[derive(Debug)]
struct IoPattern {
    /// Type name to match (e.g., "File", "TcpStream")
    receiver_type: Option<&'static str>,
    /// Method or function name to match
    method_name: &'static str,
    /// Category of I/O operation
    operation_type: &'static str,
}

// File I/O patterns
const FILE_IO_PATTERNS: &[IoPattern] = &[
    // std::fs module functions
    IoPattern {
        receiver_type: None,
        method_name: "read",
        operation_type: "file_io",
    },
    IoPattern {
        receiver_type: None,
        method_name: "write",
        operation_type: "file_io",
    },
    IoPattern {
        receiver_type: None,
        method_name: "read_to_string",
        operation_type: "file_io",
    },
    IoPattern {
        receiver_type: None,
        method_name: "read_to_end",
        operation_type: "file_io",
    },
    IoPattern {
        receiver_type: None,
        method_name: "write_all",
        operation_type: "file_io",
    },
    // File type methods
    IoPattern {
        receiver_type: Some("File"),
        method_name: "open",
        operation_type: "file_io",
    },
    IoPattern {
        receiver_type: Some("File"),
        method_name: "create",
        operation_type: "file_io",
    },
    IoPattern {
        receiver_type: Some("File"),
        method_name: "read",
        operation_type: "file_io",
    },
    IoPattern {
        receiver_type: Some("File"),
        method_name: "write",
        operation_type: "file_io",
    },
    IoPattern {
        receiver_type: Some("File"),
        method_name: "write_all",
        operation_type: "file_io",
    },
    IoPattern {
        receiver_type: Some("File"),
        method_name: "read_to_end",
        operation_type: "file_io",
    },
    IoPattern {
        receiver_type: Some("File"),
        method_name: "read_to_string",
        operation_type: "file_io",
    },
    // BufReader/BufWriter
    IoPattern {
        receiver_type: Some("BufReader"),
        method_name: "new",
        operation_type: "file_io",
    },
    IoPattern {
        receiver_type: Some("BufWriter"),
        method_name: "new",
        operation_type: "file_io",
    },
    IoPattern {
        receiver_type: Some("BufReader"),
        method_name: "read_line",
        operation_type: "file_io",
    },
    IoPattern {
        receiver_type: Some("BufWriter"),
        method_name: "flush",
        operation_type: "file_io",
    },
];

// Network I/O patterns
const NETWORK_IO_PATTERNS: &[IoPattern] = &[
    // std::net types
    IoPattern {
        receiver_type: Some("TcpStream"),
        method_name: "connect",
        operation_type: "network",
    },
    IoPattern {
        receiver_type: Some("TcpListener"),
        method_name: "bind",
        operation_type: "network",
    },
    IoPattern {
        receiver_type: Some("TcpListener"),
        method_name: "accept",
        operation_type: "network",
    },
    IoPattern {
        receiver_type: Some("UdpSocket"),
        method_name: "bind",
        operation_type: "network",
    },
    IoPattern {
        receiver_type: Some("UdpSocket"),
        method_name: "send",
        operation_type: "network",
    },
    IoPattern {
        receiver_type: Some("UdpSocket"),
        method_name: "recv",
        operation_type: "network",
    },
    // HTTP client patterns (generic)
    IoPattern {
        receiver_type: None,
        method_name: "get",
        operation_type: "network",
    },
    IoPattern {
        receiver_type: None,
        method_name: "post",
        operation_type: "network",
    },
    IoPattern {
        receiver_type: None,
        method_name: "put",
        operation_type: "network",
    },
    IoPattern {
        receiver_type: None,
        method_name: "delete",
        operation_type: "network",
    },
    IoPattern {
        receiver_type: None,
        method_name: "send",
        operation_type: "network",
    },
    IoPattern {
        receiver_type: None,
        method_name: "fetch",
        operation_type: "network",
    },
    IoPattern {
        receiver_type: None,
        method_name: "request",
        operation_type: "network",
    },
];

// Database I/O patterns
const DATABASE_IO_PATTERNS: &[IoPattern] = &[
    IoPattern {
        receiver_type: None,
        method_name: "execute",
        operation_type: "database",
    },
    IoPattern {
        receiver_type: None,
        method_name: "query",
        operation_type: "database",
    },
    IoPattern {
        receiver_type: None,
        method_name: "prepare",
        operation_type: "database",
    },
    IoPattern {
        receiver_type: None,
        method_name: "query_as",
        operation_type: "database",
    },
    IoPattern {
        receiver_type: None,
        method_name: "fetch",
        operation_type: "database",
    },
    IoPattern {
        receiver_type: None,
        method_name: "fetch_one",
        operation_type: "database",
    },
    IoPattern {
        receiver_type: None,
        method_name: "fetch_all",
        operation_type: "database",
    },
];

// Console I/O macro names
const CONSOLE_IO_MACROS: &[&str] = &[
    "println", "print", "eprintln", "eprint", "dbg", "write", "writeln",
];

/// Visitor that detects I/O operations in a function's AST
pub struct IoDetectorVisitor {
    operations: Vec<IoOperation>,
}

impl IoDetectorVisitor {
    fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }

    /// Extract line number from a span
    fn extract_line(&self, span: proc_macro2::Span) -> usize {
        span.start().line
    }

    /// Check if a path string represents an I/O module
    fn is_io_module_path(&self, path: &str) -> bool {
        path.contains("std::fs::")
            || path.contains("std::io::")
            || path.contains("std::net::")
            || path.contains("tokio::fs::")
            || path.contains("tokio::net::")
            || path.contains("async_std::fs::")
            || path.contains("async_std::net::")
    }

    /// Determine operation type from module path
    fn operation_type_from_path(&self, path: &str) -> Option<&'static str> {
        if path.contains("::fs::") {
            Some("file_io")
        } else if path.contains("::net::") {
            Some("network")
        } else if path.contains("::io::stdout")
            || path.contains("::io::stderr")
            || path.contains("::io::stdin")
        {
            Some("console")
        } else {
            None
        }
    }

    /// Check if a method call matches any I/O pattern
    fn check_method_patterns(
        &self,
        method_name: &str,
        receiver: Option<&str>,
    ) -> Option<&'static str> {
        // Check all pattern lists
        for pattern in FILE_IO_PATTERNS
            .iter()
            .chain(NETWORK_IO_PATTERNS.iter())
            .chain(DATABASE_IO_PATTERNS.iter())
        {
            // Match method name
            if pattern.method_name != method_name {
                continue;
            }

            // If pattern specifies a receiver type, check it
            if let Some(required_receiver) = pattern.receiver_type {
                if let Some(actual_receiver) = receiver {
                    if actual_receiver.contains(required_receiver) {
                        return Some(pattern.operation_type);
                    }
                }
                // If receiver type is required but we don't have one, skip
                continue;
            }

            // Pattern doesn't require specific receiver, so it matches
            return Some(pattern.operation_type);
        }

        None
    }

    /// Extract receiver type from expression (simplified type inference)
    fn infer_receiver_type(&self, receiver: &Expr) -> Option<String> {
        match receiver {
            Expr::Path(path) => {
                // Extract the last segment as the type
                let path_str = quote::quote!(#path).to_string();
                Some(path_str)
            }
            Expr::Call(call) => {
                // For constructor calls like File::create()
                if let Expr::Path(path) = &*call.func {
                    let path_str = quote::quote!(#path).to_string();
                    // Extract type name before ::
                    if let Some(pos) = path_str.rfind("::") {
                        return Some(path_str[..pos].to_string());
                    }
                    Some(path_str)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Extract variables from expression arguments
    fn extract_variables_from_args(&self, args: &Punctuated<Expr, Comma>) -> Vec<String> {
        let mut vars = Vec::new();
        for arg in args {
            collect_variables_from_expr(arg, &mut vars, 0);
        }
        vars.sort();
        vars.dedup();
        vars
    }

    /// Extract variables from macro tokens (heuristic-based)
    fn extract_variables_from_macro(&self, mac: &Macro) -> Vec<String> {
        let mut vars = Vec::new();
        let token_str = mac.tokens.to_string();

        // Split on common delimiters and extract identifiers
        for token in token_str.split(&[',', ' ', '{', '}', '(', ')', '[', ']', ':', ';'][..]) {
            let trimmed = token.trim();
            if is_valid_identifier(trimmed) && !is_literal_or_keyword(trimmed) {
                vars.push(trimmed.to_string());
            }
        }

        vars.sort();
        vars.dedup();
        vars
    }

    /// Handle macro invocations
    fn handle_macro(&mut self, mac: &Macro) {
        let path = &mac.path;
        let path_str = quote::quote!(#path).to_string();

        // Check console I/O macros
        for &macro_name in CONSOLE_IO_MACROS {
            if path_str.ends_with(macro_name) {
                let line = self.extract_line(path.segments.span());
                let variables = self.extract_variables_from_macro(mac);
                self.operations.push(IoOperation {
                    operation_type: "console".to_string(),
                    variables,
                    line,
                });
                return;
            }
        }

        // Check for write!/writeln! to stdout/stderr
        if (path_str.ends_with("write") || path_str.ends_with("writeln"))
            && path_str.contains("std::io")
        {
            let line = self.extract_line(path.segments.span());
            let variables = self.extract_variables_from_macro(mac);
            self.operations.push(IoOperation {
                operation_type: "console".to_string(),
                variables,
                line,
            });
        }
    }
}

impl<'ast> Visit<'ast> for IoDetectorVisitor {
    fn visit_expr_method_call(&mut self, method_call: &'ast ExprMethodCall) {
        let method_name = method_call.method.to_string();

        // Infer receiver type
        let receiver_type = self.infer_receiver_type(&method_call.receiver);

        // Check against patterns
        if let Some(op_type) = self.check_method_patterns(&method_name, receiver_type.as_deref()) {
            let line = self.extract_line(method_call.method.span());

            // Extract variables from receiver and arguments
            let mut variables = Vec::new();
            collect_variables_from_expr(&method_call.receiver, &mut variables, 0);
            for arg in &method_call.args {
                collect_variables_from_expr(arg, &mut variables, 0);
            }
            variables.sort();
            variables.dedup();

            self.operations.push(IoOperation {
                operation_type: op_type.to_string(),
                variables,
                line,
            });
        }

        // Continue visiting nested expressions
        syn::visit::visit_expr_method_call(self, method_call);
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        // Check if this is a function call to an I/O API
        if let Expr::Path(path) = &*call.func {
            let path_str = quote::quote!(#path).to_string();

            // Check for module-based I/O calls
            if self.is_io_module_path(&path_str) {
                if let Some(op_type) = self.operation_type_from_path(&path_str) {
                    let line = self.extract_line(path.span());
                    let variables = self.extract_variables_from_args(&call.args);
                    self.operations.push(IoOperation {
                        operation_type: op_type.to_string(),
                        variables,
                        line,
                    });
                }
            } else {
                // Check for type-associated functions like File::create
                // Note: quote adds spaces around ::, so we need to trim each segment
                let segments: Vec<&str> = path_str.split("::").map(|s| s.trim()).collect();
                if segments.len() >= 2 {
                    let type_name = segments[segments.len() - 2];
                    let method_name = segments[segments.len() - 1];

                    if let Some(op_type) = self.check_method_patterns(method_name, Some(type_name))
                    {
                        let line = self.extract_line(path.span());
                        let variables = self.extract_variables_from_args(&call.args);
                        self.operations.push(IoOperation {
                            operation_type: op_type.to_string(),
                            variables,
                            line,
                        });
                    }
                }
            }
        }

        // Continue visiting nested expressions
        syn::visit::visit_expr_call(self, call);
    }

    fn visit_macro(&mut self, mac: &'ast Macro) {
        self.handle_macro(mac);
        // Continue visiting (macros may contain nested expressions)
        syn::visit::visit_macro(self, mac);
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        // Handle expression macros
        if let Expr::Macro(expr_macro) = expr {
            self.handle_macro(&expr_macro.mac);
        }

        // Continue visiting nested expressions
        syn::visit::visit_expr(self, expr);
    }
}

/// Recursively collect variable names from an expression
fn collect_variables_from_expr(expr: &Expr, vars: &mut Vec<String>, depth: usize) {
    const MAX_DEPTH: usize = 2;
    if depth > MAX_DEPTH {
        return;
    }

    match expr {
        Expr::Path(path) => {
            // Extract variable name from path
            if let Some(ident) = path.path.get_ident() {
                vars.push(ident.to_string());
            } else {
                // Multi-segment path: self.field or module::item
                let path_str = quote::quote!(#path).to_string();
                // Only include if it looks like a variable (lowercase start)
                if let Some(first_char) = path_str.chars().next() {
                    if first_char.is_lowercase() || path_str.starts_with("self") {
                        vars.push(path_str);
                    }
                }
            }
        }
        Expr::Field(field) => {
            // field.member access - collect full path
            collect_variables_from_expr(&field.base, vars, depth + 1);
            if let Member::Named(name) = &field.member {
                vars.push(name.to_string());
            }
        }
        Expr::Reference(reference) => {
            // &expr or &mut expr - unwrap reference
            collect_variables_from_expr(&reference.expr, vars, depth);
        }
        Expr::Unary(unary) => {
            // *expr, !expr, -expr - unwrap unary
            collect_variables_from_expr(&unary.expr, vars, depth);
        }
        Expr::Call(call) => {
            // Function call: extract args
            for arg in &call.args {
                collect_variables_from_expr(arg, vars, depth + 1);
            }
        }
        Expr::MethodCall(method_call) => {
            // Receiver and args
            collect_variables_from_expr(&method_call.receiver, vars, depth + 1);
            for arg in &method_call.args {
                collect_variables_from_expr(arg, vars, depth + 1);
            }
        }
        Expr::Index(index) => {
            // array[index]
            collect_variables_from_expr(&index.expr, vars, depth + 1);
            collect_variables_from_expr(&index.index, vars, depth + 1);
        }
        // Stop at literals, blocks, closures
        _ => {}
    }
}

/// Check if a string is a valid Rust identifier
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    let first = chars.next().unwrap();

    // Must start with letter or underscore
    if !first.is_alphabetic() && first != '_' {
        return false;
    }

    // Rest must be alphanumeric or underscore
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Check if a string is a literal or keyword that should not be treated as a variable
fn is_literal_or_keyword(s: &str) -> bool {
    // Check if it's a number
    if s.chars().all(|c| c.is_numeric() || c == '.') {
        return true;
    }

    // Check if it's a string literal
    if s.starts_with('"') || s.starts_with('\'') {
        return true;
    }

    // Check common keywords and types
    matches!(
        s,
        "true"
            | "false"
            | "None"
            | "Some"
            | "Ok"
            | "Err"
            | "self"
            | "Self"
            | "super"
            | "crate"
            | "String"
            | "Vec"
            | "Option"
            | "Result"
            | "let"
            | "mut"
            | "fn"
            | "if"
            | "else"
            | "match"
            | "for"
            | "while"
            | "loop"
            | "return"
    )
}

/// Detect I/O operations in a function's AST
///
/// This function analyzes the syntax tree of a Rust function to identify
/// I/O operations based on actual code patterns rather than function names.
///
/// # Arguments
///
/// * `item_fn` - The function's AST node
///
/// # Returns
///
/// A vector of detected I/O operations with their types and line numbers
///
/// # Example
///
/// ```rust,ignore
/// use syn::parse_quote;
/// use debtmap::analyzers::io_detector::detect_io_operations;
///
/// let function: ItemFn = parse_quote! {
///     fn example() {
///         let file = File::create("test.txt")?;
///         println!("Created file");
///     }
/// };
///
/// let operations = detect_io_operations(&function);
/// assert_eq!(operations.len(), 2); // File::create + println!
/// ```
pub fn detect_io_operations(item_fn: &ItemFn) -> Vec<IoOperation> {
    let mut visitor = IoDetectorVisitor::new();
    visitor.visit_item_fn(item_fn);
    visitor.operations
}

/// Detect I/O operations from a block (for impl methods - spec 202)
///
/// This is a variant of `detect_io_operations` that takes a block directly,
/// allowing analysis of impl block methods which have a `Block` rather than
/// an `ItemFn`.
pub fn detect_io_operations_from_block(block: &syn::Block) -> Vec<IoOperation> {
    let mut visitor = IoDetectorVisitor::new();
    visitor.visit_block(block);
    visitor.operations
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_detects_file_create() {
        let function: ItemFn = parse_quote! {
            fn test() {
                let f = File::create("test.txt")?;
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, "file_io");
    }

    #[test]
    fn test_detects_file_open() {
        let function: ItemFn = parse_quote! {
            fn test() {
                let f = File::open("test.txt")?;
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, "file_io");
    }

    #[test]
    fn test_detects_println_macro() {
        let function: ItemFn = parse_quote! {
            fn test() {
                println!("Hello, world!");
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, "console");
    }

    #[test]
    fn test_detects_eprintln_macro() {
        let function: ItemFn = parse_quote! {
            fn test() {
                eprintln!("Error occurred");
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, "console");
    }

    #[test]
    fn test_detects_network_tcp_connect() {
        let function: ItemFn = parse_quote! {
            fn test() {
                let stream = TcpStream::connect("127.0.0.1:8080")?;
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, "network");
    }

    #[test]
    fn test_detects_http_get() {
        let function: ItemFn = parse_quote! {
            fn test() {
                let response = client.get("https://example.com").send()?;
            }
        };
        let ops = detect_io_operations(&function);
        // Should detect both .get() and .send()
        assert!(ops.iter().any(|op| op.operation_type == "network"));
    }

    #[test]
    fn test_detects_database_query() {
        let function: ItemFn = parse_quote! {
            fn test() {
                let rows = conn.query("SELECT * FROM users")?;
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, "database");
    }

    #[test]
    fn test_detects_database_execute() {
        let function: ItemFn = parse_quote! {
            fn test() {
                conn.execute("INSERT INTO users VALUES (?)", params)?;
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, "database");
    }

    #[test]
    fn test_detects_multiple_operations() {
        let function: ItemFn = parse_quote! {
            fn test() {
                let f = File::open("test.txt")?;
                println!("Opened file");
                let data = f.read_to_string(&mut String::new())?;
                eprintln!("Read {} bytes", data.len());
            }
        };
        let ops = detect_io_operations(&function);
        assert!(ops.len() >= 3); // At least file open, println, eprintln
        assert!(ops.iter().any(|op| op.operation_type == "file_io"));
        assert!(ops.iter().any(|op| op.operation_type == "console"));
    }

    #[test]
    fn test_no_io_operations() {
        let function: ItemFn = parse_quote! {
            fn test() {
                let x = 42;
                let y = x * 2;
                y + 10
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 0);
    }

    #[test]
    fn test_detects_std_fs_read() {
        let function: ItemFn = parse_quote! {
            fn test() {
                let contents = std::fs::read_to_string("test.txt")?;
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, "file_io");
    }

    #[test]
    fn test_detects_bufreader() {
        let function: ItemFn = parse_quote! {
            fn test() {
                let file = File::open("test.txt")?;
                let reader = BufReader::new(file);
            }
        };
        let ops = detect_io_operations(&function);
        assert!(ops.len() >= 2); // File::open + BufReader::new
        assert!(ops.iter().all(|op| op.operation_type == "file_io"));
    }

    // Variable extraction tests
    #[test]
    fn test_extract_simple_variable() {
        let function: ItemFn = parse_quote! {
            fn test() {
                std::fs::write(path, content)?;
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert!(ops[0].variables.contains(&"path".to_string()));
        assert!(ops[0].variables.contains(&"content".to_string()));
    }

    #[test]
    fn test_extract_field_access() {
        let function: ItemFn = parse_quote! {
            fn test() {
                file.write(self.buffer)?;
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert!(
            ops[0].variables.contains(&"buffer".to_string())
                || ops[0].variables.contains(&"self".to_string())
        );
    }

    #[test]
    fn test_extract_multiple_args() {
        let function: ItemFn = parse_quote! {
            fn test() {
                conn.execute(query, params)?;
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        let vars = &ops[0].variables;
        assert!(vars.contains(&"conn".to_string()));
        assert!(vars.contains(&"params".to_string()) || vars.contains(&"query".to_string()));
    }

    #[test]
    fn test_extract_from_println() {
        let function: ItemFn = parse_quote! {
            fn test() {
                println!("Value: {}", x);
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert!(ops[0].variables.contains(&"x".to_string()));
    }

    #[test]
    fn test_extract_multiple_variables_from_macro() {
        let function: ItemFn = parse_quote! {
            fn test() {
                println!("User {} has status {}", name, status);
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        let vars = &ops[0].variables;
        assert!(vars.contains(&"name".to_string()));
        assert!(vars.contains(&"status".to_string()));
    }

    #[test]
    fn test_deduplication() {
        let function: ItemFn = parse_quote! {
            fn test() {
                println!("{} {}", x, x);
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        let x_count = ops[0].variables.iter().filter(|v| *v == "x").count();
        assert_eq!(x_count, 1); // Deduplicated
    }

    #[test]
    fn test_method_chain() {
        let function: ItemFn = parse_quote! {
            fn test() {
                file.write_all(data.clone())?;
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert!(
            ops[0].variables.contains(&"data".to_string())
                || ops[0].variables.contains(&"file".to_string())
        );
    }

    #[test]
    fn test_reference_extraction() {
        let function: ItemFn = parse_quote! {
            fn test() {
                file.write(&buffer)?;
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert!(ops[0].variables.contains(&"buffer".to_string()));
    }

    #[test]
    fn test_complex_expression() {
        let function: ItemFn = parse_quote! {
            fn test() {
                println!("{}", calculate(x, y));
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        // Should extract x and y from calculate(x, y)
        let vars = &ops[0].variables;
        assert!(vars.contains(&"x".to_string()) || vars.contains(&"y".to_string()));
    }

    #[test]
    fn test_no_false_positives_from_literals() {
        let function: ItemFn = parse_quote! {
            fn test() {
                println!("literal string");
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        // Should not include "literal" or "string" as variables
        assert!(!ops[0].variables.contains(&"literal".to_string()));
        assert!(!ops[0].variables.contains(&"string".to_string()));
    }

    #[test]
    fn test_integration_file_write_with_variables() {
        let function: ItemFn = parse_quote! {
            fn write_data(path: &str, content: String) {
                std::fs::write(path, content).unwrap();
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, "file_io");
        assert!(ops[0].variables.contains(&"path".to_string()));
        assert!(ops[0].variables.contains(&"content".to_string()));
    }

    #[test]
    fn test_integration_println_with_variables() {
        let function: ItemFn = parse_quote! {
            fn log_status(name: &str, status: i32) {
                println!("User {} has status {}", name, status);
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        assert!(ops[0].variables.contains(&"name".to_string()));
        assert!(ops[0].variables.contains(&"status".to_string()));
    }

    #[test]
    fn test_integration_network_with_variables() {
        let function: ItemFn = parse_quote! {
            fn send_request(url: &str, body: String) {
                client.post(url).send(body).unwrap();
            }
        };
        let ops = detect_io_operations(&function);
        assert!(!ops.is_empty());
        // Should have variables from the network operations
        let all_vars: Vec<String> = ops
            .iter()
            .flat_map(|op| op.variables.iter())
            .cloned()
            .collect();
        assert!(
            all_vars.contains(&"url".to_string())
                || all_vars.contains(&"body".to_string())
                || all_vars.contains(&"client".to_string())
        );
    }

    #[test]
    fn test_variables_sorted_and_unique() {
        let function: ItemFn = parse_quote! {
            fn test() {
                println!("{} {} {}", z, a, z);
            }
        };
        let ops = detect_io_operations(&function);
        assert_eq!(ops.len(), 1);
        let vars = &ops[0].variables;
        // Check sorted
        let mut sorted_vars = vars.clone();
        sorted_vars.sort();
        assert_eq!(vars, &sorted_vars);
        // Check unique
        assert_eq!(
            vars.len(),
            vars.iter().collect::<std::collections::HashSet<_>>().len()
        );
    }

    // Helper function tests
    #[test]
    fn test_is_valid_identifier() {
        assert!(is_valid_identifier("x"));
        assert!(is_valid_identifier("variable_name"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("value123"));
        assert!(!is_valid_identifier("123invalid"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("with-dash"));
    }

    #[test]
    fn test_is_literal_or_keyword() {
        assert!(is_literal_or_keyword("true"));
        assert!(is_literal_or_keyword("false"));
        assert!(is_literal_or_keyword("None"));
        assert!(is_literal_or_keyword("123"));
        assert!(is_literal_or_keyword("\"string\""));
        assert!(is_literal_or_keyword("'c'"));
        assert!(!is_literal_or_keyword("variable"));
        assert!(!is_literal_or_keyword("my_var"));
    }
}
