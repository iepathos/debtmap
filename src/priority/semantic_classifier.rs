use crate::analyzers::rust_constructor_detector::{
    analyze_function_body, extract_return_type, ConstructorReturnType,
};
use crate::analyzers::rust_data_flow_analyzer::analyze_data_flow;
use crate::analyzers::rust_enum_converter_detector::is_enum_converter;
use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionRole {
    PureLogic,    // Business logic, high test priority
    Orchestrator, // Coordinates other functions
    IOWrapper,    // Thin I/O layer
    EntryPoint,   // Main entry points
    PatternMatch, // Pattern matching function (low complexity)
    Debug,        // Debug/diagnostic functions (low test priority)
    Unknown,      // Cannot classify
}

pub fn classify_function_role(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
) -> FunctionRole {
    // Use a functional approach with classification rules
    // Note: AST is not available at this level, so we pass None
    // Full AST-based detection will be integrated when threading syn::ItemFn
    classify_by_rules(func, func_id, call_graph, None).unwrap_or(FunctionRole::PureLogic)
}

// Pure function that applies classification rules in order
fn classify_by_rules(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
    syn_func: Option<&syn::ItemFn>,
) -> Option<FunctionRole> {
    // Entry point has highest precedence
    if is_entry_point(func_id, call_graph) {
        return Some(FunctionRole::EntryPoint);
    }

    // Check for debug/diagnostic functions early (Spec 119)
    if is_debug_function(func) {
        return Some(FunctionRole::Debug);
    }

    // Check for constructors BEFORE pattern matching (Spec 117 + 122)
    if is_constructor_enhanced(func, syn_func) {
        return Some(FunctionRole::IOWrapper);
    }

    // Check for enum converters (Spec 124)
    if let Some(syn_func) = syn_func {
        if is_enum_converter_enhanced(func, syn_func) {
            return Some(FunctionRole::IOWrapper);
        }
    }

    // Check for accessor methods (Spec 125)
    // This should come after enum converter detection but before pattern matching
    if is_accessor_method(func, syn_func) {
        return Some(FunctionRole::IOWrapper);
    }

    // Check for data flow classification (Spec 126) - only if enabled and AST available
    if let Some(syn_func) = syn_func {
        let config = crate::config::get_data_flow_classification_config();

        if config.enabled {
            let profile = analyze_data_flow(syn_func);

            // Only classify if high confidence
            if profile.confidence >= config.min_confidence
                && profile.transformation_ratio >= config.min_transformation_ratio
                && profile.business_logic_ratio < config.max_business_logic_ratio
            {
                return Some(FunctionRole::Orchestrator);
            }
        }
    }

    // Check for pattern matching functions (like detect_file_type)
    if is_pattern_matching_function(func, func_id) {
        return Some(FunctionRole::PatternMatch);
    }

    // Check I/O wrapper BEFORE orchestration
    if is_io_wrapper(func) {
        return Some(FunctionRole::IOWrapper);
    }

    // Only then check orchestration patterns
    if is_orchestrator(func, func_id, call_graph) {
        return Some(FunctionRole::Orchestrator);
    }

    None // Will default to PureLogic
}

// Pure function to check if a function is an entry point
fn is_entry_point(func_id: &FunctionId, call_graph: &CallGraph) -> bool {
    call_graph.is_entry_point(func_id) || is_entry_point_by_name(&func_id.name)
}

/// Detect debug/diagnostic functions (Spec 119)
///
/// Debug functions are typically used for troubleshooting and have lower test priority.
/// Detection uses name patterns and behavioral characteristics.
///
/// # Detection Strategy
///
/// 1. Check if function name matches debug patterns
/// 2. Check if function has diagnostic behavioral characteristics:
///    - Primarily I/O operations (printing, logging)
///    - Simple return type (unit or simple status)
///    - Few external function calls
///    - Low complexity (avoid misclassifying complex business logic)
///
/// # Detected Patterns
///
/// **Debug functions** (classified as Debug):
/// - Name patterns: `debug_*`, `print_*`, `dump_*`, `trace_*`
/// - Name patterns: `*_diagnostics`, `*_debug`, `*_stats`
/// - Functions with primarily I/O operations and simple logic
/// - Low complexity with minimal external calls
///
/// **Complex functions** (NOT classified as Debug):
/// - High complexity (>10) even with debug-like name
/// - Many external function calls (>10)
/// - Complex business logic
fn is_debug_function(func: &FunctionMetrics) -> bool {
    let name_matches = matches_debug_pattern(&func.name);
    let has_debug_characteristics = has_diagnostic_characteristics(func);

    // If name matches debug pattern, check complexity isn't too high
    // (prevents misclassifying complex functions with debug-like names)
    if name_matches {
        // Allow functions with complexity up to 10
        return func.cognitive <= 10;
    }

    // If behavioral characteristics match, it's likely a debug function
    has_debug_characteristics
}

/// Check if function name matches debug patterns
fn matches_debug_pattern(name: &str) -> bool {
    let name_lower = name.to_lowercase();

    // Specific debug prefixes
    let prefixes = ["debug_", "print_", "dump_", "trace_"];
    // Specific debug suffixes
    let suffixes = ["_diagnostics", "_debug", "_stats"];
    // Specific debug contains patterns
    let contains = ["diagnostics"];

    prefixes.iter().any(|p| name_lower.starts_with(p))
        || suffixes.iter().any(|s| name_lower.ends_with(s))
        || contains.iter().any(|c| name_lower.contains(c))
}

/// Check if function has diagnostic behavioral characteristics
fn has_diagnostic_characteristics(func: &FunctionMetrics) -> bool {
    // Diagnostic functions typically have:
    // - Very low complexity (< 5)
    // - Short length (< 20 lines)
    // - Output-focused I/O patterns (print, display, log, not read/write/load/save)
    let is_very_simple = func.cognitive < 5 && func.length < 20;
    let has_output_io_name = matches_output_io_pattern(&func.name);

    is_very_simple && has_output_io_name
}

/// Check if name matches output-focused I/O patterns (not read/write operations)
fn matches_output_io_pattern(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    let output_patterns = ["print", "display", "show", "log", "trace", "dump"];

    output_patterns.iter().any(|p| name_lower.contains(p))
}

/// Detect simple constructor functions to prevent false positive classifications.
///
/// A function is considered a simple constructor if it meets ALL criteria:
/// - Has a constructor-like name (new, default, from_*, with_*, etc.)
/// - Low cyclomatic complexity (≤ 2)
/// - Short length (< 15 lines)
/// - Minimal nesting (≤ 1 level)
/// - Low cognitive complexity (≤ 3)
///
/// # Detected Patterns
///
/// **Simple constructors** (matches):
/// - Standard names: `new()`, `default()`, `from_*()`, `with_*()`
/// - Short length: < 15 lines
/// - Low complexity: cyclomatic ≤ 2, cognitive ≤ 3
/// - Minimal nesting: ≤ 1 level
/// - Contains basic struct initialization
///
/// **Complex factories** (does NOT match):
/// - Long functions with extensive validation logic
/// - High cyclomatic complexity from error handling
/// - Multiple levels of nesting or control flow
///
/// # False Positive Prevention
///
/// This function specifically addresses the false positive in ContextMatcher::any()
/// where a trivial 9-line constructor was classified as CRITICAL business logic.
fn is_simple_constructor(func: &FunctionMetrics) -> bool {
    // Get constructor detection configuration
    let config = crate::config::get_constructor_detection_config();

    // Name-based detection using configurable patterns
    let name_lower = func.name.to_lowercase();
    let matches_constructor_name = config.patterns.iter().any(|pattern| {
        name_lower == *pattern || name_lower.starts_with(pattern) || name_lower.ends_with(pattern)
    });

    // Complexity-based filtering using configurable thresholds
    let is_simple = func.cyclomatic <= config.max_cyclomatic
        && func.length < config.max_length
        && func.nesting <= config.max_nesting;

    // Structural pattern: low cognitive complexity suggests simple initialization
    let is_initialization = func.cognitive <= config.max_cognitive;

    matches_constructor_name && is_simple && is_initialization
}

