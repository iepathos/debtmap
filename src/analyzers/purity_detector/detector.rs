//! Main purity detector implementation
//!
//! The PurityDetector analyzes Rust functions to determine their purity level
//! through static analysis.

use dashmap::DashMap;
use std::sync::Arc;
use syn::{visit::Visit, Block, Expr, ExprClosure, ExprMethodCall, ItemFn, Local, Pat, Stmt};

use super::confidence::{calculate_confidence_score, ConfidenceParams};
use super::constants::ITERATOR_METHODS;
use super::io_detection::{is_io_call, is_mutation_method};
use super::macro_handling::{apply_purity, classify_builtin, extract_macro_name};
use super::mutation_scope::determine_mutation_scope;
use super::path_classification::classify_path_purity;
use super::types::{
    ImpurityReason, LocalMutation, MutationScope, PathPurity, PurityAnalysis, UpvalueMutation,
};
use super::unsafe_analysis::analyze_unsafe_block;
use crate::analysis::data_flow::{ControlFlowGraph, DataFlowAnalysis};
use crate::analyzers::closure_analyzer::{ClosureAnalyzer, ClosurePurity};
use crate::analyzers::custom_macro_analyzer::CustomMacroAnalyzer;
use crate::analyzers::macro_definition_collector::MacroDefinitions;
use crate::analyzers::scope_tracker::ScopeTracker;
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

    // Track unknown macros for confidence adjustment
    unknown_macros_count: usize,

    // Macro definitions for custom macro classification
    macro_definitions: MacroDefinitions,

    // Custom macro analyzer (Spec 160c)
    macro_analyzer: CustomMacroAnalyzer,

    // Track pure unsafe operations (Spec 161)
    has_pure_unsafe: bool,
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

    // =========================================================================
    // Shared helper methods to eliminate duplication (Spec refactoring)
    // =========================================================================

    /// Reset all detector state for a new analysis
    fn reset_state(&mut self) {
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
    }

    /// Analyze parameters and populate scope
    fn analyze_parameters<'a, I>(&mut self, inputs: I)
    where
        I: Iterator<Item = &'a syn::FnArg>,
    {
        for arg in inputs {
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
    }

    /// Build analysis result from current state
    fn build_analysis_result(&self, block: &Block, has_params: bool) -> PurityAnalysis {
        // Perform data flow analysis to refine mutation detection
        let cfg = ControlFlowGraph::from_block(block);
        let data_flow = DataFlowAnalysis::analyze(&cfg);

        // Capture var_names from CFG for VarId translation
        let var_names = cfg.var_names.clone();

        // Filter dead mutations from local_mutations
        let live_mutations = self.filter_dead_mutations(&cfg, &data_flow);

        // Determine purity level based on mutations and side effects
        let purity_level = self.determine_purity_level_internal(has_params);

        // Calculate confidence
        let mut confidence = self.calculate_confidence();
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

    fn determine_purity_level_internal(&self, has_params: bool) -> PurityLevel {
        if !self.has_side_effects
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
        } else if has_params || !self.local_mutations.is_empty() {
            // Only has local mutations, no external state access or modification
            PurityLevel::LocallyPure
        } else {
            PurityLevel::LocallyPure
        }
    }

    // =========================================================================
    // Public analysis methods
    // =========================================================================

    /// Analyzes a function to determine if it's pure
    pub fn is_pure_function(&mut self, item_fn: &ItemFn) -> PurityAnalysis {
        self.reset_state();
        self.analyze_parameters(item_fn.sig.inputs.iter());
        self.visit_block(&item_fn.block);
        self.build_analysis_result(&item_fn.block, !item_fn.sig.inputs.is_empty())
    }

    /// Analyzes an impl method to determine if it's pure (spec 202)
    pub fn is_pure_impl_method(&mut self, impl_fn: &syn::ImplItemFn) -> PurityAnalysis {
        self.reset_state();
        self.analyze_parameters(impl_fn.sig.inputs.iter());
        self.visit_block(&impl_fn.block);
        self.build_analysis_result(&impl_fn.block, !impl_fn.sig.inputs.is_empty())
    }

    /// Analyzes a block to determine if it's pure
    pub fn is_pure_block(&mut self, block: &Block) -> PurityAnalysis {
        self.reset_state();
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
            PurityLevel::Impure
        } else if self.accesses_external_state {
            PurityLevel::ReadOnly
        } else {
            PurityLevel::LocallyPure
        };

        PurityAnalysis {
            is_pure: purity_level == PurityLevel::StrictlyPure,
            purity_level,
            reasons: self.collect_impurity_reasons(),
            confidence: self.calculate_confidence(),
            data_flow_info: Some(data_flow),
            live_mutations: self.local_mutations.clone(),
            total_mutations: self.local_mutations.len(),
            var_names,
        }
    }

    // =========================================================================
    // Helper methods
    // =========================================================================

    fn filter_dead_mutations(
        &self,
        _cfg: &ControlFlowGraph,
        _data_flow: &DataFlowAnalysis,
    ) -> Vec<LocalMutation> {
        // Dead store analysis has been removed as it produced too many false positives.
        // Return all local mutations unchanged.
        self.local_mutations.clone()
    }

    fn has_mutable_reference(&self, pat: &Pat) -> bool {
        match pat {
            Pat::Type(pat_type) => self.type_has_mutable_reference(&pat_type.ty),
            Pat::Ident(pat_ident) => pat_ident.mutability.is_some(),
            _ => false,
        }
    }

    fn type_has_mutable_reference(&self, ty: &syn::Type) -> bool {
        match ty {
            syn::Type::Reference(type_ref) => type_ref.mutability.is_some(),
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
        let params = ConfidenceParams {
            has_side_effects: self.has_side_effects,
            has_io_operations: self.has_io_operations,
            has_unsafe_blocks: self.has_unsafe_blocks,
            modifies_external_state: self.modifies_external_state,
            accesses_external_state: self.accesses_external_state,
            local_mutations: &self.local_mutations,
            upvalue_mutations: &self.upvalue_mutations,
            unknown_macros_count: self.unknown_macros_count,
            has_pure_unsafe: self.has_pure_unsafe,
        };
        calculate_confidence_score(&params)
    }

    #[allow(dead_code)]
    fn determine_purity_level(&self) -> PurityLevel {
        if self.modifies_external_state
            || self.has_io_operations
            || self.has_unsafe_blocks
            || self.has_side_effects
        {
            return PurityLevel::Impure;
        }

        if !self.local_mutations.is_empty() || !self.upvalue_mutations.is_empty() {
            return PurityLevel::LocallyPure;
        }

        if self.accesses_external_state {
            return PurityLevel::ReadOnly;
        }

        PurityLevel::StrictlyPure
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

    // =========================================================================
    // Internal analysis methods
    // =========================================================================

    fn visit_expr_closure_internal(&mut self, closure: &ExprClosure) {
        let mut analyzer = ClosureAnalyzer::new(&self.scope);
        let closure_purity = analyzer.analyze_closure(closure);

        self.closure_results.push(closure_purity.clone());

        match closure_purity.level {
            PurityLevel::Impure => {
                self.modifies_external_state = true;
                self.has_side_effects = true;
            }
            PurityLevel::LocallyPure => {
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

        if ITERATOR_METHODS.contains(&method_name.as_str()) {
            for arg in &method.args {
                if let Expr::Closure(closure) = arg {
                    let mut analyzer = ClosureAnalyzer::new(&self.scope);
                    let purity = analyzer.analyze_closure(closure);

                    self.closure_results.push(purity.clone());

                    if purity.level == PurityLevel::Impure {
                        self.has_side_effects = true;
                        self.modifies_external_state = true;
                    }
                }
            }
        }

        if is_mutation_method(&method_name) {
            let receiver = &method.receiver;
            let scope = determine_mutation_scope(receiver, &self.scope);
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

        if method_name.contains("write")
            || method_name.contains("print")
            || method_name.contains("flush")
        {
            self.has_io_operations = true;
            self.has_side_effects = true;
        }

        syn::visit::visit_expr_method_call(self, method);
    }

    fn handle_macro(&mut self, mac: &syn::Macro) {
        let name = extract_macro_name(&mac.path);

        // Step 1: Check built-in macros first (from Spec 160a)
        if let Some(purity) = classify_builtin(&name) {
            let effects = apply_purity(purity);
            self.has_side_effects |= effects.has_side_effects;
            self.has_io_operations |= effects.has_io_operations;
            if effects.unknown_macro_detected {
                self.unknown_macros_count += 1;
            }
            return;
        }

        // Step 2: Check custom macros (from Spec 160b + 160c)
        let custom_macro_body = self
            .macro_definitions
            .get(&name)
            .map(|def| def.body.clone());
        if let Some(body) = custom_macro_body {
            let purity = self.macro_analyzer.analyze(&body);
            let effects = apply_purity(purity);
            self.has_side_effects |= effects.has_side_effects;
            self.has_io_operations |= effects.has_io_operations;
            if effects.unknown_macro_detected {
                self.unknown_macros_count += 1;
            }
            return;
        }

        // Step 3: Truly unknown macro
        self.unknown_macros_count += 1;
    }
}

impl<'ast> Visit<'ast> for PurityDetector {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::Closure(closure) => {
                self.visit_expr_closure_internal(closure);
                return;
            }
            Expr::Macro(expr_macro) => {
                self.handle_macro(&expr_macro.mac);
            }
            Expr::Unsafe(unsafe_expr) => {
                let result = analyze_unsafe_block(&unsafe_expr.block);
                if result.has_impure {
                    self.has_unsafe_blocks = true;
                    self.modifies_external_state = true;
                    self.has_side_effects = true;
                } else if result.has_pure_unsafe {
                    self.has_pure_unsafe = true;
                }
                return;
            }
            Expr::Assign(assign) => {
                let scope = determine_mutation_scope(&assign.left, &self.scope);
                match scope {
                    MutationScope::Local => match &*assign.left {
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
                    },
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
            Expr::Call(call) => {
                if let Expr::Path(expr_path) = &*call.func {
                    let path_str = quote::quote!(#expr_path).to_string();
                    if is_io_call(&path_str) {
                        self.has_io_operations = true;
                        self.has_side_effects = true;
                    }
                }
                for arg in &call.args {
                    self.visit_expr(arg);
                }
                return;
            }
            Expr::Path(path) => {
                let path_str = quote::quote!(#path).to_string();
                if path_str.contains("::") && !self.scope.is_local(&path_str) {
                    match classify_path_purity(&path_str) {
                        PathPurity::Constant => {}
                        PathPurity::ProbablyConstant => {
                            self.unknown_macros_count += 1;
                        }
                        PathPurity::Unknown => {
                            self.accesses_external_state = true;
                        }
                    }
                }
            }
            Expr::MethodCall(method_call) => {
                self.visit_expr_method_call_with_closures(method_call);
                return;
            }
            _ => {}
        }

        syn::visit::visit_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        if let Stmt::Local(local) = stmt {
            self.visit_local(local);
        }

        if let Stmt::Macro(stmt_macro) = stmt {
            self.handle_macro(&stmt_macro.mac);
        }

        syn::visit::visit_stmt(self, stmt);
    }

    fn visit_local(&mut self, local: &'ast Local) {
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
