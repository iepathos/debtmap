//! Pure functions for function role classification and context building.
//!
//! This module handles converting FunctionAnalysis to FunctionRole
//! and building RiskContext for risk assessment.

use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::priority::semantic_classifier::{classify_function_role, FunctionRole};
use crate::priority::{FunctionAnalysis, FunctionVisibility};
use crate::risk::evidence::RiskContext;
use std::path::Path;

use super::module_classifier::classify_module_type;

/// Classifies the role of a function based on its metrics and call graph position.
pub fn classify_role(function: &FunctionAnalysis, call_graph: &CallGraph) -> FunctionRole {
    let func_id = FunctionId::new(
        function.file.clone(),
        function.function.clone(),
        function.line,
    );

    let func_metrics = crate::core::FunctionMetrics {
        file: function.file.clone(),
        name: function.function.clone(),
        line: function.line,
        length: function.function_length,
        cyclomatic: function.cyclomatic_complexity,
        cognitive: function.cognitive_complexity,
        nesting: function.nesting_depth,
        is_test: function.is_test,
        visibility: Some(visibility_to_string(&function.visibility)),
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
    };

    classify_function_role(&func_metrics, &func_id, call_graph)
}

/// Converts FunctionVisibility enum to its string representation.
pub fn visibility_to_string(visibility: &FunctionVisibility) -> String {
    match visibility {
        FunctionVisibility::Public => "pub".to_string(),
        FunctionVisibility::Crate => "pub(crate)".to_string(),
        FunctionVisibility::Private => "".to_string(),
    }
}

/// Converts a FunctionRole to a human-readable string.
pub fn role_to_display_string(role: &FunctionRole) -> &'static str {
    match role {
        FunctionRole::PureLogic => "pure logic",
        FunctionRole::Orchestrator => "orchestrator",
        FunctionRole::IOWrapper => "I/O wrapper",
        FunctionRole::EntryPoint => "entry point",
        FunctionRole::PatternMatch => "pattern matching",
        FunctionRole::Debug => "debug/diagnostic",
        FunctionRole::Unknown => "general",
    }
}

/// Builds a RiskContext from function analysis and its classified role.
pub fn build_risk_context(
    function: &FunctionAnalysis,
    role: FunctionRole,
    file: &Path,
) -> RiskContext {
    RiskContext {
        role,
        visibility: function.visibility.clone(),
        module_type: classify_module_type(file),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visibility_to_string() {
        assert_eq!(visibility_to_string(&FunctionVisibility::Public), "pub");
        assert_eq!(
            visibility_to_string(&FunctionVisibility::Crate),
            "pub(crate)"
        );
        assert_eq!(visibility_to_string(&FunctionVisibility::Private), "");
    }

    #[test]
    fn test_role_to_display_string() {
        assert_eq!(
            role_to_display_string(&FunctionRole::PureLogic),
            "pure logic"
        );
        assert_eq!(
            role_to_display_string(&FunctionRole::Orchestrator),
            "orchestrator"
        );
        assert_eq!(
            role_to_display_string(&FunctionRole::IOWrapper),
            "I/O wrapper"
        );
        assert_eq!(
            role_to_display_string(&FunctionRole::EntryPoint),
            "entry point"
        );
        assert_eq!(
            role_to_display_string(&FunctionRole::PatternMatch),
            "pattern matching"
        );
        assert_eq!(
            role_to_display_string(&FunctionRole::Debug),
            "debug/diagnostic"
        );
        assert_eq!(role_to_display_string(&FunctionRole::Unknown), "general");
    }
}