/// Enhanced constructor detection using AST (spec 122)
///
/// This function enhances name-based detection with AST analysis when available.
/// Falls back to `is_simple_constructor()` if AST is unavailable or disabled.
///
/// # Detection Strategy
///
/// 1. Check configuration - if AST detection disabled, use name-based only
/// 2. If AST available, analyze return type and body patterns
/// 3. Return type must be `Self`, `Result<Self>`, or `Option<Self>`
/// 4. Body must show constructor patterns (struct init, Self refs)
/// 5. Complexity must be reasonable (≤5 cyclomatic, no loops)
///
/// # Detected Patterns
///
/// **AST-based detection** (when enabled and available):
/// - Functions with non-standard names like `create_default_client()`, `from_config()`
/// - Returns `Self`, `Result<Self>`, or `Option<Self>`
/// - Body contains struct initialization patterns with `Self { ... }`
/// - Simple logic without loops or complex control flow
///
/// **Name-based detection** (fallback):
/// - Standard constructor names: `new`, `default`, `from_*`, `with_*`
/// - Short functions (< 15 lines) with low complexity
/// - Minimal nesting and simple initialization patterns
fn is_constructor_enhanced(func: &FunctionMetrics, syn_func: Option<&syn::ItemFn>) -> bool {
    // Check configuration
    let config = crate::config::get_constructor_detection_config();

    // If AST detection disabled or unavailable, use name-based detection
    if !config.ast_detection || syn_func.is_none() {
        return is_simple_constructor(func);
    }

    let syn_func = syn_func.unwrap();

    // Extract AST information
    let return_type = extract_return_type(syn_func);
    let body_pattern = analyze_function_body(syn_func);

    // Check return type (must return Self)
    let returns_self = matches!(
        return_type,
        Some(
            ConstructorReturnType::OwnedSelf
                | ConstructorReturnType::ResultSelf
                | ConstructorReturnType::OptionSelf
        )
    );

    if !returns_self {
        // Fallback to name-based detection if not returning Self
        return is_simple_constructor(func);
    }

    // Check body pattern
    if !body_pattern.is_constructor_like() {
        return false;
    }

    // Check complexity thresholds (more lenient for AST-detected constructors)
    let is_simple_enough =
        func.cyclomatic <= 5 && func.nesting <= 2 && func.length < 30 && !body_pattern.has_loop;

    returns_self && is_simple_enough
}

/// Enhanced enum converter detection using AST (spec 124)
///
/// This function detects simple enum-to-string converter functions that are
/// flagged as CRITICAL but are just data accessors.
///
/// # Detection Strategy
///
/// 1. Check name matches converter patterns (name, as_str, to_*, etc.)
/// 2. Verify low cognitive complexity (≤3)
/// 3. Analyze function body for exhaustive match returning only literals
///
/// # Detected Patterns
///
/// **Simple enum converters** (classified as IOWrapper):
/// - Methods like `name()`, `as_str()`, `to_string()` on enums
/// - Body contains exhaustive `match` statement on `self`
/// - All match arms return string/numeric literals only
/// - No function calls, computations, or complex logic
/// - Very low cognitive complexity (≤ 3)
///
/// **Complex enum methods** (NOT detected, remain PureLogic):
/// - Match arms that call functions like `format!()` or constructors
/// - Methods with additional logic beyond simple mapping
/// - Methods that aggregate or compute values
fn is_enum_converter_enhanced(func: &FunctionMetrics, syn_func: &syn::ItemFn) -> bool {
    is_enum_converter(func, syn_func)
}

/// Detect simple accessor/getter methods (spec 125)
///
/// Identifies simple accessor and getter methods that should be classified as
/// IOWrapper instead of PureLogic to reduce their priority score.
///
/// # Detection Strategy
///
/// 1. Check name matches accessor patterns (id, name, get_*, is_*, etc.)
/// 2. Verify low complexity (cyclomatic ≤ 2, cognitive ≤ 1)
/// 3. Check function is short (< 10 lines, nesting ≤ 1)
/// 4. If AST available, verify body is simple accessor pattern
///
/// # Detected Patterns
///
/// **Simple accessors** (classified as IOWrapper):
/// - Direct field access: `id()`, `name()` returning field values
/// - Boolean checks: `is_active()`, `has_permission()` with simple conditions
/// - Type conversions: `as_str()`, `to_string()` with minimal logic
/// - Uses immutable `&self` reference only
/// - Very low complexity (cyclomatic ≤ 2, cognitive ≤ 1)
/// - Short length (< 10 lines, nesting ≤ 1)
///
/// **Complex methods** (NOT detected, remain PureLogic):
/// - Methods with calculations, iterations, or aggregations
/// - Methods calling other business logic functions
/// - Methods with complex control flow or multiple branches
fn is_accessor_method(func: &FunctionMetrics, syn_func: Option<&syn::ItemFn>) -> bool {
    let config = crate::config::get_accessor_detection_config();

    // Check if accessor detection is enabled
    if !config.enabled {
        return false;
    }

    // Check name matches accessor pattern
    if !matches_accessor_name(&func.name, &config) {
        return false;
    }

    // Check complexity is minimal
    if func.cyclomatic > config.max_cyclomatic
        || func.cognitive > config.max_cognitive
        || func.length >= config.max_length
        || func.nesting > config.max_nesting
    {
        return false;
    }

    // If AST available, verify body is simple
    if let Some(syn_func) = syn_func {
        if !is_simple_accessor_body(syn_func) {
            return false;
        }
    }

    true
}

/// Check if name matches accessor patterns
fn matches_accessor_name(name: &str, config: &crate::config::AccessorDetectionConfig) -> bool {
    let name_lower = name.to_lowercase();

    // Single-word accessors
    if config.single_word_patterns.contains(&name_lower) {
        return true;
    }

    // Prefix patterns
    if config
        .prefix_patterns
        .iter()
        .any(|p| name_lower.starts_with(p))
    {
        return true;
    }

    false
}

/// Check if function body is simple accessor pattern (AST analysis)
fn is_simple_accessor_body(syn_func: &syn::ItemFn) -> bool {
    // Function should take &self (not &mut self)
    if !has_immutable_self_receiver(syn_func) {
        return false;
    }

    // Single statement or expression
    let stmts = &syn_func.block.stmts;
    if stmts.is_empty() {
        return false;
    }

    // Check for simple patterns
    match stmts.len() {
        1 => {
            // Single expression: self.field, &self.field, self.field.clone()
            match &stmts[0] {
                syn::Stmt::Expr(expr, _) => is_simple_accessor_expr(expr),
                _ => false,
            }
        }
        2 => {
            // Let binding + return: let x = self.field; x
            // This is acceptable for accessors
            is_simple_binding_pattern(stmts)
        }
        _ => false, // Multiple statements - too complex
    }
}

