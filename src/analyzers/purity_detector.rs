use syn::{
    visit::Visit, Block, Expr, ExprCall, ExprField, ExprMethodCall, ItemFn, Local, Pat, Stmt,
};

use super::scope_tracker::{ScopeTracker, SelfKind};
use crate::core::PurityLevel;

/// Detects whether a function is pure through static analysis
pub struct PurityDetector {
    has_side_effects: bool,
    has_mutable_params: bool,
    has_io_operations: bool,
    has_unsafe_blocks: bool,
    accesses_external_state: bool,
    modifies_external_state: bool,

    // Scope-aware mutation tracking
    scope: ScopeTracker,
    local_mutations: Vec<LocalMutation>,
    upvalue_mutations: Vec<UpvalueMutation>,
}

/// A mutation of a local variable or owned parameter
#[derive(Debug, Clone)]
pub struct LocalMutation {
    pub target: String,
}

/// A mutation of a closure-captured variable
#[derive(Debug, Clone)]
pub struct UpvalueMutation {
    pub captured_var: String,
}

/// Classification of mutation scope
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationScope {
    /// Mutation of local variable or owned parameter
    Local,
    /// Mutation of closure-captured variable
    Upvalue,
    /// Mutation of external state (fields, statics, etc.)
    External,
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
            scope: ScopeTracker::new(),
            local_mutations: Vec::new(),
            upvalue_mutations: Vec::new(),
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
        self.scope = ScopeTracker::new();
        self.local_mutations.clear();
        self.upvalue_mutations.clear();

        // Initialize scope with parameters
        for arg in &item_fn.sig.inputs {
            self.scope.add_parameter(arg);

            // Check function signature for mutable parameters
            match arg {
                syn::FnArg::Receiver(receiver) => {
                    // Check if receiver is &mut self
                    if receiver.reference.is_some() && receiver.mutability.is_some() {
                        self.has_mutable_params = true;
                        self.modifies_external_state = true;
                    }
                }
                syn::FnArg::Typed(pat_type) => {
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
        }

        // Visit the function body
        self.visit_block(&item_fn.block);

        let purity_level = self.determine_purity_level();

        PurityAnalysis {
            is_pure: !self.has_side_effects
                && !self.has_mutable_params
                && !self.has_io_operations
                && !self.has_unsafe_blocks
                && !self.modifies_external_state,
            purity_level,
            reasons: self.collect_impurity_reasons(),
            confidence: self.calculate_confidence_score(),
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
        self.scope = ScopeTracker::new();
        self.local_mutations.clear();
        self.upvalue_mutations.clear();

        self.visit_block(block);

        let purity_level = self.determine_purity_level();

        PurityAnalysis {
            is_pure: !self.has_side_effects
                && !self.has_io_operations
                && !self.has_unsafe_blocks
                && !self.modifies_external_state,
            purity_level,
            reasons: self.collect_impurity_reasons(),
            confidence: self.calculate_confidence_score(),
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

    fn determine_mutation_scope(&self, expr: &Expr) -> MutationScope {
        match expr {
            // Simple identifier: x = value
            Expr::Path(path) => {
                let ident = path
                    .path
                    .get_ident()
                    .map(|i| i.to_string())
                    .unwrap_or_default();

                if self.scope.is_local(&ident) {
                    MutationScope::Local
                } else {
                    // Conservative: assume external
                    MutationScope::External
                }
            }

            // Field access: obj.field = value
            Expr::Field(field) => self.determine_field_mutation_scope(field),

            // Index: arr[i] = value
            Expr::Index(index) => {
                if let Expr::Path(path) = &*index.expr {
                    if let Some(ident) = path.path.get_ident() {
                        if self.scope.is_local(&ident.to_string()) {
                            return MutationScope::Local;
                        }
                    }
                }
                MutationScope::External
            }

            // Pointer dereference: *ptr = value
            Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                // Conservative: assume external
                MutationScope::External
            }

            _ => MutationScope::External,
        }
    }

    fn determine_field_mutation_scope(&self, field: &ExprField) -> MutationScope {
        match &*field.base {
            // self.field = value
            Expr::Path(path)
                if self.scope.is_self(
                    &path
                        .path
                        .get_ident()
                        .map(|i| i.to_string())
                        .unwrap_or_default(),
                ) =>
            {
                // Check self kind
                if let Some(self_kind) = self.scope.get_self_kind() {
                    match self_kind {
                        SelfKind::MutRef => MutationScope::External, // &mut self
                        SelfKind::Owned | SelfKind::MutOwned => {
                            // mut self or self (owned) - local mutation
                            MutationScope::Local
                        }
                        _ => MutationScope::External,
                    }
                } else {
                    MutationScope::External
                }
            }

            // local_var.field = value
            Expr::Path(path) => {
                let ident = path
                    .path
                    .get_ident()
                    .map(|i| i.to_string())
                    .unwrap_or_default();

                if self.scope.is_local(&ident) {
                    MutationScope::Local
                } else {
                    MutationScope::External
                }
            }

            _ => MutationScope::External,
        }
    }

    fn determine_purity_level(&self) -> PurityLevel {
        // Has external side effects (I/O, external mutations)?
        if self.modifies_external_state || self.has_io_operations || self.has_unsafe_blocks {
            return PurityLevel::Impure;
        }

        // Has local mutations?
        if !self.local_mutations.is_empty() || !self.upvalue_mutations.is_empty() {
            return PurityLevel::LocallyPure;
        }

        // Only reads external state?
        if self.accesses_external_state {
            return PurityLevel::ReadOnly;
        }

        // No mutations or side effects at all
        PurityLevel::StrictlyPure
    }

    fn calculate_confidence_score(&self) -> f32 {
        let mut confidence: f32 = 1.0;

        // Reduce confidence if we only access external state
        if self.accesses_external_state && !self.modifies_external_state {
            confidence *= 0.8;
        }

        // Reduce confidence for upvalue mutations (closures)
        if !self.upvalue_mutations.is_empty() {
            confidence *= 0.85;
        }

        // High confidence for simple local mutations
        if !self.local_mutations.is_empty() && self.local_mutations.len() < 5 {
            confidence *= 0.95;
        }

        // If no impurities detected, high confidence
        if !self.has_side_effects
            && !self.has_io_operations
            && !self.has_unsafe_blocks
            && !self.modifies_external_state
        {
            confidence = 0.95;
        }

        confidence.clamp(0.5, 1.0)
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
                    // Classify the mutation scope
                    let scope = self.determine_mutation_scope(receiver);
                    match scope {
                        MutationScope::Local => {
                            if let Expr::Path(path) = &**receiver {
                                if let Some(ident) = path.path.get_ident() {
                                    self.local_mutations.push(LocalMutation {
                                        target: ident.to_string(),
                                    });
                                }
                            }
                        }
                        MutationScope::Upvalue => {
                            if let Expr::Path(path) = &**receiver {
                                if let Some(ident) = path.path.get_ident() {
                                    self.upvalue_mutations.push(UpvalueMutation {
                                        captured_var: ident.to_string(),
                                    });
                                }
                            }
                        }
                        MutationScope::External => {
                            self.modifies_external_state = true;
                            self.has_side_effects = true;
                        }
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
                let scope = self.determine_mutation_scope(&assign.left);
                match scope {
                    MutationScope::Local => {
                        // Extract target name from the expression
                        let target_name = match &*assign.left {
                            Expr::Path(path) => path.path.get_ident().map(|i| i.to_string()),
                            Expr::Field(field) => {
                                // For field access like self.value, use "self.field"
                                if let Expr::Path(path) = &*field.base {
                                    path.path.get_ident().map(|i| {
                                        if let syn::Member::Named(field_name) = &field.member {
                                            format!("{}.{}", i, field_name)
                                        } else {
                                            i.to_string()
                                        }
                                    })
                                } else {
                                    Some("field".to_string())
                                }
                            }
                            _ => Some("unknown".to_string()),
                        };

                        if let Some(target) = target_name {
                            self.local_mutations.push(LocalMutation { target });
                        }
                    }
                    MutationScope::Upvalue => {
                        if let Expr::Path(path) = &*assign.left {
                            if let Some(ident) = path.path.get_ident() {
                                self.upvalue_mutations.push(UpvalueMutation {
                                    captured_var: ident.to_string(),
                                });
                            }
                        }
                    }
                    MutationScope::External => {
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
        // Track local variable declarations
        if let Stmt::Local(local) = stmt {
            self.visit_local(local);
        }

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

    fn visit_local(&mut self, local: &'ast Local) {
        // Extract variable name from pattern
        if let Pat::Ident(pat_ident) = &local.pat {
            self.scope.add_local_var(pat_ident.ident.to_string());
        }
        syn::visit::visit_local(self, local);
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
    pub purity_level: PurityLevel,
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

    #[test]
    fn test_local_mutation_is_locally_pure() {
        let analysis = analyze_function_str(
            r#"
            fn process_data(input: Vec<i32>) -> Vec<i32> {
                let mut result = Vec::new();
                for item in input {
                    result.push(item * 2);
                }
                result
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::LocallyPure);
        assert!(analysis.confidence > 0.85);
    }

    #[test]
    fn test_builder_pattern_is_locally_pure() {
        let analysis = analyze_function_str(
            r#"
            fn with_value(mut self, value: u32) -> Self {
                self.value = value;
                self
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::LocallyPure);
    }

    #[test]
    fn test_strictly_pure_function() {
        let analysis = analyze_function_str(
            r#"
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_read_only_function() {
        let analysis = analyze_function_str(
            r#"
            fn is_valid(x: i32) -> bool {
                x < std::i32::MAX
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::ReadOnly);
    }

    #[test]
    fn test_external_mutation_is_impure() {
        let analysis = analyze_function_str(
            r#"
            fn increment(&mut self) {
                self.count += 1;
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }
}
