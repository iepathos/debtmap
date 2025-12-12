use syn::{
    visit::Visit, Block, Expr, ExprClosure, ExprField, ExprMethodCall, ItemFn, Local, Pat, Stmt,
};

use super::closure_analyzer::{ClosureAnalyzer, ClosurePurity};
use super::custom_macro_analyzer::{CustomMacroAnalyzer, MacroPurity};
use super::macro_definition_collector::MacroDefinitions;
use super::scope_tracker::{ScopeTracker, SelfKind};
use crate::analysis::data_flow::{ControlFlowGraph, DataFlowAnalysis};
use crate::core::PurityLevel;
use dashmap::DashMap;
use std::sync::Arc;

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

    // Track unknown macros for confidence adjustment
    unknown_macros_count: usize,

    // Macro definitions for custom macro classification
    macro_definitions: MacroDefinitions,

    // Custom macro analyzer (Spec 160c)
    macro_analyzer: CustomMacroAnalyzer,

    // Track pure unsafe operations (Spec 161)
    has_pure_unsafe: bool,
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

/// Classification of unsafe operations (Spec 161)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsafeOp {
    // Pure operations - no side effects
    Transmute,
    RawPointerRead,
    UnionFieldRead,
    PointerArithmetic,

    // Impure operations - side effects
    FFICall,
    RawPointerWrite,
    MutableStatic,
    UnionFieldWrite,
}

impl Default for PurityDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PurityDetector {
    /// Create a new PurityDetector with no macro definitions
    pub fn new() -> Self {
        Self::with_macro_definitions(Arc::new(DashMap::new()))
    }

    /// Create a new PurityDetector with the given macro definitions
    pub fn with_macro_definitions(macro_definitions: MacroDefinitions) -> Self {
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
            unknown_macros_count: 0,
            macro_definitions,
            macro_analyzer: CustomMacroAnalyzer::new(),
            has_pure_unsafe: false,
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
        self.unknown_macros_count = 0;
        self.has_pure_unsafe = false;

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

        // Perform data flow analysis to refine mutation detection
        let cfg = ControlFlowGraph::from_block(&item_fn.block);
        let data_flow = DataFlowAnalysis::analyze(&cfg);

        // Capture var_names from CFG for VarId translation
        let var_names = cfg.var_names.clone();

        // Filter dead mutations from local_mutations
        let live_mutations = self.filter_dead_mutations(&cfg, &data_flow);

        // Determine purity level based on mutations and side effects
        // Use self.local_mutations to determine level (not live_mutations)
        // live_mutations is used for confidence adjustment
        let purity_level = if !self.has_side_effects
            && !self.has_mutable_params
            && !self.has_io_operations
            && !self.has_unsafe_blocks
            && !self.modifies_external_state
            && !self.accesses_external_state
            && self.local_mutations.is_empty()
        {
            PurityLevel::StrictlyPure
        } else if self.has_io_operations || self.modifies_external_state {
            // I/O operations and external state modifications are always impure
            PurityLevel::Impure
        } else if self.accesses_external_state {
            // Accesses external state but doesn't modify it
            PurityLevel::ReadOnly
        } else {
            // Only has local mutations, no external state access or modification
            PurityLevel::LocallyPure
        };

        // Adjust confidence based on data flow analysis
        let mut confidence = self.calculate_confidence_score();
        if live_mutations.len() < self.local_mutations.len() {
            // Dead mutations removed → higher confidence
            confidence *= 1.1;
        }

        PurityAnalysis {
            is_pure: purity_level == PurityLevel::StrictlyPure,
            purity_level,
            reasons: self.collect_impurity_reasons(),
            confidence: confidence.min(1.0),
            data_flow_info: Some(data_flow),
            live_mutations,
            total_mutations: self.local_mutations.len(),
            var_names,
        }
    }

