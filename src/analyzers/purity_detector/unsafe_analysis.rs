//! Unsafe block analysis for purity detection (Spec 161)
//!
//! Classifies unsafe operations as pure (no side effects) or impure.

use super::types::UnsafeOp;
use syn::{visit::Visit, Block, Expr};

/// Collector for unsafe operations within a block
pub struct UnsafeOpCollector {
    pub ops: Vec<UnsafeOp>,
}

impl UnsafeOpCollector {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    /// Collect all unsafe operations from a block
    pub fn collect(block: &Block) -> Vec<UnsafeOp> {
        let mut collector = Self::new();
        collector.visit_block(block);
        collector.ops
    }
}

impl Default for UnsafeOpCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ast> Visit<'ast> for UnsafeOpCollector {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // Function calls: check for transmute or FFI
            Expr::Call(call) => {
                if let Expr::Path(path) = &*call.func {
                    let path_str = quote::quote!(#path).to_string();
                    if path_str.contains("transmute") {
                        self.ops.push(UnsafeOp::Transmute);
                    } else if !path_str.starts_with("std::")
                        && !path_str.starts_with("core::")
                        && path.path.segments.len() == 1
                    {
                        // Likely an extern "C" function (FFI call)
                        self.ops.push(UnsafeOp::FFICall);
                    }
                }
            }
            // Method calls that might be unsafe operations
            Expr::MethodCall(method) => {
                let method_name = method.method.to_string();
                match method_name.as_str() {
                    "read" | "read_volatile" | "read_unaligned" => {
                        self.ops.push(UnsafeOp::RawPointerRead);
                    }
                    "write" | "write_volatile" | "write_unaligned" => {
                        self.ops.push(UnsafeOp::RawPointerWrite);
                    }
                    "offset" | "add" | "sub" | "wrapping_offset" | "wrapping_add"
                    | "wrapping_sub" => {
                        self.ops.push(UnsafeOp::PointerArithmetic);
                    }
                    _ => {}
                }
            }
            // Assignment to dereferenced pointers
            Expr::Assign(assign) => {
                if matches!(&*assign.left, Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)))
                {
                    self.ops.push(UnsafeOp::RawPointerWrite);
                }
            }
            // Dereference operations (reading)
            Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                // Context-dependent: could be read or write
                // Conservative: assume read unless in assignment LHS
                self.ops.push(UnsafeOp::RawPointerRead);
            }
            // Path expressions that might be mutable statics
            Expr::Path(path) => {
                let path_str = quote::quote!(#path).to_string();
                // Heuristic: all-caps paths might be statics
                if path_str.chars().all(|c| c.is_uppercase() || c == '_') {
                    self.ops.push(UnsafeOp::MutableStatic);
                }
            }
            // Union field access
            Expr::Field(_field) => {
                // Union field reads are unsafe
                self.ops.push(UnsafeOp::UnionFieldRead);
            }
            _ => {}
        }
        syn::visit::visit_expr(self, expr);
    }
}

/// Classify unsafe operations in a block
pub fn classify_unsafe_operations(block: &Block) -> Vec<UnsafeOp> {
    UnsafeOpCollector::collect(block)
}

/// Result of analyzing an unsafe block
pub struct UnsafeAnalysisResult {
    /// Whether the unsafe block has impure operations
    pub has_impure: bool,
    /// Whether the unsafe block has pure-only operations
    pub has_pure_unsafe: bool,
}

/// Analyze an unsafe block and classify its purity
pub fn analyze_unsafe_block(block: &Block) -> UnsafeAnalysisResult {
    let ops = classify_unsafe_operations(block);

    let has_impure = ops.iter().any(|op| {
        matches!(
            op,
            UnsafeOp::FFICall
                | UnsafeOp::RawPointerWrite
                | UnsafeOp::MutableStatic
                | UnsafeOp::UnionFieldWrite
        )
    });

    UnsafeAnalysisResult {
        has_impure,
        has_pure_unsafe: !has_impure && !ops.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_transmute_is_pure() {
        let block: Block = parse_quote!({ std::mem::transmute(x) });
        let ops = classify_unsafe_operations(&block);
        assert!(ops.contains(&UnsafeOp::Transmute));

        let result = analyze_unsafe_block(&block);
        assert!(!result.has_impure);
        assert!(result.has_pure_unsafe);
    }

    #[test]
    fn test_pointer_read_is_pure() {
        let block: Block = parse_quote!({ ptr.read() });
        let ops = classify_unsafe_operations(&block);
        assert!(ops.contains(&UnsafeOp::RawPointerRead));

        let result = analyze_unsafe_block(&block);
        assert!(!result.has_impure);
        assert!(result.has_pure_unsafe);
    }

    #[test]
    fn test_pointer_write_is_impure() {
        let block: Block = parse_quote!({ ptr.write(value) });
        let ops = classify_unsafe_operations(&block);
        assert!(ops.contains(&UnsafeOp::RawPointerWrite));

        let result = analyze_unsafe_block(&block);
        assert!(result.has_impure);
        assert!(!result.has_pure_unsafe);
    }

    #[test]
    fn test_pointer_arithmetic_is_pure() {
        let block: Block = parse_quote!({ ptr.offset(1) });
        let ops = classify_unsafe_operations(&block);
        assert!(ops.contains(&UnsafeOp::PointerArithmetic));

        let result = analyze_unsafe_block(&block);
        assert!(!result.has_impure);
        assert!(result.has_pure_unsafe);
    }
}
