//! Classification rule functions for semantic analysis
//!
//! This module contains the core classification logic that determines
//! function roles based on metrics, patterns, and AST analysis.

use crate::analyzers::rust_constructor_detector::{
    analyze_function_body, extract_return_type, ConstructorReturnType,
};
use crate::analyzers::rust_enum_converter_detector::is_enum_converter;
use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use super::pattern_matchers::{matches_debug_pattern, matches_output_io_pattern};

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
pub(crate) fn is_debug_function(func: &FunctionMetrics) -> bool {
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

/// Check if function has diagnostic behavioral characteristics
pub(crate) fn has_diagnostic_characteristics(func: &FunctionMetrics) -> bool {
    // Diagnostic functions typically have:
    // - Very low complexity (< 5)
    // - Short length (< 20 lines)
    // - Output-focused I/O patterns (print, display, log, not read/write/load/save)
    let is_very_simple = func.cognitive < 5 && func.length < 20;
    let has_output_io_name = matches_output_io_pattern(&func.name);

    is_very_simple && has_output_io_name
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
pub(crate) fn is_simple_constructor(func: &FunctionMetrics) -> bool {
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
pub(crate) fn is_constructor_enhanced(func: &FunctionMetrics, syn_func: Option<&syn::ItemFn>) -> bool {
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
pub(crate) fn is_enum_converter_enhanced(func: &FunctionMetrics, syn_func: &syn::ItemFn) -> bool {
    is_enum_converter(func, syn_func)
}

/// Check if function is a pattern matching function
///
/// Pattern matching functions typically have low cyclomatic but high cognitive complexity
/// due to many branches in if/else chains or match statements.
pub(crate) fn is_pattern_matching_function(func: &FunctionMetrics, func_id: &FunctionId) -> bool {
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

/// Check if function is an orchestrator
///
/// Orchestrators coordinate multiple other functions with simple delegation logic.
pub(crate) fn is_orchestrator(func: &FunctionMetrics, func_id: &FunctionId, call_graph: &CallGraph) -> bool {
    use super::pattern_matchers::is_orchestrator_by_name;

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

/// Check if function is an I/O wrapper
pub(crate) fn is_io_wrapper(func: &FunctionMetrics) -> bool {
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

/// Calculate delegation ratio for a function
///
/// Returns the ratio of function calls to total statements (approximated by function length).
/// A higher ratio indicates more coordination/delegation behavior.
pub(crate) fn calculate_delegation_ratio(func: &FunctionMetrics, meaningful_callees: &[&FunctionId]) -> f64 {
    if func.length == 0 {
        return 0.0;
    }
    meaningful_callees.len() as f64 / func.length as f64
}

pub(crate) fn delegates_to_tested_functions(
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
