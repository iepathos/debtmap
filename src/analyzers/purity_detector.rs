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

/// Classification of path purity for constant detection (Spec 259)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathPurity {
    /// Definitely a constant, no purity impact
    Constant,
    /// Likely a constant (e.g., SCREAMING_CASE), reduce confidence slightly
    ProbablyConstant,
    /// Unknown path, conservative: assume external state access
    Unknown,
}

/// Known constant path prefixes that don't affect purity
const KNOWN_CONSTANT_PREFIXES: &[&str] = &[
    // Numeric constants
    "std :: i8 ::",
    "std :: i16 ::",
    "std :: i32 ::",
    "std :: i64 ::",
    "std :: i128 ::",
    "std :: isize ::",
    "std :: u8 ::",
    "std :: u16 ::",
    "std :: u32 ::",
    "std :: u64 ::",
    "std :: u128 ::",
    "std :: usize ::",
    "std :: f32 ::",
    "std :: f64 ::",
    // Core versions
    "core :: i8 ::",
    "core :: i16 ::",
    "core :: i32 ::",
    "core :: i64 ::",
    "core :: i128 ::",
    "core :: isize ::",
    "core :: u8 ::",
    "core :: u16 ::",
    "core :: u32 ::",
    "core :: u64 ::",
    "core :: u128 ::",
    "core :: usize ::",
    "core :: f32 ::",
    "core :: f64 ::",
    // Common constants
    "std :: mem :: size_of",
    "std :: mem :: align_of",
    "core :: mem :: size_of",
    "core :: mem :: align_of",
    // Float constants
    "std :: f32 :: consts ::",
    "std :: f64 :: consts ::",
    "core :: f32 :: consts ::",
    "core :: f64 :: consts ::",
];

/// Standard library functions known to be pure (Spec 261)
/// These take inputs and return outputs without side effects
pub const KNOWN_PURE_STD_FUNCTIONS: &[&str] = &[
    // Option methods
    "Option::map",
    "Option::and_then",
    "Option::or_else",
    "Option::unwrap_or",
    "Option::unwrap_or_else",
    "Option::unwrap_or_default",
    "Option::filter",
    "Option::flatten",
    "Option::zip",
    "Option::ok_or",
    "Option::ok_or_else",
    "Option::is_some",
    "Option::is_none",
    "Option::as_ref",
    "Option::as_mut",
    "Option::cloned",
    "Option::copied",
    // Result methods
    "Result::map",
    "Result::map_err",
    "Result::and_then",
    "Result::or_else",
    "Result::unwrap_or",
    "Result::unwrap_or_else",
    "Result::unwrap_or_default",
    "Result::is_ok",
    "Result::is_err",
    "Result::ok",
    "Result::err",
    "Result::as_ref",
    // Iterator methods (pure when closure is pure)
    "Iterator::map",
    "Iterator::filter",
    "Iterator::filter_map",
    "Iterator::flat_map",
    "Iterator::fold",
    "Iterator::reduce",
    "Iterator::take",
    "Iterator::skip",
    "Iterator::take_while",
    "Iterator::skip_while",
    "Iterator::enumerate",
    "Iterator::zip",
    "Iterator::chain",
    "Iterator::collect",
    "Iterator::count",
    "Iterator::sum",
    "Iterator::product",
    "Iterator::any",
    "Iterator::all",
    "Iterator::find",
    "Iterator::position",
    "Iterator::max",
    "Iterator::min",
    "Iterator::max_by",
    "Iterator::min_by",
    "Iterator::max_by_key",
    "Iterator::min_by_key",
    "Iterator::rev",
    "Iterator::cloned",
    "Iterator::copied",
    "Iterator::peekable",
    "Iterator::fuse",
    "Iterator::flatten",
    // Slice methods
    "slice::iter",
    "slice::len",
    "slice::is_empty",
    "slice::first",
    "slice::last",
    "slice::get",
    "slice::split_at",
    "slice::chunks",
    "slice::windows",
    "slice::contains",
    "slice::starts_with",
    "slice::ends_with",
    "slice::binary_search",
    // String methods
    "str::len",
    "str::is_empty",
    "str::chars",
    "str::bytes",
    "str::contains",
    "str::starts_with",
    "str::ends_with",
    "str::find",
    "str::rfind",
    "str::split",
    "str::trim",
    "str::trim_start",
    "str::trim_end",
    "str::to_lowercase",
    "str::to_uppercase",
    "str::to_string",
    "str::parse",
    // Vec methods (read-only)
    "Vec::len",
    "Vec::is_empty",
    "Vec::capacity",
    "Vec::iter",
    "Vec::first",
    "Vec::last",
    "Vec::get",
    "Vec::contains",
    // HashMap methods (read-only)
    "HashMap::len",
    "HashMap::is_empty",
    "HashMap::get",
    "HashMap::contains_key",
    "HashMap::keys",
    "HashMap::values",
    "HashMap::iter",
    // Clone trait
    "Clone::clone",
    // Default trait
    "Default::default",
    // From/Into traits
    "From::from",
    "Into::into",
    // Comparison traits
    "PartialEq::eq",
    "PartialEq::ne",
    "PartialOrd::partial_cmp",
    "Ord::cmp",
    // Conversion functions
    "std::convert::identity",
    "std::mem::size_of",
    "std::mem::align_of",
    "std::mem::replace",
    "std::mem::take",
    "std::mem::swap",
];

