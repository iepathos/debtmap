use syn::{visit::Visit, Block, Expr, ExprCall, ExprMethodCall, ItemFn, Pat, Stmt};

/// Detects whether a function is pure through static analysis
pub struct PurityDetector {
    has_side_effects: bool,
    has_mutable_params: bool,
    has_io_operations: bool,
    has_unsafe_blocks: bool,
    accesses_external_state: bool,
    modifies_external_state: bool,
}

impl Default for PurityDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PurityDetector {
    pub fn new() -> Self {
        Self {
            has_side_effects: false,
            has_mutable_params: false,
            has_io_operations: false,
            has_unsafe_blocks: false,
            accesses_external_state: false,
            modifies_external_state: false,
        }
    }

    /// Analyzes a function to determine if it's pure
    pub fn is_pure_function(&mut self, item_fn: &ItemFn) -> PurityAnalysis {
        // Reset state
        self.has_side_effects = false;
        self.has_mutable_params = false;
        self.has_io_operations = false;
        self.has_unsafe_blocks = false;
        self.accesses_external_state = false;
        self.modifies_external_state = false;

        // Check function signature for mutable parameters
        for arg in &item_fn.sig.inputs {
            if let syn::FnArg::Typed(pat_type) = arg {
                // Check if the type itself is a mutable reference
                if self.type_has_mutable_reference(&pat_type.ty) {
                    self.has_mutable_params = true;
                }
                // Also check if the pattern indicates mutability
                if self.has_mutable_reference(&pat_type.pat) {
                    self.has_mutable_params = true;
                }
            }
        }

        // Visit the function body
        self.visit_block(&item_fn.block);

        PurityAnalysis {
            is_pure: !self.has_side_effects
                && !self.has_mutable_params
                && !self.has_io_operations
                && !self.has_unsafe_blocks
                && !self.modifies_external_state,
            reasons: self.collect_impurity_reasons(),
            confidence: self.calculate_confidence(),
        }
    }

    /// Analyzes a block to determine if it's pure
    pub fn is_pure_block(&mut self, block: &Block) -> PurityAnalysis {
        // Reset state
        self.has_side_effects = false;
        self.has_mutable_params = false;
        self.has_io_operations = false;
        self.has_unsafe_blocks = false;
        self.accesses_external_state = false;
        self.modifies_external_state = false;

        self.visit_block(block);

        PurityAnalysis {
            is_pure: !self.has_side_effects
                && !self.has_io_operations
                && !self.has_unsafe_blocks
                && !self.modifies_external_state,
            reasons: self.collect_impurity_reasons(),
            confidence: self.calculate_confidence(),
        }
    }

    fn has_mutable_reference(&self, pat: &Pat) -> bool {
        // Check if the pattern contains mutable references
        match pat {
            Pat::Type(pat_type) => self.type_has_mutable_reference(&pat_type.ty),
            Pat::Ident(pat_ident) => {
                // Check if the identifier itself is mutable
                pat_ident.mutability.is_some()
            }
            _ => false,
        }
    }

    fn type_has_mutable_reference(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Reference(type_ref) => {
                // Check if this is a mutable reference (&mut T)
                type_ref.mutability.is_some()
            }
            _ => false,
        }
    }

    fn collect_impurity_reasons(&self) -> Vec<ImpurityReason> {
        let mut reasons = Vec::new();

        if self.has_side_effects {
            reasons.push(ImpurityReason::SideEffects);
        }
        if self.has_mutable_params {
            reasons.push(ImpurityReason::MutableParameters);
        }
        if self.has_io_operations {
            reasons.push(ImpurityReason::IOOperations);
        }
        if self.has_unsafe_blocks {
            reasons.push(ImpurityReason::UnsafeCode);
        }
        if self.modifies_external_state {
            reasons.push(ImpurityReason::ModifiesExternalState);
        }
        if self.accesses_external_state {
            reasons.push(ImpurityReason::AccessesExternalState);
        }

        reasons
    }

    fn calculate_confidence(&self) -> f32 {
        // Start with high confidence
        let mut confidence = 1.0;

        // Reduce confidence if we only access external state (might be reading constants)
        if self.accesses_external_state && !self.modifies_external_state {
            confidence *= 0.8;
        }

        // If no impurities detected, we're fairly confident
        if !self.has_side_effects
            && !self.has_io_operations
            && !self.has_unsafe_blocks
            && !self.modifies_external_state
        {
            confidence = 0.95; // High confidence but not 100% due to potential false negatives
        }

        confidence
    }

    fn is_io_call(&self, path_str: &str) -> bool {
        const IO_PATTERNS: &[&str] = &[
            "print",
            "write",
            "read",
            "File",
            "stdin",
            "stdout",
            "stderr",
            "fs::",
            "io::",
            "net::",
            "TcpStream",
            "UdpSocket",
            "reqwest",
            "tokio",
        ];

        IO_PATTERNS.iter().any(|pattern| path_str.contains(pattern))
    }

    fn is_mutation_method(&self, method_name: &str) -> bool {
        // Common mutation methods
        matches!(
            method_name,
            "push"
                | "pop"
                | "insert"
                | "remove"
                | "clear"
                | "append"
                | "extend"
                | "retain"
                | "truncate"
                | "swap"
                | "reverse"
                | "sort"
                | "sort_by"
                | "dedup"
                | "drain"
                | "split_off"
        )
    }
}