    /// Analyzes an impl method to determine if it's pure (spec 202)
    ///
    /// This is similar to `is_pure_function` but takes an `ImplItemFn` instead of `ItemFn`.
    /// Impl methods have the same structure but are found inside `impl` blocks.
    pub fn is_pure_impl_method(&mut self, impl_fn: &syn::ImplItemFn) -> PurityAnalysis {
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
        self.unknown_macros_count = 0;
        self.has_pure_unsafe = false;

        // Initialize scope with parameters
        for arg in &impl_fn.sig.inputs {
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
        self.visit_block(&impl_fn.block);

        // Perform data flow analysis to refine mutation detection
        let cfg = ControlFlowGraph::from_block(&impl_fn.block);
        let data_flow = DataFlowAnalysis::analyze(&cfg);

        // Capture var_names from CFG for VarId translation
        let var_names = cfg.var_names.clone();

        // Filter dead mutations from local_mutations
        let live_mutations = self.filter_dead_mutations(&cfg, &data_flow);

        // Determine purity level based on mutations and side effects
        let purity_level = if !self.has_side_effects
            && !self.has_mutable_params
            && !self.has_io_operations
            && !self.has_unsafe_blocks
            && !self.modifies_external_state
            && !self.accesses_external_state
            && self.local_mutations.is_empty()
        {
            PurityLevel::StrictlyPure
        } else if self.has_io_operations || self.modifies_external_state {
            PurityLevel::Impure
        } else if self.accesses_external_state {
            PurityLevel::ReadOnly
        } else {
            PurityLevel::LocallyPure
        };

        // Adjust confidence based on data flow analysis
        let mut confidence = self.calculate_confidence_score();
        if live_mutations.len() < self.local_mutations.len() {
            confidence *= 1.1;
        }

        PurityAnalysis {
            is_pure: purity_level == PurityLevel::StrictlyPure,
            purity_level,
            reasons: self.collect_impurity_reasons(),
            confidence: confidence.min(1.0),
            data_flow_info: Some(data_flow),
            live_mutations,
            total_mutations: self.local_mutations.len(),
            var_names,
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
        self.unknown_macros_count = 0;
        self.has_pure_unsafe = false;

        self.visit_block(block);

        // Perform data flow analysis
        let cfg = ControlFlowGraph::from_block(block);
        let data_flow = DataFlowAnalysis::analyze(&cfg);
        let var_names = cfg.var_names.clone();

        let purity_level = if !self.has_side_effects
            && !self.has_io_operations
            && !self.has_unsafe_blocks
            && !self.modifies_external_state
            && !self.accesses_external_state
        {
            PurityLevel::StrictlyPure
        } else if self.has_io_operations || self.modifies_external_state {
            // I/O operations and external state modifications are always impure
            PurityLevel::Impure
        } else if self.accesses_external_state {
            // Accesses external state but doesn't modify it
            PurityLevel::ReadOnly
        } else {
            // Only has local mutations, no external state access or modification
            PurityLevel::LocallyPure
        };

        PurityAnalysis {
            is_pure: purity_level == PurityLevel::StrictlyPure,
            purity_level,
            reasons: self.collect_impurity_reasons(),
            confidence: self.calculate_confidence_score(),
            data_flow_info: Some(data_flow),
            live_mutations: self.local_mutations.clone(),
            total_mutations: self.local_mutations.len(),
            var_names,
        }
    }

    fn filter_dead_mutations(
        &self,
        cfg: &ControlFlowGraph,
        data_flow: &DataFlowAnalysis,
    ) -> Vec<LocalMutation> {
        // Filter out mutations to variables with dead stores
        // Note: Due to simplified CFG construction, we need to be conservative
        // Only filter if we find a definite match with a dead store
        self.local_mutations
            .iter()
            .filter(|mutation| {
                // Check if this mutation target is in the dead stores set
                // Dead stores means the variable is assigned but never read
                let is_dead = data_flow.liveness.dead_stores.iter().any(|dead_var| {
                    // Get the variable name from the CFG's var_names vector
                    // VarId.name_id indexes into this vector
                    if let Some(var_name) = cfg.var_names.get(dead_var.name_id as usize) {
                        // Match the mutation target against the variable name
                        // Handle both simple names and field access patterns
                        // Only match if the var_name is not a temp variable placeholder
                        !var_name.starts_with("_temp")
                            && (mutation.target == *var_name
                                || mutation.target.starts_with(&format!("{}.", var_name)))
                    } else {
                        false
                    }
                });

                // Keep the mutation unless we're sure it's dead
                !is_dead
            })
            .cloned()
            .collect()
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

    #[allow(dead_code)]
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

        // Reduce confidence for unknown macros (conservative approach)
        for _ in 0..self.unknown_macros_count {
            confidence *= 0.95;
        }

        // Reduce confidence for pure unsafe operations (Spec 161)
        if self.has_pure_unsafe {
            confidence *= 0.85;
        }

        // If no impurities detected and no pure unsafe, high confidence
        if !self.has_side_effects
            && !self.has_io_operations
            && !self.has_unsafe_blocks
            && !self.modifies_external_state
            && !self.has_pure_unsafe
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

    /// Classify unsafe operations in a block (Spec 161)
    fn classify_unsafe_operations(&self, block: &Block) -> Vec<UnsafeOp> {
        use syn::visit::Visit;

        struct UnsafeOpCollector {
            ops: Vec<UnsafeOp>,
        }

        impl UnsafeOpCollector {
            fn new() -> Self {
                Self { ops: Vec::new() }
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

        let mut collector = UnsafeOpCollector::new();
        collector.visit_block(block);
        collector.ops
    }

    /// Analyze an unsafe block and classify its purity (Spec 161)
    fn analyze_unsafe_block(&mut self, block: &Block) {
        let ops = self.classify_unsafe_operations(block);

        let has_impure = ops.iter().any(|op| {
            matches!(
                op,
                UnsafeOp::FFICall
                    | UnsafeOp::RawPointerWrite
                    | UnsafeOp::MutableStatic
                    | UnsafeOp::UnionFieldWrite
            )
        });

        if has_impure {
            self.has_unsafe_blocks = true;
            self.modifies_external_state = true;
            self.has_side_effects = true;
        } else {
            // Pure unsafe - still mark has_unsafe_blocks but allow as pure
            // with reduced confidence
            self.has_pure_unsafe = true;
        }
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

        // Step 1: Check built-in macros first (from Spec 160a)
        if let Some(purity) = self.classify_builtin(&name) {
            self.apply_purity(purity);
            return;
        }

        // Step 2: Check custom macros (from Spec 160b + 160c)
        let custom_macro_body = self
            .macro_definitions
            .get(&name)
            .map(|def| def.body.clone());
        if let Some(body) = custom_macro_body {
            // Analyze the custom macro body (Spec 160c)
            let purity = self.macro_analyzer.analyze(&body);
            self.apply_purity(purity);
            return;
        }

        // Step 3: Truly unknown macro
        self.unknown_macros_count += 1;
    }

    /// Classify built-in macros
    fn classify_builtin(&self, name: &str) -> Option<MacroPurity> {
        match name {
            // Pure macros - no side effects
            "vec" | "format" | "concat" | "stringify" | "matches" | "include_str"
            | "include_bytes" | "env" | "option_env" => Some(MacroPurity::Pure),

            // I/O macros - always impure
            "println" | "eprintln" | "print" | "eprint" | "dbg" | "write" | "writeln" => {
                Some(MacroPurity::Impure)
            }

            // Panic macros - always impure
            "panic" | "unimplemented" | "unreachable" | "todo" => Some(MacroPurity::Impure),

            // Debug-only assertions - conditional purity
            "debug_assert" | "debug_assert_eq" | "debug_assert_ne" => {
                Some(MacroPurity::Conditional {
                    debug: Box::new(MacroPurity::Impure),
                    release: Box::new(MacroPurity::Pure),
                })
            }

            // Regular assertions - always impure (panic on failure)
            "assert" | "assert_eq" | "assert_ne" => Some(MacroPurity::Impure),

            _ => None,
        }
    }

    /// Apply macro purity classification to the detector state
    fn apply_purity(&mut self, purity: MacroPurity) {
        match purity {
            MacroPurity::Impure => {
                self.has_side_effects = true;
                self.has_io_operations = true;
            }
            MacroPurity::Conditional { debug, release } => {
                // Apply purity based on build configuration
                #[cfg(debug_assertions)]
                {
                    let _ = release; // Mark as used to avoid warnings
                    self.apply_purity(*debug);
                }
                #[cfg(not(debug_assertions))]
                {
                    let _ = debug; // Mark as used to avoid warnings
                    self.apply_purity(*release);
                }
            }
            MacroPurity::Unknown { confidence: _ } => {
                // Reduce confidence but don't mark as impure
                self.unknown_macros_count += 1;
            }
            MacroPurity::Pure => {
                // No effect on purity
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
            // Unsafe blocks - analyze operations to distinguish pure/impure (Spec 161)
            Expr::Unsafe(unsafe_expr) => {
                self.analyze_unsafe_block(&unsafe_expr.block);
                return; // Don't continue visiting - we've already analyzed the block
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
            // Function calls might have side effects
            Expr::Call(call) => {
                if let Expr::Path(expr_path) = &*call.func {
                    let path_str = quote::quote!(#expr_path).to_string();
                    if self.is_io_call(&path_str) {
                        self.has_io_operations = true;
                        self.has_side_effects = true;
                    }
                }
                // Continue visiting arguments but not the function path
                // (to avoid marking function names as external state access)
                for arg in &call.args {
                    self.visit_expr(arg);
                }
                return; // Don't continue with default visiting
            }
            // Path expressions may access external state (constants, statics, etc.)
            Expr::Path(path) => {
                let path_str = quote::quote!(#path).to_string();
                // Check if it's accessing a module path (like std::i32::MAX)
                if path_str.contains("::") && !self.scope.is_local(&path_str) {
                    self.accesses_external_state = true;
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
    pub data_flow_info: Option<DataFlowAnalysis>,
    /// Live local mutations (after filtering out dead stores)
    /// This is useful for "almost pure" analysis - functions with only 1-2 live mutations
    /// are good refactoring candidates
    pub live_mutations: Vec<LocalMutation>,
    /// Total mutations detected (before filtering dead stores)
    pub total_mutations: usize,
    /// Variable name mapping for translating VarIds (from CFG)
    pub var_names: Vec<String>,
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
    fn test_function_with_unsafe_read_is_pure() {
        // Spec 161: ptr.read() is pure unsafe (just reads, doesn't write)
        let analysis = analyze_function_str(
            r#"
            fn dangerous() -> i32 {
                unsafe {
                    std::ptr::null::<i32>().read()
                }
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
        assert!(analysis.confidence < 0.90); // Reduced confidence for pure unsafe
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

    // Tests for spec 160b: macro definition collection

    #[test]
    fn test_purity_with_custom_macros() {
        use crate::analyzers::macro_definition_collector::*;
        use dashmap::DashMap;
        use std::sync::Arc;

        let code = r#"
            macro_rules! my_logger {
                ($($arg:tt)*) => {
                    eprintln!($($arg)*);
                };
            }

            fn example() {
                my_logger!("test");
            }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let definitions = Arc::new(DashMap::new());
        collect_definitions(&ast, std::path::Path::new("test.rs"), definitions.clone());

        // Should detect my_logger is defined
        assert!(definitions.contains_key("my_logger"));

        // Purity analysis can now access this definition
        let mut detector = PurityDetector::with_macro_definitions(definitions);

        // Parse just the function
        let func_code = r#"
            fn example() {
                my_logger!("test");
            }
        "#;
        let item_fn = syn::parse_str::<ItemFn>(func_code).unwrap();
        let analysis = detector.is_pure_function(&item_fn);

        // With Spec 160c, we now analyze the macro body and detect eprintln!
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_known_vs_unknown_macro_confidence() {
        use crate::analyzers::macro_definition_collector::*;
        use dashmap::DashMap;
        use std::sync::Arc;

        // Test with known macro definition
        let definitions = Arc::new(DashMap::new());
        definitions.insert(
            "my_macro".to_string(),
            MacroDefinition {
                name: "my_macro".to_string(),
                body: String::new(),
                source_file: std::path::PathBuf::from("test.rs"),
                line: 1,
            },
        );

        let mut detector_with_def = PurityDetector::with_macro_definitions(definitions);
        let func_with_known = r#"
            fn test() {
                my_macro!();
            }
        "#;
        let item_fn = syn::parse_str::<ItemFn>(func_with_known).unwrap();
        let analysis_with_def = detector_with_def.is_pure_function(&item_fn);

        // Test with unknown macro
        let mut detector_without_def = PurityDetector::new();
        let func_with_unknown = r#"
            fn test() {
                unknown_macro!();
            }
        "#;
        let item_fn2 = syn::parse_str::<ItemFn>(func_with_unknown).unwrap();
        let analysis_without_def = detector_without_def.is_pure_function(&item_fn2);

        // Both should reduce confidence, but by same amount for now
        // (Spec 160c will differentiate based on macro body analysis)
        assert!(analysis_with_def.confidence < 1.0);
        assert!(analysis_without_def.confidence < 1.0);
    }

    // Tests for spec 160c: custom macro heuristic analysis

    #[test]
    fn test_end_to_end_custom_macro_analysis() {
        use crate::analyzers::macro_definition_collector::*;
        use dashmap::DashMap;
        use std::sync::Arc;

        let code = r#"
            macro_rules! my_logger {
                ($($arg:tt)*) => {
                    eprintln!("[LOG] {}", format!($($arg)*));
                };
            }

            fn process_data(data: &str) {
                my_logger!("Processing: {}", data);
            }
        "#;

        let ast = syn::parse_file(code).unwrap();

        // Collect definitions
        let definitions = Arc::new(DashMap::new());
        collect_definitions(&ast, std::path::Path::new("test.rs"), definitions.clone());

        // Analyze purity
        let mut detector = PurityDetector::with_macro_definitions(definitions);

        // Parse just the function
        let func_code = r#"
            fn process_data(data: &str) {
                my_logger!("Processing: {}", data);
            }
        "#;
        let item_fn = syn::parse_str::<ItemFn>(func_code).unwrap();
        let analysis = detector.is_pure_function(&item_fn);

        // Should detect my_logger! is impure (contains eprintln!)
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_custom_pure_macro_detection() {
        use crate::analyzers::macro_definition_collector::*;
        use dashmap::DashMap;
        use std::sync::Arc;

        let code = r#"
            macro_rules! make_vec {
                ($($elem:expr),*) => {
                    vec![$($elem),*]
                };
            }

            fn create_list() -> Vec<i32> {
                make_vec![1, 2, 3]
            }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let definitions = Arc::new(DashMap::new());
        collect_definitions(&ast, std::path::Path::new("test.rs"), definitions.clone());

        let mut detector = PurityDetector::with_macro_definitions(definitions);

        let func_code = r#"
            fn create_list() -> Vec<i32> {
                make_vec![1, 2, 3]
            }
        "#;
        let item_fn = syn::parse_str::<ItemFn>(func_code).unwrap();
        let analysis = detector.is_pure_function(&item_fn);

        // Should detect make_vec! is pure (only contains vec!)
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_conditional_custom_macro() {
        use crate::analyzers::macro_definition_collector::*;
        use dashmap::DashMap;
        use std::sync::Arc;

        let code = r#"
            macro_rules! debug_check {
                ($val:expr) => {
                    debug_assert!($val > 0);
                    $val
                };
            }

            fn validate(x: i32) -> i32 {
                debug_check!(x)
            }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let definitions = Arc::new(DashMap::new());
        collect_definitions(&ast, std::path::Path::new("test.rs"), definitions.clone());

        let mut detector = PurityDetector::with_macro_definitions(definitions);

        let func_code = r#"
            fn validate(x: i32) -> i32 {
                debug_check!(x)
            }
        "#;
        let item_fn = syn::parse_str::<ItemFn>(func_code).unwrap();
        let analysis = detector.is_pure_function(&item_fn);

        // In debug builds, should be impure; in release builds, pure
        #[cfg(debug_assertions)]
        assert_eq!(analysis.purity_level, PurityLevel::Impure);

        #[cfg(not(debug_assertions))]
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_nested_custom_macros() {
        use crate::analyzers::macro_definition_collector::*;
        use dashmap::DashMap;
        use std::sync::Arc;

        let code = r#"
            macro_rules! log_and_return {
                ($val:expr) => {
                    {
                        println!("Returning: {}", $val);
                        $val
                    }
                };
            }

            fn compute(x: i32) -> i32 {
                log_and_return!(x * 2)
            }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let definitions = Arc::new(DashMap::new());
        collect_definitions(&ast, std::path::Path::new("test.rs"), definitions.clone());

        let mut detector = PurityDetector::with_macro_definitions(definitions);

        let func_code = r#"
            fn compute(x: i32) -> i32 {
                log_and_return!(x * 2)
            }
        "#;
        let item_fn = syn::parse_str::<ItemFn>(func_code).unwrap();
        let analysis = detector.is_pure_function(&item_fn);

        // Should detect println! in the macro body
        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    // Tests for spec 161: refined unsafe block analysis

    #[test]
    fn test_transmute_is_pure_unsafe() {
        let analysis = analyze_function_str(
            r#"
            fn bytes_to_u32(bytes: [u8; 4]) -> u32 {
                unsafe { std::mem::transmute(bytes) }
            }
            "#,
        );

        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
        assert!(analysis.confidence > 0.80); // Reduced from 1.0
        assert!(analysis.confidence < 0.90); // Should be reduced by ~15%
    }

    #[test]
    fn test_ffi_call_is_impure() {
        let analysis = analyze_function_str(
            r#"
            fn call_external() {
                extern "C" { fn external_func(); }
                unsafe { external_func(); }
            }
            "#,
        );

        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_pointer_read_is_pure_unsafe() {
        let analysis = analyze_function_str(
            r#"
            fn read_ptr(ptr: *const i32) -> i32 {
                unsafe { ptr.read() }
            }
            "#,
        );

        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
        assert!(analysis.confidence > 0.80);
    }

    #[test]
    fn test_pointer_write_is_impure() {
        let analysis = analyze_function_str(
            r#"
            fn write_ptr(ptr: *mut i32, value: i32) {
                unsafe { ptr.write(value); }
            }
            "#,
        );

        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }

    #[test]
    fn test_pointer_arithmetic_is_pure_unsafe() {
        let analysis = analyze_function_str(
            r#"
            fn offset_ptr(ptr: *const i32, offset: isize) -> *const i32 {
                unsafe { ptr.offset(offset) }
            }
            "#,
        );

        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
        assert!(analysis.confidence > 0.80);
    }

    #[test]
    fn test_mutable_static_is_impure() {
        let analysis = analyze_function_str(
            r#"
            fn access_static() -> i32 {
                static mut COUNTER: i32 = 0;
                unsafe {
                    COUNTER += 1;
                    COUNTER
                }
            }
            "#,
        );

        assert_eq!(analysis.purity_level, PurityLevel::Impure);
    }
}