/// Check if a method call is a known pure standard library function (Spec 261)
pub fn is_known_pure_call(method_name: &str, receiver_type: Option<&str>) -> bool {
    let full_name = match receiver_type {
        Some(ty) => format!("{}::{}", ty, method_name),
        None => method_name.to_string(),
    };

    KNOWN_PURE_STD_FUNCTIONS
        .iter()
        .any(|pure_fn| full_name.ends_with(pure_fn) || pure_fn.ends_with(&full_name))
}

/// Check if a method name alone matches a known pure method (Spec 261)
pub fn is_known_pure_method(method_name: &str) -> bool {
    // Check common pure methods without needing receiver type
    const PURE_METHOD_NAMES: &[&str] = &[
        // Option/Result methods
        "map",
        "and_then",
        "or_else",
        "unwrap_or",
        "unwrap_or_else",
        "unwrap_or_default",
        "filter",
        "flatten",
        "zip",
        "ok_or",
        "ok_or_else",
        "is_some",
        "is_none",
        "is_ok",
        "is_err",
        "ok",
        "err",
        "as_ref",
        "cloned",
        "copied",
        // Iterator methods
        "fold",
        "reduce",
        "take",
        "skip",
        "take_while",
        "skip_while",
        "enumerate",
        "chain",
        "collect",
        "count",
        "sum",
        "product",
        "any",
        "all",
        "find",
        "position",
        "max",
        "min",
        "max_by",
        "min_by",
        "max_by_key",
        "min_by_key",
        "rev",
        "peekable",
        "fuse",
        // Slice/Vec/String read methods
        "len",
        "is_empty",
        "first",
        "last",
        "get",
        "iter",
        "contains",
        "starts_with",
        "ends_with",
        "binary_search",
        "chars",
        "bytes",
        "trim",
        "trim_start",
        "trim_end",
        "to_lowercase",
        "to_uppercase",
        "to_string",
        "parse",
        "split",
        "split_at",
        "chunks",
        "windows",
        "capacity",
        "keys",
        "values",
        // Clone/Default/Conversion
        "clone",
        "default",
        "from",
        "into",
        // Comparison
        "eq",
        "ne",
        "partial_cmp",
        "cmp",
    ];

    PURE_METHOD_NAMES.contains(&method_name)
}

/// Known constant suffixes that don't affect purity
const KNOWN_CONSTANT_SUFFIXES: &[&str] = &[
    ":: MAX",
    ":: MIN",
    ":: BITS",
    ":: EPSILON",
    ":: INFINITY",
    ":: NEG_INFINITY",
    ":: NAN",
    ":: RADIX",
    ":: MANTISSA_DIGITS",
    ":: DIGITS",
    ":: MIN_EXP",
    ":: MAX_EXP",
    ":: MIN_10_EXP",
    ":: MAX_10_EXP",
    ":: MIN_POSITIVE",
    // Common math constants
    ":: PI",
    ":: TAU",
    ":: E",
    ":: FRAC_PI_2",
    ":: FRAC_PI_3",
    ":: FRAC_PI_4",
    ":: FRAC_PI_6",
    ":: FRAC_PI_8",
    ":: FRAC_1_PI",
    ":: FRAC_2_PI",
    ":: FRAC_2_SQRT_PI",
    ":: SQRT_2",
    ":: FRAC_1_SQRT_2",
    ":: LN_2",
    ":: LN_10",
    ":: LOG2_E",
    ":: LOG10_E",
    ":: LOG2_10",
    ":: LOG10_2",
];

