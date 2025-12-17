//! Dead code analysis and hints generation
//!
//! Pure functions for analyzing function visibility and generating
//! actionable hints for dead code removal.
//! Following Stillwater philosophy: pure core, focused responsibility.

use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::priority::FunctionVisibility;

/// Determine visibility level from function metadata
///
/// Maps visibility string from AST to our visibility enum.
pub fn determine_visibility(func: &FunctionMetrics) -> FunctionVisibility {
    let vis_str = func.visibility.as_deref();
    match vis_str {
        Some("pub") => FunctionVisibility::Public,
        Some("pub(crate)") => FunctionVisibility::Crate,
        Some(vis) if vis.starts_with("pub(") => FunctionVisibility::Crate,
        _ => FunctionVisibility::Private,
    }
}

/// Generate enhanced dead code hints based on visibility and context
///
/// Produces actionable hints for removing potentially dead code,
/// considering visibility level and file context (test files, etc).
pub fn generate_enhanced_dead_code_hints(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
) -> Vec<String> {
    let mut hints = Vec::new();

    // Add visibility-specific hint
    hints.push(visibility_hint(visibility));

    // Check for test context
    let file_str = func.file.to_string_lossy();
    if file_str.contains("test") {
        hints.push("Test-related function - may be test helper".to_string());
    }

    // Check for test function naming
    if func.name.starts_with("test_") {
        hints.push("Test function - verify no test dependencies".to_string());
    }

    hints
}

/// Generate visibility-specific hint
fn visibility_hint(visibility: &FunctionVisibility) -> String {
    match visibility {
        FunctionVisibility::Public => {
            "Public function - verify not used by external crates".to_string()
        }
        FunctionVisibility::Private => {
            "Private function - safe to remove if no local callers".to_string()
        }
        FunctionVisibility::Crate => {
            "Crate-visible function - check for usage within crate".to_string()
        }
    }
}

/// Generate comprehensive usage hints for dead code analysis
///
/// Combines visibility hints with call graph analysis to provide
/// complete context for dead code removal decisions.
pub fn generate_usage_hints(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> Vec<String> {
    let visibility = determine_visibility(func);
    let mut hints = generate_enhanced_dead_code_hints(func, &visibility);

    // Add call graph context
    let callees = call_graph.get_callees(func_id);
    if callees.is_empty() {
        hints.push("Function has no dependencies - safe to remove".to_string());
    } else {
        hints.push(format!("Function calls {} other functions", callees.len()));
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function(name: &str, visibility: Option<&str>) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 2,
            length: 50,
            is_test: false,
            visibility: visibility.map(|v| v.to_string()),
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }
    }

    fn create_test_function_with_file(
        name: &str,
        file: &str,
        visibility: Option<&str>,
    ) -> FunctionMetrics {
        let mut func = create_test_function(name, visibility);
        func.file = PathBuf::from(file);
        func
    }

    #[test]
    fn test_determine_visibility_public() {
        let func = create_test_function("test", Some("pub"));
        assert!(matches!(
            determine_visibility(&func),
            FunctionVisibility::Public
        ));
    }

    #[test]
    fn test_determine_visibility_crate() {
        let func = create_test_function("test", Some("pub(crate)"));
        assert!(matches!(
            determine_visibility(&func),
            FunctionVisibility::Crate
        ));

        let func_super = create_test_function("test", Some("pub(super)"));
        assert!(matches!(
            determine_visibility(&func_super),
            FunctionVisibility::Crate
        ));
    }

    #[test]
    fn test_determine_visibility_private() {
        let func = create_test_function("test", None);
        assert!(matches!(
            determine_visibility(&func),
            FunctionVisibility::Private
        ));
    }

    #[test]
    fn test_enhanced_dead_code_hints_public() {
        let func = create_test_function("my_function", Some("pub"));
        let visibility = FunctionVisibility::Public;

        let hints = generate_enhanced_dead_code_hints(&func, &visibility);

        assert!(!hints.is_empty());
        assert!(hints.contains(&"Public function - verify not used by external crates".to_string()));
    }

    #[test]
    fn test_enhanced_dead_code_hints_private() {
        let func = create_test_function("my_function", None);
        let visibility = FunctionVisibility::Private;

        let hints = generate_enhanced_dead_code_hints(&func, &visibility);

        assert!(!hints.is_empty());
        assert!(
            hints.contains(&"Private function - safe to remove if no local callers".to_string())
        );
    }

    #[test]
    fn test_enhanced_dead_code_hints_crate() {
        let func = create_test_function("my_function", Some("pub(crate)"));
        let visibility = FunctionVisibility::Crate;

        let hints = generate_enhanced_dead_code_hints(&func, &visibility);

        assert!(!hints.is_empty());
        assert!(
            hints.contains(&"Crate-visible function - check for usage within crate".to_string())
        );
    }

    #[test]
    fn test_enhanced_dead_code_hints_test_file() {
        let func =
            create_test_function_with_file("helper_function", "tests/integration_test.rs", None);
        let visibility = FunctionVisibility::Private;

        let hints = generate_enhanced_dead_code_hints(&func, &visibility);

        assert!(hints.len() >= 2);
        assert!(
            hints.contains(&"Private function - safe to remove if no local callers".to_string())
        );
        assert!(hints.contains(&"Test-related function - may be test helper".to_string()));
    }

    #[test]
    fn test_enhanced_dead_code_hints_test_function() {
        let func = create_test_function("test_something", None);
        let visibility = FunctionVisibility::Private;

        let hints = generate_enhanced_dead_code_hints(&func, &visibility);

        assert!(hints.len() >= 2);
        assert!(
            hints.contains(&"Private function - safe to remove if no local callers".to_string())
        );
        assert!(hints.contains(&"Test function - verify no test dependencies".to_string()));
    }

    #[test]
    fn test_enhanced_dead_code_hints_test_file_and_function() {
        let func =
            create_test_function_with_file("test_helper", "src/tests/helpers.rs", Some("pub"));
        let visibility = FunctionVisibility::Public;

        let hints = generate_enhanced_dead_code_hints(&func, &visibility);

        assert!(hints.len() >= 3);
        assert!(hints.contains(&"Public function - verify not used by external crates".to_string()));
        assert!(hints.contains(&"Test-related function - may be test helper".to_string()));
        assert!(hints.contains(&"Test function - verify no test dependencies".to_string()));
    }

    #[test]
    fn test_generate_usage_hints_basic() {
        let func = create_test_function("unused_func", None);
        let call_graph = CallGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "unused_func".to_string(), 10);

        let hints = generate_usage_hints(&func, &call_graph, &func_id);

        assert!(!hints.is_empty());
        assert!(hints.iter().any(|h| h.contains("Private function")));
    }
}
