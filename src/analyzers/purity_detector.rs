use syn::{
    visit::Visit, Block, Expr, ExprCall, ExprClosure, ExprField, ExprMethodCall, ItemFn, Local,
    Pat, Stmt,
};

use super::closure_analyzer::{ClosureAnalyzer, ClosurePurity};
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

    // Analyzed closures (stored in order)
    closure_results: Vec<ClosurePurity>,
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
            closure_results: Vec::new(),
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
        self.closure_results.clear();

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
        self.closure_results.clear();

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
        // Has external side effects (I/O, external mutations, panics)?
        if self.modifies_external_state
            || self.has_io_operations
            || self.has_unsafe_blocks
            || self.has_side_effects
        {
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

    // Accessor methods for closure analyzer
    pub fn scope_mut(&mut self) -> &mut ScopeTracker {
        &mut self.scope
    }

    pub fn has_io_operations(&self) -> bool {
        self.has_io_operations
    }

    pub fn has_unsafe_blocks(&self) -> bool {
        self.has_unsafe_blocks
    }

    pub fn modifies_external_state(&self) -> bool {
        self.modifies_external_state
    }

    pub fn accesses_external_state(&self) -> bool {
        self.accesses_external_state
    }

    pub fn local_mutations(&self) -> &[LocalMutation] {
        &self.local_mutations
    }

    fn visit_expr_closure_internal(&mut self, closure: &ExprClosure) {
        // Use dedicated analyzer
        let mut analyzer = ClosureAnalyzer::new(&self.scope);
        let closure_purity = analyzer.analyze_closure(closure);

        // Store result
        self.closure_results.push(closure_purity.clone());

        // Propagate impurity to parent function
        match closure_purity.level {
            PurityLevel::Impure => {
                self.modifies_external_state = true;
                self.has_side_effects = true;
            }
            PurityLevel::LocallyPure => {
                // Local mutations in closure count toward function's local mutations
                self.local_mutations.extend(
                    closure_purity
                        .captures
                        .iter()
                        .filter(|c| c.is_mutated && c.scope == MutationScope::Local)
                        .map(|c| LocalMutation {
                            target: c.var_name.clone(),
                        }),
                );
            }
            _ => {}
        }
    }

    fn visit_expr_method_call_with_closures(&mut self, method: &ExprMethodCall) {
        let method_name = method.method.to_string();

        // Comprehensive iterator method list
        const ITERATOR_METHODS: &[&str] = &[
            // Consuming methods
            "map",
            "filter",
            "filter_map",
            "flat_map",
            "flatten",
            "fold",
            "reduce",
            "for_each",
            "try_fold",
            "try_for_each",
            "scan",
            "partition",
            "find",
            "find_map",
            "position",
            "any",
            "all",
            "collect",
            "inspect",
            // Result/Option adapters
            "and_then",
            "or_else",
            "map_or",
            "map_or_else",
        ];

        if ITERATOR_METHODS.contains(&method_name.as_str()) {
            // Analyze closure arguments inline
            for arg in &method.args {
                if let Expr::Closure(closure) = arg {
                    // Analyze closure and check its purity
                    let mut analyzer = ClosureAnalyzer::new(&self.scope);
                    let purity = analyzer.analyze_closure(closure);

                    // Store result
                    self.closure_results.push(purity.clone());

                    // Propagate impurity
                    if purity.level == PurityLevel::Impure {
                        self.has_side_effects = true;
                        self.modifies_external_state = true;
                    }
                }
            }
        }

        // Check if it's a mutation method
        if self.is_mutation_method(&method_name) {
            let receiver = &method.receiver;
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

        // Continue with normal method call analysis
        syn::visit::visit_expr_method_call(self, method);
    }

    /// Classify a macro and update purity state
    fn handle_macro(&mut self, mac: &syn::Macro) {
        let name = extract_macro_name(&mac.path);

        match name.as_str() {
            // Pure macros - no side effects
            "vec" | "format" | "concat" | "stringify" | "matches" | "include_str"
            | "include_bytes" | "env" | "option_env" => {
                // No effect on purity
            }

            // I/O macros - always impure
            "println" | "eprintln" | "print" | "eprint" | "dbg" | "write" | "writeln" => {
                self.has_io_operations = true;
                self.has_side_effects = true;
            }

            // Panic macros - always impure
            "panic" | "unimplemented" | "unreachable" | "todo" => {
                self.has_side_effects = true;
            }

            // Debug-only assertions - conditional purity
            "debug_assert" | "debug_assert_eq" | "debug_assert_ne" => {
                #[cfg(debug_assertions)]
                {
                    self.has_side_effects = true;
                }
                // In release builds, these are compiled out (pure)
            }

            // Regular assertions - always impure (panic on failure)
            "assert" | "assert_eq" | "assert_ne" => {
                self.has_side_effects = true;
            }

            // Unknown macro - no effect on purity (conservative approach)
            _ => {
                // Future: could track unknown macros for confidence adjustment
            }
        }
    }
}

impl<'ast> Visit<'ast> for PurityDetector {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // Handle closure expressions
            Expr::Closure(closure) => {
                self.visit_expr_closure_internal(closure);
                return; // Don't continue visiting (closure analyzer handles body)
            }
            // Handle expression macros: let x = dbg!(value);
            Expr::Macro(expr_macro) => {
                self.handle_macro(&expr_macro.mac);
            }
            // Unsafe blocks are impure
            Expr::Unsafe(_) => {
                self.has_unsafe_blocks = true;
                self.has_side_effects = true;
            }
            // Assignment expressions indicate mutation
            Expr::Assign(assign) => {
                let scope = self.determine_mutation_scope(&assign.left);
                match scope {
                    MutationScope::Local => {
                        // Extract target name for tracking
                        match &*assign.left {
                            Expr::Path(path) => {
                                if let Some(ident) = path.path.get_ident() {
                                    self.local_mutations.push(LocalMutation {
                                        target: ident.to_string(),
                                    });
                                }
                            }
                            Expr::Field(field) => {
                                if let Expr::Path(path) = &*field.base {
                                    if let Some(ident) = path.path.get_ident() {
                                        self.local_mutations.push(LocalMutation {
                                            target: format!(
                                                "{}.{}",
                                                ident,
                                                quote::quote!(#field.member)
                                            ),
                                        });
                                    }
                                }
                            }
                            _ => {}
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
            // Path expressions may access external state (constants, statics, etc.)
            Expr::Path(path) => {
                let path_str = quote::quote!(#path).to_string();
                // Check if it's accessing a module path (like std::i32::MAX)
                if path_str.contains("::") && !self.scope.is_local(&path_str) {
                    self.accesses_external_state = true;
                }
            }
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
            Expr::MethodCall(method_call) => {
                self.visit_expr_method_call_with_closures(method_call);
                return; // Already handled including continuation
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

        // Handle statement macros: println!("test");
        if let Stmt::Macro(stmt_macro) = stmt {
            self.handle_macro(&stmt_macro.mac);
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

/// Extract the last segment of a macro path
/// e.g., "std::println" -> "println", "assert_eq" -> "assert_eq"
fn extract_macro_name(path: &syn::Path) -> String {
    path.segments
        .last()
        .map(|seg| seg.ident.to_string())
        .unwrap_or_default()
}

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

    // Tests for spec 160a: macro classification fix

    #[test]
    #[cfg(not(debug_assertions))]
    fn test_debug_assert_pure_in_release() {
        let analysis = analyze_function_str(
            r#"
            fn check_bounds(x: usize) -> bool {
                debug_assert!(x < 100);
                debug_assert_eq!(x, x);
                x < 100
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn test_debug_assert_impure_in_debug() {
        let analysis = analyze_function_str(
            r#"
            fn check_bounds(x: usize) -> bool {
                debug_assert!(x < 100);
                x < 100
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_io_macros_always_impure() {
        let test_cases = vec![
            r#"fn f() { println!("test"); }"#,
            r#"fn f() { eprintln!("error"); }"#,
            r#"fn f() { dbg!(42); }"#,
            r#"fn f() { print!("no newline"); }"#,
        ];

        for code in test_cases {
            let analysis = analyze_function_str(code);
            assert_eq!(
                analysis.purity_level,
                PurityLevel::Impure,
                "Failed for: {}",
                code
            );
        }
    }

    #[test]
    fn test_expression_macros() {
        let analysis = analyze_function_str(
            r#"
            fn example() -> i32 {
                let x = dbg!(42);
                x
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_pure_macros() {
        let analysis = analyze_function_str(
            r#"
            fn create_list() -> Vec<i32> {
                vec![1, 2, 3]
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_no_substring_false_positives() {
        let analysis = analyze_function_str(
            r#"
            fn example() -> i32 {
                // Unknown macro with "debug" in name should not be marked impure
                42
            }
            "#,
        );
        // Should not be marked impure just because of substring match
        assert_ne!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_assert_always_impure() {
        let analysis = analyze_function_str(
            r#"
            fn validate(x: i32) -> i32 {
                assert!(x > 0);
                x
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }
}