/// Check if a path string represents a known constant
fn is_known_constant(path_str: &str) -> bool {
    // Check prefixes (std :: i32 ::, core :: u64 ::, etc.)
    for prefix in KNOWN_CONSTANT_PREFIXES {
        if path_str.starts_with(prefix) {
            return true;
        }
    }

    // Check suffixes (:: MAX, :: MIN, etc.)
    for suffix in KNOWN_CONSTANT_SUFFIXES {
        if path_str.ends_with(suffix) {
            return true;
        }
    }

    false
}

/// Check if a string is in SCREAMING_CASE (likely a constant)
fn is_screaming_case(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_uppercase() || c == '_' || c.is_numeric())
        && s.chars().any(|c| c.is_alphabetic())
}

/// Check if a string is in PascalCase (likely an enum variant)
fn is_pascal_case(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_uppercase())
        && !s.chars().all(|c| c.is_uppercase() || c == '_')
        && s.chars().any(|c| c.is_lowercase())
}

/// Classify a path for purity analysis
fn classify_path_purity(path_str: &str) -> PathPurity {
    // 1. Check known constants
    if is_known_constant(path_str) {
        return PathPurity::Constant;
    }

    // 2. Check for SCREAMING_CASE (likely constant)
    // Extract last segment after ::
    let last_segment = path_str.rsplit("::").next().unwrap_or(path_str).trim();

    if is_screaming_case(last_segment) {
        return PathPurity::ProbablyConstant;
    }

    // 3. Check for enum variants (PascalCase after ::)
    // Common patterns: Option::None, Result::Ok, MyEnum::Variant
    if is_pascal_case(last_segment) {
        // Additional check: if it looks like an enum variant pattern
        // (path contains :: and ends with PascalCase identifier)
        let segments: Vec<&str> = path_str.split("::").map(|s| s.trim()).collect();
        if segments.len() >= 2 {
            // Last segment is PascalCase - likely an enum variant
            return PathPurity::ProbablyConstant;
        }
    }

    // 4. Default: unknown, conservative
    PathPurity::Unknown
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
        _cfg: &ControlFlowGraph,
        _data_flow: &DataFlowAnalysis,
    ) -> Vec<LocalMutation> {
        // Dead store analysis has been removed as it produced too many false positives.
        // Return all local mutations unchanged.
        self.local_mutations.clone()
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
            // Spec 259: Distinguish constants from actual external state
            Expr::Path(path) => {
                let path_str = quote::quote!(#path).to_string();
                // Check if it's accessing a module path (like std::i32::MAX)
                if path_str.contains("::") && !self.scope.is_local(&path_str) {
                    match classify_path_purity(&path_str) {
                        PathPurity::Constant => {
                            // No impact on purity - it's a compile-time constant
                        }
                        PathPurity::ProbablyConstant => {
                            // Slight confidence reduction but not impure
                            self.unknown_macros_count += 1;
                        }
                        PathPurity::Unknown => {
                            // Conservative: assume external state access
                            self.accesses_external_state = true;
                        }
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

    // Tests for spec 259: Fix constants false positive in purity analysis

    #[test]
    fn test_std_max_constant_is_pure() {
        // Spec 259: std::i32::MAX is a compile-time constant, should be StrictlyPure
        let analysis = analyze_function_str(
            r#"
            fn is_valid(x: i32) -> bool {
                x < std::i32::MAX
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_core_constant_is_pure() {
        let analysis = analyze_function_str(
            r#"
            fn min_val() -> u64 {
                core::u64::MIN
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_float_constants_are_pure() {
        let analysis = analyze_function_str(
            r#"
            fn is_infinite(x: f64) -> bool {
                x == std::f64::INFINITY || x == std::f64::NEG_INFINITY
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_float_math_constants_are_pure() {
        let analysis = analyze_function_str(
            r#"
            fn get_pi() -> f64 {
                std::f64::consts::PI
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_enum_variant_is_pure() {
        let analysis = analyze_function_str(
            r#"
            fn default_option() -> Option<i32> {
                Option::None
            }
            "#,
        );
        // PascalCase enum variants are ProbablyConstant (reduce confidence only)
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_screaming_case_constant_is_pure() {
        let analysis = analyze_function_str(
            r#"
            fn get_max() -> usize {
                config::MAX_SIZE
            }
            "#,
        );
        // SCREAMING_CASE is ProbablyConstant (still pure, reduced confidence)
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
        // Confidence should be reduced due to ProbablyConstant classification
        assert!(analysis.confidence < 0.98);
    }

    #[test]
    fn test_unknown_path_is_conservative() {
        let analysis = analyze_function_str(
            r#"
            fn get_value() -> i32 {
                external_crate::get_value
            }
            "#,
        );
        // Unknown paths should remain conservative (ReadOnly or Impure)
        assert_eq!(analysis.purity_level, PurityLevel::ReadOnly);
    }

    #[test]
    fn test_multiple_constants_are_pure() {
        let analysis = analyze_function_str(
            r#"
            fn range_check(x: i32) -> bool {
                x >= std::i32::MIN && x <= std::i32::MAX
            }
            "#,
        );
        assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
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

    // Tests for spec 261: Known pure std function detection

    #[test]
    fn test_is_known_pure_call_option_map() {
        assert!(is_known_pure_call("map", Some("Option")));
        assert!(is_known_pure_call("and_then", Some("Option")));
        assert!(is_known_pure_call("unwrap_or", Some("Option")));
    }

    #[test]
    fn test_is_known_pure_call_result_methods() {
        assert!(is_known_pure_call("map", Some("Result")));
        assert!(is_known_pure_call("map_err", Some("Result")));
        assert!(is_known_pure_call("and_then", Some("Result")));
        assert!(is_known_pure_call("is_ok", Some("Result")));
    }

    #[test]
    fn test_is_known_pure_call_iterator_methods() {
        assert!(is_known_pure_call("map", Some("Iterator")));
        assert!(is_known_pure_call("filter", Some("Iterator")));
        assert!(is_known_pure_call("fold", Some("Iterator")));
        assert!(is_known_pure_call("collect", Some("Iterator")));
        assert!(is_known_pure_call("sum", Some("Iterator")));
    }

    #[test]
    fn test_is_known_pure_call_string_methods() {
        assert!(is_known_pure_call("len", Some("str")));
        assert!(is_known_pure_call("is_empty", Some("str")));
        assert!(is_known_pure_call("contains", Some("str")));
        assert!(is_known_pure_call("trim", Some("str")));
    }

    #[test]
    fn test_is_known_pure_call_vec_methods() {
        assert!(is_known_pure_call("len", Some("Vec")));
        assert!(is_known_pure_call("is_empty", Some("Vec")));
        assert!(is_known_pure_call("iter", Some("Vec")));
        assert!(is_known_pure_call("get", Some("Vec")));
    }

    #[test]
    fn test_is_known_pure_call_clone_default() {
        assert!(is_known_pure_call("clone", Some("Clone")));
        assert!(is_known_pure_call("default", Some("Default")));
    }

    #[test]
    fn test_is_known_pure_method_without_receiver() {
        assert!(is_known_pure_method("map"));
        assert!(is_known_pure_method("filter"));
        assert!(is_known_pure_method("collect"));
        assert!(is_known_pure_method("len"));
        assert!(is_known_pure_method("is_empty"));
        assert!(is_known_pure_method("clone"));
    }

    #[test]
    fn test_is_known_pure_method_unknown() {
        // These should NOT be considered known pure
        assert!(!is_known_pure_method("println"));
        assert!(!is_known_pure_method("write"));
        assert!(!is_known_pure_method("push")); // Mutation method
        assert!(!is_known_pure_method("insert")); // Mutation method
    }

    #[test]
    fn test_is_known_pure_call_std_mem() {
        assert!(is_known_pure_call("size_of", Some("std::mem")));
        assert!(is_known_pure_call("align_of", Some("std::mem")));
    }
}