impl<'ast> Visit<'ast> for PurityDetector {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // Function calls might have side effects
            Expr::Call(ExprCall { func, .. }) => {
                if let Expr::Path(expr_path) = &**func {
                    let path_str = quote::quote!(#expr_path).to_string();
                    if self.is_io_call(&path_str) {
                        self.has_io_operations = true;
                        self.has_side_effects = true;
                    }
                }
            }
            // Method calls might have side effects
            Expr::MethodCall(ExprMethodCall {
                method, receiver, ..
            }) => {
                let method_name = method.to_string();

                // Check if it's a mutation method
                if self.is_mutation_method(&method_name) {
                    // Check if we're mutating self or external data
                    if let Expr::Path(path) = &**receiver {
                        let path_str = quote::quote!(#path).to_string();
                        if !path_str.starts_with("self") {
                            self.modifies_external_state = true;
                            self.has_side_effects = true;
                        }
                    } else {
                        // Conservative: assume it might modify external state
                        self.modifies_external_state = true;
                        self.has_side_effects = true;
                    }
                }

                // Check for I/O methods
                if method_name.contains("write")
                    || method_name.contains("print")
                    || method_name.contains("flush")
                {
                    self.has_io_operations = true;
                    self.has_side_effects = true;
                }
            }
            // Unsafe blocks are impure by definition
            Expr::Unsafe(_) => {
                self.has_unsafe_blocks = true;
                self.has_side_effects = true;
            }
            // Static variable access might be impure
            Expr::Path(expr_path) => {
                let path_str = quote::quote!(#expr_path).to_string();
                // Check if accessing static or external state
                if path_str.contains("::")
                    && !path_str.starts_with("self")
                    && !path_str.starts_with("Self")
                {
                    // This might be accessing external state
                    self.accesses_external_state = true;
                }
            }
            // Assignment to external state
            Expr::Assign(assign) => {
                if let Expr::Path(path) = &*assign.left {
                    let path_str = quote::quote!(#path).to_string();
                    if !path_str.starts_with("self") {
                        self.modifies_external_state = true;
                        self.has_side_effects = true;
                    }
                }
            }
            _ => {}
        }

        // Continue visiting nested expressions
        syn::visit::visit_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        if let Stmt::Macro(stmt_macro) = stmt {
            let macro_path = stmt_macro.mac.path.to_token_stream().to_string();
            // Macros that typically have side effects
            if macro_path.contains("print")
                || macro_path.contains("panic")
                || macro_path.contains("assert")
                || macro_path.contains("debug")
                || macro_path.contains("log")
                || macro_path.contains("trace")
                || macro_path.contains("info")
                || macro_path.contains("warn")
                || macro_path.contains("error")
            {
                self.has_io_operations = true;
                self.has_side_effects = true;
            }
        }

        syn::visit::visit_stmt(self, stmt);
    }

    fn visit_block(&mut self, block: &'ast Block) {
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
    }
}

#[derive(Debug, Clone)]
pub struct PurityAnalysis {
    pub is_pure: bool,
    pub reasons: Vec<ImpurityReason>,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImpurityReason {
    SideEffects,
    MutableParameters,
    IOOperations,
    UnsafeCode,
    ModifiesExternalState,
    AccessesExternalState,
}

impl ImpurityReason {
    pub fn description(&self) -> &str {
        match self {
            Self::SideEffects => "Function has side effects",
            Self::MutableParameters => "Function takes mutable parameters",
            Self::IOOperations => "Function performs I/O operations",
            Self::UnsafeCode => "Function contains unsafe code",
            Self::ModifiesExternalState => "Function modifies external state",
            Self::AccessesExternalState => "Function accesses external state",
        }
    }
}

use quote::ToTokens;

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze_function_str(code: &str) -> PurityAnalysis {
        let item_fn = syn::parse_str::<ItemFn>(code).unwrap();
        let mut detector = PurityDetector::new();
        detector.is_pure_function(&item_fn)
    }

    #[test]
    fn test_pure_function() {
        let analysis = analyze_function_str(
            r#"
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
            "#,
        );
        assert!(analysis.is_pure);
        assert!(analysis.reasons.is_empty());
    }

    #[test]
    fn test_function_with_print() {
        let analysis = analyze_function_str(
            r#"
            fn debug_add(a: i32, b: i32) -> i32 {
                println!("Adding {} + {}", a, b);
                a + b
            }
            "#,
        );
        assert!(!analysis.is_pure);
        assert!(analysis.reasons.contains(&ImpurityReason::IOOperations));
    }

    #[test]
    fn test_function_with_mutable_param() {
        let analysis = analyze_function_str(
            r#"
            fn increment(x: &mut i32) {
                *x += 1;
            }
            "#,
        );
        assert!(!analysis.is_pure);
        assert!(analysis
            .reasons
            .contains(&ImpurityReason::MutableParameters));
    }

    #[test]
    fn test_function_with_unsafe() {
        let analysis = analyze_function_str(
            r#"
            fn dangerous() -> i32 {
                unsafe {
                    std::ptr::null::<i32>().read()
                }
            }
            "#,
        );
        assert!(!analysis.is_pure);
        assert!(analysis.reasons.contains(&ImpurityReason::UnsafeCode));
    }
}