/// Check if expression is simple accessor pattern
fn is_simple_accessor_expr(expr: &syn::Expr) -> bool {
    match expr {
        // Direct field access: self.field
        syn::Expr::Field(field_expr) => {
            matches!(&*field_expr.base, syn::Expr::Path(path)
                if path.path.is_ident("self"))
        }

        // Reference to field: &self.field
        syn::Expr::Reference(ref_expr) => is_simple_accessor_expr(&ref_expr.expr),

        // Method call on field: self.field.clone()
        syn::Expr::MethodCall(method_call) => {
            // Must be called on self.field
            is_simple_accessor_expr(&method_call.receiver)
                // Common accessor methods
                && is_simple_accessor_method(&method_call.method)
        }

        // Simple match or if (for bool accessors)
        syn::Expr::Match(_) | syn::Expr::If(_) => {
            // Already validated by complexity metrics
            // If cognitive ≤ 1, it's simple enough
            true
        }

        _ => false,
    }
}

/// Check if method is a simple accessor method
fn is_simple_accessor_method(method: &syn::Ident) -> bool {
    matches!(
        method.to_string().as_str(),
        "clone" | "to_string" | "as_ref" | "as_str" | "as_bytes" | "copied"
    )
}

/// Check if function has immutable self receiver
fn has_immutable_self_receiver(syn_func: &syn::ItemFn) -> bool {
    if let Some(syn::FnArg::Receiver(receiver)) = syn_func.sig.inputs.first() {
        receiver.mutability.is_none()
    } else {
        false
    }
}

/// Check if statements follow simple binding pattern
fn is_simple_binding_pattern(stmts: &[syn::Stmt]) -> bool {
    if stmts.len() != 2 {
        return false;
    }

    // First statement should be a let binding
    let _binding = match &stmts[0] {
        syn::Stmt::Local(_) => true,
        _ => return false,
    };

    // Second statement should be an expression (return value)
    matches!(&stmts[1], syn::Stmt::Expr(_, _))
}

// Pure function to check if a function is a pattern matching function
fn is_pattern_matching_function(func: &FunctionMetrics, func_id: &FunctionId) -> bool {
    // Check for typical pattern matching function names
    let name_lower = func_id.name.to_lowercase();
    let pattern_match_names = [
        "detect",
        "classify",
        "identify",
        "determine",
        "resolve",
        "match",
        "parse_type",
        "get_type",
        "find_type",
    ];

    // Name suggests pattern matching AND has low cyclomatic but high cognitive complexity
    // (typical of if/else chains or match statements with many branches)
    let name_matches = pattern_match_names
        .iter()
        .any(|pattern| name_lower.contains(pattern));

    // Pattern matching functions typically have:
    // - Low cyclomatic complexity (1-2, just sequential checks)
    // - Higher cognitive complexity due to many conditions
    // - Cognitive/cyclomatic ratio > 5 suggests pattern matching
    if name_matches && func.cyclomatic <= 2 {
        let ratio = if func.cyclomatic > 0 {
            func.cognitive as f32 / func.cyclomatic as f32
        } else {
            func.cognitive as f32
        };
        return ratio > 5.0;
    }

    false
}

/// Calculate delegation ratio for a function
///
/// Returns the ratio of function calls to total statements (approximated by function length).
/// A higher ratio indicates more coordination/delegation behavior.
fn calculate_delegation_ratio(func: &FunctionMetrics, meaningful_callees: &[&FunctionId]) -> f64 {
    if func.length == 0 {
        return 0.0;
    }
    meaningful_callees.len() as f64 / func.length as f64
}

// Pure function to check if a function is an orchestrator
fn is_orchestrator(func: &FunctionMetrics, func_id: &FunctionId, call_graph: &CallGraph) -> bool {
    // First check if there are meaningful callees to orchestrate
    let callees = call_graph.get_callees(func_id);
    let meaningful_callees: Vec<_> = callees
        .iter()
        .filter(|f| {
            // Filter out standard library and utility functions
            !matches!(
                f.name.as_str(),
                "format" | "write" | "print" | "println" | "clone" | "to_string" | "into" | "from"
            ) && !f.name.starts_with("std::")
                && !f.name.starts_with("core::")
                && !f.name.starts_with("alloc::")
        })
        .collect();

    // Check if this is a functional chain (all calls are functional methods)
    // Default: allow functional chains (they're idiomatic patterns)
    if !meaningful_callees.is_empty() && callees.len() > meaningful_callees.len() {
        // If all non-utility calls are removed, this might be a functional chain
        let functional_chain = callees.iter().all(|f| {
            // Check for standard library and utility functions
            matches!(
                f.name.as_str(),
                "format" | "write" | "print" | "println" | "clone" | "to_string" | "into" | "from"
            ) || f.name.starts_with("std::")
                || f.name.starts_with("core::")
                || f.name.starts_with("alloc::")
                || f.name.contains("Pipeline")
                || f.name.contains("Stream")
                || f.name.contains("Iterator")
        });
        if functional_chain {
            return false;
        }
    }

    // Check for single delegation (adapter pattern)
    // Default: exclude adapters (they're not orchestration)
    if meaningful_callees.len() == 1 {
        // This is likely an adapter/wrapper, not orchestration
        return false;
    }

    // Can't be an orchestrator without functions to orchestrate
    // Default minimum delegation count: 2
    if meaningful_callees.len() < 2 {
        return false;
    }

    // Calculate delegation ratio to better identify orchestrators
    let delegation_ratio = calculate_delegation_ratio(func, &meaningful_callees);

    // Name-based orchestration with lenient complexity threshold
    let name_suggests_orchestration =
        is_orchestrator_by_name(&func_id.name) && func.cyclomatic <= 5;

    // Lenient complexity delegation pattern with delegation ratio check
    // Orchestrators can have complexity up to 5 (allowing for error handling)
    // and should have at least 20% of their code as function calls
    let is_simple_delegation = func.cyclomatic <= 5
        && delegation_ratio >= 0.2
        && delegates_to_tested_functions(func_id, call_graph, 0.8);

    name_suggests_orchestration || is_simple_delegation
}

// Pure function to check if a function is an I/O wrapper
fn is_io_wrapper(func: &FunctionMetrics) -> bool {
    if !contains_io_patterns(func) {
        return false;
    }

    // Short I/O functions are clearly wrappers
    if func.length < 20 {
        return true;
    }

    // Longer functions can still be I/O wrappers if they match I/O orchestration patterns
    func.length <= 50 && is_io_orchestration(func)
}

fn is_entry_point_by_name(name: &str) -> bool {
    let entry_patterns = [
        "main", "run", "start", "init", "handle", "process", "execute", "serve", "listen",
    ];

    let name_lower = name.to_lowercase();
    entry_patterns
        .iter()
        .any(|pattern| name_lower.starts_with(pattern) || name_lower.ends_with(pattern))
}

