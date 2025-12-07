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
use syn::{spanned::Spanned, visit::Visit, Expr, ExprCall, ExprMethodCall, ItemFn, Macro};

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

    /// Handle macro invocations
    fn handle_macro(&mut self, mac: &Macro) {
        let path = &mac.path;
        let path_str = quote::quote!(#path).to_string();

        // Check console I/O macros
        for &macro_name in CONSOLE_IO_MACROS {
            if path_str.ends_with(macro_name) {
                let line = self.extract_line(path.segments.span());
                self.operations.push(IoOperation {
                    operation_type: "console".to_string(),
                    variables: vec![],
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
            self.operations.push(IoOperation {
                operation_type: "console".to_string(),
                variables: vec![],
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
            self.operations.push(IoOperation {
                operation_type: op_type.to_string(),
                variables: vec![],
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
                    self.operations.push(IoOperation {
                        operation_type: op_type.to_string(),
                        variables: vec![],
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
                        self.operations.push(IoOperation {
                            operation_type: op_type.to_string(),
                            variables: vec![],
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
}
