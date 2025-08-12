use crate::core::FunctionMetrics;
use crate::priority::FunctionVisibility;
use std::path::Path;

/// Enhanced detection of whether a public function is likely part of an external API
pub fn is_likely_external_api(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
) -> (bool, Vec<String>) {
    let mut indicators = Vec::new();
    let mut confidence_score = 0;

    // Only analyze public functions
    if !matches!(visibility, FunctionVisibility::Public) {
        return (false, vec![]);
    }

    // Check if in lib.rs or mod.rs (module boundary)
    if let Some(file_name) = func.file.file_name() {
        let name = file_name.to_string_lossy();
        if name == "lib.rs" {
            confidence_score += 3;
            indicators.push("Defined in lib.rs (library root)".to_string());
        } else if name == "mod.rs" {
            confidence_score += 2;
            indicators.push("Defined in mod.rs (module root)".to_string());
        }
    }

    // Check if re-exported in lib.rs (would need call graph info)
    // This is a placeholder for future enhancement
    if is_in_public_module(&func.file) {
        confidence_score += 2;
        indicators.push("In public module hierarchy".to_string());
    }

    // Check for common API patterns in function name
    if has_api_pattern_in_name(&func.name) {
        confidence_score += 2;
        indicators.push(format!("API pattern in name: {}", func.name));
    }

    // Check for builder/factory patterns
    if is_builder_or_factory(&func.name) {
        confidence_score += 2;
        indicators.push("Builder/factory pattern detected".to_string());
    }

    // Check for trait implementations (would need more AST info)
    if func.name.starts_with("from_") || func.name == "new" || func.name == "default" {
        confidence_score += 2;
        indicators.push("Common trait method pattern".to_string());
    }

    // Check if it's a constructor or initialization function
    if is_constructor_pattern(&func.name) {
        confidence_score += 3;
        indicators.push("Constructor/initialization pattern".to_string());
    }

    // Functions with specific prefixes that indicate public API
    if has_public_api_prefix(&func.name) {
        confidence_score += 2;
        indicators.push("Public API prefix detected".to_string());
    }

    // Check module path depth - deeper paths less likely to be external API
    let path_depth = func.file.components().count();
    if path_depth <= 3 {
        confidence_score += 1;
        indicators.push("Shallow module path (likely public)".to_string());
    } else if path_depth > 5 {
        confidence_score -= 1;
        indicators.push("Deep module path (likely internal)".to_string());
    }

    // Decision threshold
    let is_likely_api = confidence_score >= 4;

    (is_likely_api, indicators)
}

/// Check if the file is in a public module hierarchy
fn is_in_public_module(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Check for common public module patterns
    !path_str.contains("/internal/")
        && !path_str.contains("/private/")
        && !path_str.contains("/impl/")
        && !path_str.contains("/detail/")
        && !path_str.contains("/tests/")
        && !path_str.contains("/benches/")
        && !path_str.contains("/examples/")
}

/// Check for API-like patterns in function names
fn has_api_pattern_in_name(name: &str) -> bool {
    name.starts_with("get_")
        || name.starts_with("set_")
        || name.starts_with("with_")
        || name.starts_with("try_")
        || name.starts_with("is_")
        || name.starts_with("has_")
        || name.starts_with("create_")
        || name.starts_with("parse_")
        || name.starts_with("to_")
        || name.starts_with("into_")
        || name.starts_with("as_")
}

/// Check for builder or factory patterns
fn is_builder_or_factory(name: &str) -> bool {
    name == "build"
        || name == "builder"
        || name.ends_with("_builder")
        || name.starts_with("create_")
        || name.starts_with("make_")
        || name.ends_with("_factory")
}

/// Check for constructor patterns
fn is_constructor_pattern(name: &str) -> bool {
    name == "new"
        || name == "default"
        || name.starts_with("new_")
        || name.starts_with("from_")
        || name == "init"
        || name.starts_with("init_")
}

/// Check for public API prefixes
fn has_public_api_prefix(name: &str) -> bool {
    name.starts_with("public_") || name.starts_with("api_") || name.starts_with("export_")
}

/// Generate enhanced dead code hints with API detection
pub fn generate_enhanced_dead_code_hints(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
) -> Vec<String> {
    let mut hints = Vec::new();

    // Get external API detection results
    let (is_likely_api, api_indicators) = is_likely_external_api(func, visibility);

    if is_likely_api {
        hints.push("⚠️ Likely external API - verify before removing".to_string());
        for indicator in api_indicators {
            hints.push(format!("  • {indicator}"));
        }
    } else if matches!(visibility, FunctionVisibility::Public) {
        hints.push("Public but no external API indicators found".to_string());
    }

    // Add existing hints based on function characteristics
    if func.cyclomatic <= 3 && func.cognitive <= 5 {
        hints.push("Low complexity - low impact removal".to_string());
    } else {
        hints.push("High complexity - removing may eliminate significant unused code".to_string());
    }

    // Check for test helper patterns
    if func.name.contains("helper") || func.name.contains("util") || func.name.contains("fixture") {
        hints.push("May be a test helper - consider moving to test module".to_string());
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function(name: &str, path: &str) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from(path),
            line: 1,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 2,
            length: 20,
            is_test: false,
            visibility: Some("pub".to_string()),
        }
    }

    #[test]
    fn test_lib_rs_detection() {
        let func = create_test_function("process_data", "src/lib.rs");
        let visibility = FunctionVisibility::Public;

        let (is_api, indicators) = is_likely_external_api(&func, &visibility);

        assert!(is_api);
        assert!(indicators.iter().any(|i| i.contains("lib.rs")));
    }

    #[test]
    fn test_constructor_pattern() {
        let func = create_test_function("new", "src/data/processor.rs");
        let visibility = FunctionVisibility::Public;

        let (is_api, indicators) = is_likely_external_api(&func, &visibility);

        assert!(is_api);
        assert!(indicators.iter().any(|i| i.contains("Constructor")));
    }

    #[test]
    fn test_deep_internal_module() {
        let func = create_test_function("helper", "src/internal/impl/detail/utils/helpers.rs");
        let visibility = FunctionVisibility::Public;

        let (is_api, _indicators) = is_likely_external_api(&func, &visibility);

        assert!(!is_api);
    }

    #[test]
    fn test_api_pattern_names() {
        let func = create_test_function("get_configuration", "src/config.rs");
        let visibility = FunctionVisibility::Public;

        let (_is_api, indicators) = is_likely_external_api(&func, &visibility);

        assert!(indicators.iter().any(|i| i.contains("API pattern")));
    }

    #[test]
    fn test_private_function() {
        let func = create_test_function("internal_helper", "src/utils.rs");
        let visibility = FunctionVisibility::Private;

        let (is_api, indicators) = is_likely_external_api(&func, &visibility);

        assert!(!is_api);
        assert!(indicators.is_empty());
    }
}
