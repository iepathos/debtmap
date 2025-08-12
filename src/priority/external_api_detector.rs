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

    // Check module boundary indicators
    let (boundary_score, boundary_indicator) = classify_module_boundary(&func.file);
    confidence_score += boundary_score;
    if let Some(indicator) = boundary_indicator {
        indicators.push(indicator);
    }

    // Check if re-exported in lib.rs (would need call graph info)
    // This is a placeholder for future enhancement
    if is_in_public_module(&func.file) {
        confidence_score += 2;
        indicators.push("In public module hierarchy".to_string());
    }

    // Check for API naming patterns
    let (pattern_score, pattern_indicators) = classify_api_patterns(&func.name);
    confidence_score += pattern_score;
    indicators.extend(pattern_indicators);

    // Check module path depth - deeper paths less likely to be external API
    let (depth_score, depth_indicator) = classify_path_depth(&func.file);
    confidence_score += depth_score;
    if let Some(indicator) = depth_indicator {
        indicators.push(indicator);
    }

    // Decision threshold
    let is_likely_api = confidence_score >= 4;

    (is_likely_api, indicators)
}

/// Classify module boundary indicators (lib.rs, mod.rs)
fn classify_module_boundary(path: &Path) -> (i32, Option<String>) {
    if let Some(file_name) = path.file_name() {
        let name = file_name.to_string_lossy();
        match name.as_ref() {
            "lib.rs" => (3, Some("Defined in lib.rs (library root)".to_string())),
            "mod.rs" => (2, Some("Defined in mod.rs (module root)".to_string())),
            _ => (0, None),
        }
    } else {
        (0, None)
    }
}

/// Classify API patterns in function names
fn classify_api_patterns(name: &str) -> (i32, Vec<String>) {
    let mut score = 0;
    let mut indicators = Vec::new();

    // Check for common API patterns
    if has_api_pattern_in_name(name) {
        score += 2;
        indicators.push(format!("API pattern in name: {name}"));
    }

    // Check for builder/factory patterns
    if is_builder_or_factory(name) {
        score += 2;
        indicators.push("Builder/factory pattern detected".to_string());
    }

    // Check for trait implementations
    if is_trait_method_pattern(name) {
        score += 2;
        indicators.push("Common trait method pattern".to_string());
    }

    // Check for constructor patterns (gets higher score)
    if is_constructor_pattern(name) {
        score += 3;
        indicators.push("Constructor/initialization pattern".to_string());
    }

    // Check for public API prefixes
    if has_public_api_prefix(name) {
        score += 2;
        indicators.push("Public API prefix detected".to_string());
    }

    (score, indicators)
}

/// Check if function name matches trait method patterns
fn is_trait_method_pattern(name: &str) -> bool {
    name.starts_with("from_") || name == "new" || name == "default"
}

/// Classify module path depth
fn classify_path_depth(path: &Path) -> (i32, Option<String>) {
    let path_depth = path.components().count();
    match path_depth {
        0..=3 => (1, Some("Shallow module path (likely public)".to_string())),
        4..=5 => (0, None),
        _ => (-1, Some("Deep module path (likely internal)".to_string())),
    }
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