fn is_orchestrator_by_name(name: &str) -> bool {
    let name_lower = name.to_lowercase();

    // Exclude common non-orchestration patterns first
    let exclude_patterns = [
        "print",
        "format",
        "create",
        "build",
        "extract",
        "parse",
        "new",
        "from",
        "to",
        "into",
        "write",
        "read",
        "display",
        "render",
        "emit",
        // Exclude adapter/wrapper patterns
        "adapt",
        "wrap",
        "convert",
        "transform",
        "translate",
        // Exclude functional patterns
        "map",
        "filter",
        "reduce",
        "fold",
        "collect",
        "apply",
        // Exclude single-purpose functions
        "get",
        "set",
        "find",
        "search",
        "check",
        "validate",
    ];

    for pattern in &exclude_patterns {
        if name_lower.starts_with(pattern) || name_lower.ends_with(pattern) {
            return false;
        }
    }

    // Check common orchestration patterns that override excludes
    // (These would have been in include_patterns config)
    let include_patterns = [
        "workflow_",
        "pipeline_",
        "process_",
        "orchestrate_",
        "coordinate_",
        "execute_flow_",
    ];
    for pattern in &include_patterns {
        if name_lower.starts_with(pattern) {
            return true;
        }
    }

    // Then check for true orchestration patterns
    let orchestrator_patterns = [
        "orchestrate",
        "coordinate",
        "manage",
        "dispatch",
        "route",
        "if_requested",
        "if_needed",
        "if_enabled",
        "maybe",
        "try_",
        "attempt_",
        "delegate",
        "forward",
    ];

    // Check for conditional patterns like generate_report_if_requested
    if name_lower.contains("_if_") || name_lower.contains("_when_") {
        return true;
    }

    orchestrator_patterns
        .iter()
        .any(|pattern| name_lower.contains(pattern))
}

fn delegates_to_tested_functions(
    func_id: &FunctionId,
    call_graph: &CallGraph,
    _threshold: f64,
) -> bool {
    let callees = call_graph.get_callees(func_id);
    if callees.is_empty() {
        return false;
    }

    // Filter out standard library functions and common utilities
    let meaningful_callees: Vec<_> = callees
        .iter()
        .filter(|f| {
            // Filter out standard library and utility functions
            !matches!(
                f.name.as_str(),
                "format" | "write" | "print" | "println" | "clone" | "to_string" | "into" | "from"
            ) && !f.name.starts_with("std::")
                && !f.name.starts_with("core::")
                && !f.name.starts_with("alloc::")
        })
        .collect();

    // Orchestrators should coordinate MULTIPLE functions (at least 2)
    // This is now consistent with the check in is_orchestrator
    meaningful_callees.len() >= 2 && call_graph.detect_delegation_pattern(func_id)
}

fn contains_io_patterns(func: &FunctionMetrics) -> bool {
    // Check for I/O related patterns in function name or content
    let io_keywords = vec![
        "read",
        "write",
        "file",
        "socket",
        "http",
        "request",
        "response",
        "stream",
        "buffer",
        "stdin",
        "stdout",
        "stderr",
        "print",
        "input",
        "output",
        "display",
        // Note: "format" removed - string formatting is not I/O
        "json",
        "serialize",
        "deserialize",
        "emit",
        "render",
        "save",
        "load",
        "export",
        "import",
        "log",
        "trace",
        "debug",
        "info",
        "warn",
        "error",
        "summary",
        "report",
    ];

    let name_lower = func.name.to_lowercase();
    io_keywords
        .iter()
        .any(|keyword| name_lower.contains(keyword))
}

fn is_io_orchestration(func: &FunctionMetrics) -> bool {
    // Function is I/O orchestration if it has I/O in the name and:
    // - Moderate cyclomatic complexity (mostly from format/output branching)
    // - Not deeply nested (nesting <= 3)
    // - Name strongly suggests I/O operations
    let name_lower = func.name.to_lowercase();

    // Strong I/O indicators in function name
    let strong_io_patterns = [
        "output_",
        "write_",
        "print_",
        "format_",
        "serialize_",
        "save_",
        "export_",
        "display_",
        "render_",
        "emit_",
    ];

    let has_strong_io_name = strong_io_patterns
        .iter()
        .any(|pattern| name_lower.starts_with(pattern));

    // I/O orchestration typically has branching for different formats/destinations
    // but not deep business logic
    has_strong_io_name && func.nesting <= 3
}

pub fn get_role_multiplier(role: FunctionRole) -> f64 {
    // Get multipliers from configuration
    let config = crate::config::get_role_multipliers();

    match role {
        FunctionRole::PureLogic => config.pure_logic,
        FunctionRole::Orchestrator => config.orchestrator,
        FunctionRole::IOWrapper => config.io_wrapper,
        FunctionRole::EntryPoint => config.entry_point,
        FunctionRole::PatternMatch => config.pattern_match,
        FunctionRole::Debug => config.debug,
        FunctionRole::Unknown => config.unknown,
    }
}

