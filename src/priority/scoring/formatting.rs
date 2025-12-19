// Formatting and helper utility functions for debt item processing
// Spec 262: Recommendation generation removed - only visibility determination remains

use crate::core::FunctionMetrics;
use crate::priority::FunctionVisibility;

/// Determine function visibility from metrics
pub fn determine_visibility(func: &FunctionMetrics) -> FunctionVisibility {
    // Use the visibility field from FunctionMetrics if available
    match &func.visibility {
        Some(vis) if vis == "pub" => FunctionVisibility::Public,
        Some(vis) if vis == "pub(crate)" => FunctionVisibility::Crate,
        Some(vis) if vis.starts_with("pub(") => FunctionVisibility::Crate, // pub(super), pub(in ...), etc.
        _ => FunctionVisibility::Private,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_visibility() {
        let pub_func = FunctionMetrics {
            name: "test".to_string(),
            file: "test.rs".into(),
            line: 1,
            cyclomatic: 1,
            cognitive: 1,
            nesting: 1,
            length: 10,
            is_test: false,
            visibility: Some("pub".to_string()),
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
        };

        assert_eq!(determine_visibility(&pub_func), FunctionVisibility::Public);

        let priv_func = FunctionMetrics {
            visibility: None,
            ..pub_func.clone()
        };

        assert_eq!(
            determine_visibility(&priv_func),
            FunctionVisibility::Private
        );

        let crate_func = FunctionMetrics {
            visibility: Some("pub(crate)".to_string()),
            ..pub_func.clone()
        };

        assert_eq!(
            determine_visibility(&crate_func),
            FunctionVisibility::Crate
        );

        let super_func = FunctionMetrics {
            visibility: Some("pub(super)".to_string()),
            ..pub_func.clone()
        };

        assert_eq!(
            determine_visibility(&super_func),
            FunctionVisibility::Crate
        );
    }
}