// Semantic priority calculation removed per spec 58
// Role multipliers now provide the only role-based adjustment to avoid double penalties

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FunctionMetrics;
    use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
    use std::path::PathBuf;

    fn create_test_metrics(
        name: &str,
        cyclomatic: u32,
        cognitive: u32,
        lines: usize,
    ) -> FunctionMetrics {
        FunctionMetrics {
            file: PathBuf::from("test.rs"),
            name: name.to_string(),
            line: 1,
            length: lines,
            cyclomatic,
            cognitive,
            nesting: 0,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
        }
    }

    #[test]
    fn test_entry_point_classification() {
        let graph = CallGraph::new();
        let func = create_test_metrics("main", 5, 8, 50);
        let func_id = FunctionId::new(PathBuf::from("main.rs"), "main".to_string(), 1);

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::EntryPoint);
    }

    #[test]
    fn test_orchestrator_classification() {
        let mut graph = CallGraph::new();
        let func = create_test_metrics("coordinate_tasks", 2, 3, 15);
        let func_id = FunctionId::new(
            PathBuf::from("coord.rs"),
            "coordinate_tasks".to_string(),
            10,
        );

        // Add the orchestrator function
        graph.add_function(func_id.clone(), false, false, 2, 15);

        // Add some worker functions it calls
        for i in 0..3 {
            let worker_id =
                FunctionId::new(PathBuf::from("worker.rs"), format!("worker_{i}"), i * 10);
            graph.add_function(worker_id.clone(), false, false, 8, 40);
            graph.add_call(crate::priority::call_graph::FunctionCall {
                caller: func_id.clone(),
                callee: worker_id,
                call_type: crate::priority::call_graph::CallType::Delegate,
            });
        }

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::Orchestrator);
    }

    #[test]
    fn test_io_wrapper_classification() {
        let graph = CallGraph::new();
        let func = create_test_metrics("read_file", 1, 2, 10);
        let func_id = FunctionId::new(PathBuf::from("io.rs"), "read_file".to_string(), 5);

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::IOWrapper);
    }

    #[test]
    fn test_io_orchestration_classification() {
        let graph = CallGraph::new();

        // Test case similar to output_unified_priorities:
        // - Has "output_" prefix (strong I/O pattern)
        // - 38 lines (within the 50 line limit)
        // - Cyclomatic 12 (from format branching)
        // - Nesting 3 (not deeply nested)
        let mut func = create_test_metrics("output_unified_priorities", 12, 19, 38);
        func.nesting = 3;

        let func_id = FunctionId::new(
            PathBuf::from("main.rs"),
            "output_unified_priorities".to_string(),
            861,
        );

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::IOWrapper);

        // Test that high nesting disqualifies I/O orchestration
        func.nesting = 4;
        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::PureLogic);
    }

    #[test]
    fn test_pure_logic_classification() {
        let graph = CallGraph::new();
        let func = create_test_metrics("calculate_risk", 8, 12, 60);
        let func_id = FunctionId::new(PathBuf::from("calc.rs"), "calculate_risk".to_string(), 20);

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(role, FunctionRole::PureLogic);
    }

    #[test]
    fn test_role_multipliers() {
        // Test with updated configuration values (spec 63)
        assert_eq!(get_role_multiplier(FunctionRole::PureLogic), 1.2);
        assert_eq!(get_role_multiplier(FunctionRole::Orchestrator), 0.8);
        assert_eq!(get_role_multiplier(FunctionRole::IOWrapper), 0.7);
        assert_eq!(get_role_multiplier(FunctionRole::EntryPoint), 0.9);
        assert_eq!(get_role_multiplier(FunctionRole::PatternMatch), 0.6);
        assert_eq!(get_role_multiplier(FunctionRole::Unknown), 1.0);
    }

    #[test]
    fn test_formatting_function_not_orchestrator() {
        let mut graph = CallGraph::new();

        // Create a function like format_recommendation_box_header
        let func = create_test_metrics("format_recommendation_box_header", 1, 0, 9);
        let func_id = FunctionId::new(
            PathBuf::from("insights.rs"),
            "format_recommendation_box_header".to_string(),
            142,
        );

        // Add the function to the graph
        graph.add_function(func_id.clone(), false, false, 1, 9);

        // Add callees: calculate_dash_count and format (from macro)
        let callee1 = FunctionId::new(
            PathBuf::from("insights.rs"),
            "calculate_dash_count".to_string(),
            138,
        );
        let callee2 = FunctionId::new(PathBuf::from("std"), "format".to_string(), 1);

        graph.add_function(callee1.clone(), false, false, 1, 3);
        graph.add_function(callee2.clone(), false, false, 1, 1);

        graph.add_call(FunctionCall {
            caller: func_id.clone(),
            callee: callee1,
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: func_id.clone(),
            callee: callee2,
            call_type: CallType::Direct,
        });

        // Test that it's not classified as orchestrator
        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(
            role,
            FunctionRole::PureLogic,
            "Formatting function should be PureLogic, not Orchestrator"
        );

        // Verify it doesn't match delegation pattern
        assert!(
            !delegates_to_tested_functions(&func_id, &graph, 0.8),
            "Should not be considered delegation when calling std functions"
        );
    }

    #[test]
    fn test_actual_orchestrator_with_meaningful_callees() {
        let mut graph = CallGraph::new();

        // Create an actual orchestrator function
        let func = create_test_metrics("coordinate_workflow", 2, 3, 15);
        let func_id = FunctionId::new(
            PathBuf::from("workflow.rs"),
            "coordinate_workflow".to_string(),
            10,
        );

        graph.add_function(func_id.clone(), false, false, 2, 15);

        // Add meaningful callees (not std library)
        // Need at least 3 for the current config settings
        let callee1 = FunctionId::new(
            PathBuf::from("workflow.rs"),
            "process_step_one".to_string(),
            50,
        );
        let callee2 = FunctionId::new(
            PathBuf::from("workflow.rs"),
            "process_step_two".to_string(),
            100,
        );
        let callee3 = FunctionId::new(
            PathBuf::from("workflow.rs"),
            "process_step_three".to_string(),
            150,
        );

        graph.add_function(callee1.clone(), false, false, 5, 30);
        graph.add_function(callee2.clone(), false, false, 5, 30);
        graph.add_function(callee3.clone(), false, false, 5, 30);

        graph.add_call(FunctionCall {
            caller: func_id.clone(),
            callee: callee1,
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: func_id.clone(),
            callee: callee2,
            call_type: CallType::Direct,
        });
        graph.add_call(FunctionCall {
            caller: func_id.clone(),
            callee: callee3,
            call_type: CallType::Direct,
        });

        // This should be classified as orchestrator
        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(
            role,
            FunctionRole::Orchestrator,
            "Function coordinating multiple steps should be Orchestrator"
        );
    }

    #[test]
    fn test_orchestrator_with_error_handling_complexity_5() {
        let mut graph = CallGraph::new();

        // Function with complexity 5 from Result handling (spec 117)
        let func = create_test_metrics("coordinate_tasks", 5, 3, 20);
        let func_id = FunctionId::new(
            PathBuf::from("tasks.rs"),
            "coordinate_tasks".to_string(),
            10,
        );

        graph.add_function(func_id.clone(), false, false, 5, 20);

        // Add 4 meaningful callees (20% of 20 lines = 4 calls for delegation ratio)
        for i in 0..4 {
            let callee_id = FunctionId::new(
                PathBuf::from("worker.rs"),
                format!("worker_task_{}", i),
                i * 20,
            );
            graph.add_function(callee_id.clone(), false, false, 8, 40);
            graph.add_call(FunctionCall {
                caller: func_id.clone(),
                callee: callee_id,
                call_type: CallType::Direct,
            });
        }

        // Should be classified as orchestrator despite higher complexity
        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(
            role,
            FunctionRole::Orchestrator,
            "Function with complexity 5 and good delegation ratio should be Orchestrator"
        );
    }

    #[test]
    fn test_delegation_ratio_calculation() {
        let func = create_test_metrics("orchestrator", 4, 2, 20);

        // Create test callees - 4 calls in 20 lines = 20% ratio
        let callees_vec: Vec<FunctionId> = (0..4)
            .map(|i| FunctionId::new(PathBuf::from("test.rs"), format!("callee_{}", i), i * 10))
            .collect();

        let callees: Vec<&FunctionId> = callees_vec.iter().collect();

        let ratio = calculate_delegation_ratio(&func, &callees);
        assert!(
            (ratio - 0.2).abs() < 0.01,
            "Expected delegation ratio of 0.2, got {}",
            ratio
        );
    }

    #[test]
    fn test_high_complexity_not_orchestrator() {
        let mut graph = CallGraph::new();

        // Function with complexity > 5 should not be orchestrator
        let func = create_test_metrics("complex_logic", 8, 10, 30);
        let func_id = FunctionId::new(PathBuf::from("logic.rs"), "complex_logic".to_string(), 10);

        graph.add_function(func_id.clone(), false, false, 8, 30);

        // Even with callees, complexity > 5 means not orchestrator
        for i in 0..3 {
            let callee_id =
                FunctionId::new(PathBuf::from("worker.rs"), format!("worker_{}", i), i * 20);
            graph.add_function(callee_id.clone(), false, false, 5, 20);
            graph.add_call(FunctionCall {
                caller: func_id.clone(),
                callee: callee_id,
                call_type: CallType::Direct,
            });
        }

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(
            role,
            FunctionRole::PureLogic,
            "Function with complexity > 5 should be PureLogic, not Orchestrator"
        );
    }

    #[test]
    fn test_simple_constructor_detection() {
        // Test: ContextMatcher::any() case (spec 117)
        let func = create_test_metrics("any", 1, 0, 9);
        assert!(
            is_simple_constructor(&func),
            "any() should be detected as constructor"
        );

        // Test: Standard new() constructor
        let func = create_test_metrics("new", 1, 0, 5);
        assert!(
            is_simple_constructor(&func),
            "new() should be detected as constructor"
        );

        // Test: from_* constructor
        let func = create_test_metrics("from_config", 1, 0, 8);
        assert!(
            is_simple_constructor(&func),
            "from_config() should be detected as constructor"
        );

        // Test: with_* constructor
        let func = create_test_metrics("with_defaults", 2, 1, 12);
        assert!(
            is_simple_constructor(&func),
            "with_defaults() should be detected as constructor"
        );

        // Test: Complex factory should NOT match
        let func = create_test_metrics("create_complex", 8, 12, 50);
        assert!(
            !is_simple_constructor(&func),
            "Complex function should NOT be detected as constructor"
        );

        // Test: Long function should NOT match
        let func = create_test_metrics("new_complex", 2, 2, 25);
        assert!(
            !is_simple_constructor(&func),
            "Long function should NOT be detected as constructor"
        );

        // Test: High cognitive complexity should NOT match
        let func = create_test_metrics("new_with_logic", 2, 8, 10);
        assert!(
            !is_simple_constructor(&func),
            "High cognitive complexity should NOT be detected as constructor"
        );
    }

    #[test]
    fn test_constructor_classification_precedence() {
        let graph = CallGraph::new();
        let func = create_test_metrics("any", 1, 0, 9);
        let func_id = FunctionId::new(PathBuf::from("context/rules.rs"), "any".to_string(), 52);

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(
            role,
            FunctionRole::IOWrapper,
            "Simple constructor should be classified as IOWrapper, not PureLogic"
        );
    }

    #[test]
    fn test_constructor_name_patterns() {
        // Test exact matches
        assert!(is_simple_constructor(&create_test_metrics("new", 1, 0, 5)));
        assert!(is_simple_constructor(&create_test_metrics(
            "default", 1, 0, 5
        )));
        assert!(is_simple_constructor(&create_test_metrics(
            "empty", 1, 0, 5
        )));
        assert!(is_simple_constructor(&create_test_metrics("zero", 1, 0, 5)));

        // Test prefix matches
        assert!(is_simple_constructor(&create_test_metrics(
            "from_str", 1, 0, 8
        )));
        assert!(is_simple_constructor(&create_test_metrics(
            "with_capacity",
            2,
            1,
            10
        )));
        assert!(is_simple_constructor(&create_test_metrics(
            "create_default",
            1,
            0,
            7
        )));
        assert!(is_simple_constructor(&create_test_metrics(
            "make_instance",
            1,
            0,
            6
        )));
        assert!(is_simple_constructor(&create_test_metrics(
            "build_config",
            2,
            2,
            12
        )));

        // Test non-constructor names
        let func = create_test_metrics("calculate_score", 1, 0, 5);
        assert!(
            !is_simple_constructor(&func),
            "Non-constructor name should not match"
        );
    }

    #[test]
    fn test_constructor_complexity_thresholds() {
        // Test at threshold boundaries
        let func = create_test_metrics("new", 2, 3, 14);
        assert!(
            is_simple_constructor(&func),
            "At threshold limits should match"
        );

        // Test just over cyclomatic threshold
        let func = create_test_metrics("new", 3, 2, 10);
        assert!(
            !is_simple_constructor(&func),
            "Over cyclomatic threshold should not match"
        );

        // Test just over cognitive threshold
        let func = create_test_metrics("new", 1, 4, 10);
        assert!(
            !is_simple_constructor(&func),
            "Over cognitive threshold should not match"
        );

        // Test just over length threshold
        let func = create_test_metrics("new", 1, 2, 15);
        assert!(
            !is_simple_constructor(&func),
            "Over length threshold should not match"
        );

        // Test nesting threshold
        let mut func = create_test_metrics("new", 1, 2, 10);
        func.nesting = 2;
        assert!(
            !is_simple_constructor(&func),
            "Over nesting threshold should not match"
        );
    }

    #[test]
    fn test_ast_detects_non_standard_constructor() {
        use syn::parse_quote;

        let source: syn::ItemFn = parse_quote! {
            pub fn create_default_client() -> Self {
                Self {
                    timeout: Duration::from_secs(30),
                    retries: 3,
                }
            }
        };

        let func = create_test_metrics("create_default_client", 1, 0, 5);

        // With AST: should be detected as constructor
        assert!(
            is_constructor_enhanced(&func, Some(&source)),
            "AST should detect non-standard constructor name"
        );

        // Without AST: fallback to name-based (should also match due to create_ prefix)
        assert!(
            is_constructor_enhanced(&func, None),
            "Should fallback to name-based detection"
        );
    }

    #[test]
    fn test_ast_detects_result_self_constructor() {
        use syn::parse_quote;

        let source: syn::ItemFn = parse_quote! {
            pub fn try_new(value: i32) -> Result<Self, Error> {
                if value > 0 {
                    Ok(Self { value })
                } else {
                    Err(Error::InvalidValue)
                }
            }
        };

        let func = create_test_metrics("try_new", 2, 1, 8);

        // With AST: should be detected as constructor (Result<Self>)
        assert!(
            is_constructor_enhanced(&func, Some(&source)),
            "AST should detect Result<Self> constructor"
        );
    }

    #[test]
    fn test_ast_rejects_loop_in_constructor() {
        use syn::parse_quote;

        let source: syn::ItemFn = parse_quote! {
            pub fn process_items() -> Self {
                let mut result = Self::new();
                for item in items {
                    result.add(item);
                }
                result
            }
        };

        let func = create_test_metrics("process_items", 2, 3, 8);

        // Should NOT be detected as constructor due to loop
        assert!(
            !is_constructor_enhanced(&func, Some(&source)),
            "AST should reject constructors with loops"
        );
    }

    #[test]
    fn test_ast_fallback_when_not_returning_self() {
        use syn::parse_quote;

        let source: syn::ItemFn = parse_quote! {
            pub fn calculate_value() -> i32 {
                42
            }
        };

        let func = create_test_metrics("calculate_value", 1, 0, 3);

        // Should fallback to name-based (which will reject non-constructor name)
        assert!(
            !is_constructor_enhanced(&func, Some(&source)),
            "AST should fallback when not returning Self"
        );
    }

    #[test]
    fn test_enum_converter_framework_type_name() {
        use syn::parse_quote;

        let source: syn::ItemFn = parse_quote! {
            pub fn name(&self) -> &'static str {
                match self {
                    FrameworkType::Django => "Django",
                    FrameworkType::Flask => "Flask",
                    FrameworkType::PyQt => "PyQt",
                }
            }
        };

        let func = create_test_metrics("name", 3, 0, 7);

        // Should be detected as enum converter
        assert!(
            is_enum_converter_enhanced(&func, &source),
            "FrameworkType::name should be detected as enum converter"
        );
    }

    #[test]
    fn test_enum_converter_builtin_exception_as_str() {
        use syn::parse_quote;

        let source: syn::ItemFn = parse_quote! {
            fn as_str(&self) -> &str {
                match self {
                    Self::BaseException => "BaseException",
                    Self::ValueError => "ValueError",
                    Self::TypeError => "TypeError",
                }
            }
        };

        let func = create_test_metrics("as_str", 3, 0, 7);

        // Should be detected as enum converter
        assert!(
            is_enum_converter_enhanced(&func, &source),
            "BuiltinException::as_str should be detected as enum converter"
        );
    }

    #[test]
    fn test_enum_converter_with_function_calls_not_detected() {
        use syn::parse_quote;

        let source: syn::ItemFn = parse_quote! {
            pub fn process(&self) -> String {
                match self {
                    Variant::A => format!("A"),
                    Variant::B => format!("B"),
                }
            }
        };

        let func = create_test_metrics("process", 2, 1, 7);

        // Should NOT be detected (has function calls)
        assert!(
            !is_enum_converter_enhanced(&func, &source),
            "Converter with function calls should NOT be detected"
        );
    }

    #[test]
    fn test_enum_converter_high_cognitive_complexity_rejected() {
        use syn::parse_quote;

        let source: syn::ItemFn = parse_quote! {
            pub fn name(&self) -> &'static str {
                match self {
                    Type::A => "A",
                    Type::B => "B",
                }
            }
        };

        let func = create_test_metrics("name", 2, 5, 7); // cognitive = 5

        // Should be rejected due to high cognitive complexity
        assert!(
            !is_enum_converter_enhanced(&func, &source),
            "High cognitive complexity should be rejected"
        );
    }

    #[test]
    fn test_enum_converter_classification_precedence() {
        use syn::parse_quote;

        let graph = CallGraph::new();

        let source: syn::ItemFn = parse_quote! {
            pub fn name(&self) -> &'static str {
                match self {
                    FrameworkType::Django => "Django",
                    FrameworkType::Flask => "Flask",
                }
            }
        };

        let func = create_test_metrics("name", 2, 0, 7);
        let func_id = FunctionId::new(PathBuf::from("framework.rs"), "name".to_string(), 10);

        let role = classify_by_rules(&func, &func_id, &graph, Some(&source));
        assert_eq!(
            role,
            Some(FunctionRole::IOWrapper),
            "Enum converter should be classified as IOWrapper"
        );
    }

    #[test]
    fn test_ast_detection_can_be_disabled() {
        use syn::parse_quote;

        let source: syn::ItemFn = parse_quote! {
            pub fn create_default_client() -> Self {
                Self { field: 0 }
            }
        };

        let func = create_test_metrics("create_default_client", 1, 0, 5);

        // When AST detection is disabled via config, should use name-based only
        // NOTE: This test assumes config can be set, which currently uses a static global
        // In a real implementation, we'd inject config or use a test-specific config

        // With AST enabled (default), should detect
        assert!(
            is_constructor_enhanced(&func, Some(&source)),
            "Should detect with AST when enabled"
        );

        // Without AST parameter (None), should fallback to name-based
        assert!(
            is_constructor_enhanced(&func, None),
            "Should fallback to name-based when AST unavailable"
        );
    }

    // Accessor detection tests (spec 125)

    #[test]
    fn test_simple_field_accessor_detected() {
        let metrics = create_test_metrics("id", 1, 0, 3);
        assert!(
            is_accessor_method(&metrics, None),
            "id() should be detected as accessor"
        );

        let metrics = create_test_metrics("get_name", 1, 0, 5);
        assert!(
            is_accessor_method(&metrics, None),
            "get_name() should be detected as accessor"
        );
    }

    #[test]
    fn test_bool_accessor_detected() {
        let metrics = create_test_metrics("is_active", 2, 1, 8);
        assert!(
            is_accessor_method(&metrics, None),
            "is_active() should be detected as accessor"
        );

        let metrics = create_test_metrics("has_permission", 2, 0, 5);
        assert!(
            is_accessor_method(&metrics, None),
            "has_permission() should be detected as accessor"
        );
    }

    #[test]
    fn test_converter_method_detected() {
        let metrics = create_test_metrics("as_str", 1, 0, 3);
        assert!(
            is_accessor_method(&metrics, None),
            "as_str() should be detected as accessor"
        );

        let metrics = create_test_metrics("to_string", 1, 0, 4);
        assert!(
            is_accessor_method(&metrics, None),
            "to_string() should be detected as accessor"
        );
    }

    #[test]
    fn test_complex_method_not_detected_as_accessor() {
        // High complexity despite accessor name
        let metrics = create_test_metrics("get_value", 5, 3, 20);
        assert!(
            !is_accessor_method(&metrics, None),
            "Complex method should not be detected as accessor"
        );
    }

    #[test]
    fn test_business_logic_not_misclassified_as_accessor() {
        // Business logic method
        let metrics = create_test_metrics("calculate_total", 4, 2, 15);
        assert!(
            !is_accessor_method(&metrics, None),
            "Business logic should not be detected as accessor"
        );
    }

    #[test]
    fn test_ast_body_validation_for_accessor() {
        use syn::parse_quote;

        let code: syn::ItemFn = parse_quote! {
            pub fn id(&self) -> u32 {
                self.id
            }
        };

        assert!(
            is_simple_accessor_body(&code),
            "Simple field access should be valid accessor body"
        );
    }

    #[test]
    fn test_accessor_side_effect_rejected() {
        use syn::parse_quote;

        let code: syn::ItemFn = parse_quote! {
            pub fn get_value(&self) -> i32 {
                self.counter.fetch_add(1, Ordering::SeqCst);
                self.value
            }
        };

        assert!(
            !is_simple_accessor_body(&code),
            "Accessor with side effects should be rejected"
        );
    }

    #[test]
    fn test_accessor_classification_integration() {
        let graph = CallGraph::new();

        // Simple accessor should be IOWrapper
        let func = create_test_metrics("id", 1, 0, 3);
        let func_id = FunctionId::new(PathBuf::from("user.rs"), "id".to_string(), 10);

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(
            role,
            FunctionRole::IOWrapper,
            "Simple accessor should be classified as IOWrapper"
        );
    }

    #[test]
    fn test_accessor_with_reference_return() {
        use syn::parse_quote;

        let code: syn::ItemFn = parse_quote! {
            pub fn name(&self) -> &str {
                &self.name
            }
        };

        assert!(
            is_simple_accessor_body(&code),
            "Reference to field should be valid accessor body"
        );
    }

    #[test]
    fn test_accessor_with_method_call() {
        use syn::parse_quote;

        let code: syn::ItemFn = parse_quote! {
            pub fn name(&self) -> String {
                self.name.clone()
            }
        };

        assert!(
            is_simple_accessor_body(&code),
            "Field with .clone() should be valid accessor body"
        );
    }

    #[test]
    fn test_accessor_single_word_patterns() {
        // Test all single-word patterns
        for name in &[
            "id", "name", "value", "kind", "type", "status", "code", "key", "index",
        ] {
            let metrics = create_test_metrics(name, 1, 0, 3);
            assert!(
                is_accessor_method(&metrics, None),
                "{} should be detected as accessor",
                name
            );
        }
    }

    #[test]
    fn test_accessor_prefix_patterns() {
        // Test all prefix patterns
        for (name, expected) in &[
            ("get_value", true),
            ("is_active", true),
            ("has_data", true),
            ("can_execute", true),
            ("should_run", true),
            ("as_str", true),
            ("to_string", true),
            ("into_iter", true),
        ] {
            let metrics = create_test_metrics(name, 1, 0, 5);
            assert_eq!(
                is_accessor_method(&metrics, None),
                *expected,
                "{} accessor detection mismatch",
                name
            );
        }
    }

    #[test]
    fn test_accessor_complexity_thresholds() {
        // At threshold (should pass)
        let metrics = create_test_metrics("get_value", 2, 1, 9);
        assert!(
            is_accessor_method(&metrics, None),
            "At threshold should be detected"
        );

        // Over cyclomatic threshold (should fail)
        let metrics = create_test_metrics("get_value", 3, 1, 9);
        assert!(
            !is_accessor_method(&metrics, None),
            "Over cyclomatic threshold should not be detected"
        );

        // Over cognitive threshold (should fail)
        let metrics = create_test_metrics("get_value", 2, 2, 9);
        assert!(
            !is_accessor_method(&metrics, None),
            "Over cognitive threshold should not be detected"
        );

        // Over length threshold (should fail)
        let metrics = create_test_metrics("get_value", 2, 1, 10);
        assert!(
            !is_accessor_method(&metrics, None),
            "At or over length threshold should not be detected"
        );
    }

    #[test]
    fn test_accessor_requires_immutable_self() {
        use syn::parse_quote;

        // Immutable self - should pass
        let code: syn::ItemFn = parse_quote! {
            pub fn value(&self) -> i32 {
                self.value
            }
        };

        assert!(
            is_simple_accessor_body(&code),
            "Immutable self should be valid"
        );

        // Mutable self - should fail
        let code: syn::ItemFn = parse_quote! {
            pub fn value(&mut self) -> i32 {
                self.value
            }
        };

        assert!(
            !is_simple_accessor_body(&code),
            "Mutable self should be rejected"
        );
    }

    #[test]
    fn test_accessor_precedence_in_classification() {
        use syn::parse_quote;

        let graph = CallGraph::new();

        let code: syn::ItemFn = parse_quote! {
            pub fn id(&self) -> u32 {
                self.id
            }
        };

        let func = create_test_metrics("id", 1, 0, 3);
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "id".to_string(), 10);

        let role = classify_by_rules(&func, &func_id, &graph, Some(&code));
        assert_eq!(
            role,
            Some(FunctionRole::IOWrapper),
            "Accessor should be classified as IOWrapper"
        );
    }

    // Debug role detection tests (spec 119)

    #[test]
    fn test_debug_function_with_diagnostics_suffix() {
        let func = create_test_metrics("handle_call_graph_diagnostics", 5, 3, 20);
        assert!(
            is_debug_function(&func),
            "Function with _diagnostics suffix should be detected as debug"
        );
    }

    #[test]
    fn test_debug_function_with_debug_prefix() {
        let func = create_test_metrics("debug_print_info", 3, 2, 15);
        assert!(
            is_debug_function(&func),
            "Function with debug_ prefix should be detected as debug"
        );
    }

    #[test]
    fn test_print_function_detected_as_debug() {
        let func = create_test_metrics("print_statistics", 4, 3, 18);
        assert!(
            is_debug_function(&func),
            "Function with print_ prefix should be detected as debug"
        );
    }

    #[test]
    fn test_dump_function_detected_as_debug() {
        let func = create_test_metrics("dump_state", 2, 1, 10);
        assert!(
            is_debug_function(&func),
            "Function with dump_ prefix should be detected as debug"
        );
    }

    #[test]
    fn test_trace_function_detected_as_debug() {
        let func = create_test_metrics("trace_execution", 3, 2, 12);
        assert!(
            is_debug_function(&func),
            "Function with trace_ prefix should be detected as debug"
        );
    }

    #[test]
    fn test_high_complexity_debug_name_not_debug() {
        // High complexity should prevent misclassification
        let func = create_test_metrics("debug_complex_algorithm", 15, 12, 50);
        assert!(
            !is_debug_function(&func),
            "High complexity function should not be classified as debug even with debug name"
        );
    }

    #[test]
    fn test_debug_stats_function() {
        let func = create_test_metrics("calculate_stats", 4, 3, 20);
        assert!(
            is_debug_function(&func),
            "Function ending with _stats should be detected as debug"
        );
    }

    #[test]
    fn test_debug_classification_integration() {
        let graph = CallGraph::new();
        // Use a function that clearly matches debug pattern without entry point pattern
        let func = create_test_metrics("print_call_graph_diagnostics", 5, 3, 20);
        let func_id = FunctionId::new(
            PathBuf::from("commands/analyze.rs"),
            "print_call_graph_diagnostics".to_string(),
            396,
        );

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(
            role,
            FunctionRole::Debug,
            "Diagnostic function should be classified as Debug role"
        );
    }

    #[test]
    fn test_entry_point_takes_precedence_over_debug() {
        let graph = CallGraph::new();
        // Function with both entry point and debug patterns - entry point wins
        let func = create_test_metrics("handle_diagnostics", 5, 3, 20);
        let func_id = FunctionId::new(
            PathBuf::from("commands/analyze.rs"),
            "handle_diagnostics".to_string(),
            396,
        );

        let role = classify_function_role(&func, &func_id, &graph);
        assert_eq!(
            role,
            FunctionRole::EntryPoint,
            "Entry point pattern should take precedence over debug pattern"
        );
    }

    #[test]
    fn test_business_logic_not_misclassified_as_debug() {
        let func = create_test_metrics("calculate_complexity", 8, 5, 30);
        assert!(
            !is_debug_function(&func),
            "Business logic function should not be detected as debug"
        );
    }

    #[test]
    fn test_debug_role_multiplier() {
        let multiplier = get_role_multiplier(FunctionRole::Debug);
        assert_eq!(
            multiplier, 0.3,
            "Debug role should have lowest multiplier (0.3)"
        );
    }

    #[test]
    fn test_matches_debug_pattern() {
        assert!(matches_debug_pattern("debug_foo"));
        assert!(matches_debug_pattern("print_bar"));
        assert!(matches_debug_pattern("dump_state"));
        assert!(matches_debug_pattern("trace_execution"));
        assert!(matches_debug_pattern("foo_diagnostics"));
        assert!(matches_debug_pattern("bar_debug"));
        assert!(matches_debug_pattern("baz_stats"));
        assert!(matches_debug_pattern("test_diagnostics_helper"));

        // Should not match
        assert!(!matches_debug_pattern("calculate"));
        assert!(!matches_debug_pattern("process"));
        assert!(!matches_debug_pattern("validate"));
    }

    #[test]
    fn test_diagnostic_characteristics() {
        // Should match: simple output function
        let func = create_test_metrics("print_output", 3, 2, 15);
        assert!(
            has_diagnostic_characteristics(&func),
            "Simple print function should have diagnostic characteristics"
        );

        // Should not match: complex function
        let func = create_test_metrics("print_complex", 10, 8, 40);
        assert!(
            !has_diagnostic_characteristics(&func),
            "Complex function should not have diagnostic characteristics"
        );

        // Should not match: no output I/O pattern
        let func = create_test_metrics("calculate_total", 3, 2, 15);
        assert!(
            !has_diagnostic_characteristics(&func),
            "Non-output function should not have diagnostic characteristics"
        );

        // Should not match: read/write operations (not diagnostic output)
        let func = create_test_metrics("read_file", 3, 2, 15);
        assert!(
            !has_diagnostic_characteristics(&func),
            "Read/write operations should not have diagnostic characteristics"
        );
    }
}
