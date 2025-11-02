use crate::config;
use crate::core::FunctionMetrics;
use crate::priority::FunctionVisibility;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;

/// Configuration for external API detection
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExternalApiConfig {
    /// Whether to treat public functions as potential external APIs
    #[serde(default = "default_true")]
    pub detect_external_api: bool,

    /// Explicitly marked external API functions (format: "function_name" or "module::function_name")
    #[serde(default)]
    pub api_functions: Vec<String>,

    /// Files that contain external API functions (all public functions in these files are APIs)
    #[serde(default)]
    pub api_files: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Cache the configuration
static CONFIG: OnceLock<ExternalApiConfig> = OnceLock::new();

/// Get the cached configuration
fn get_config() -> &'static ExternalApiConfig {
    CONFIG.get_or_init(|| {
        config::get_config()
            .external_api
            .clone()
            .unwrap_or_default()
    })
}

/// Check if a function is explicitly marked as an external API
fn is_explicitly_marked_api(func: &FunctionMetrics, config: &ExternalApiConfig) -> bool {
    // Check if the function is in the explicit API functions list
    let func_matches = config.api_functions.iter().any(|api_func| {
        // Match either just the function name or module::function format
        api_func == &func.name || api_func.ends_with(&format!("::{}", func.name))
    });

    if func_matches {
        return true;
    }

    // Check if the file is in the API files list
    let file_path_str = func.file.to_string_lossy();
    config.api_files.iter().any(|api_file| {
        // Support both exact matches and pattern matches
        if api_file.contains('*') {
            // Simple glob pattern matching
            let pattern = api_file.replace("**", ".*").replace('*', "[^/]*");
            match regex::Regex::new(&pattern) {
                Ok(re) => re.is_match(&file_path_str),
                Err(e) => {
                    log::warn!(
                        "Invalid regex pattern for API file glob '{}': {}",
                        api_file,
                        e
                    );
                    false
                }
            }
        } else {
            // Exact file match or suffix match
            file_path_str.ends_with(api_file) || &file_path_str == api_file
        }
    })
}

/// Enhanced detection of whether a public function is likely part of an external API
pub fn is_likely_external_api(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
) -> (bool, Vec<String>) {
    // Check configuration
    let config = get_config();
    is_likely_external_api_with_config(func, visibility, config)
}

/// Internal version that accepts a config parameter (useful for testing)
#[doc(hidden)]
pub fn is_likely_external_api_with_config(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
    config: &ExternalApiConfig,
) -> (bool, Vec<String>) {
    let mut indicators = Vec::new();
    let mut confidence_score = 0;

    // Only analyze public functions
    if !matches!(visibility, FunctionVisibility::Public) {
        return (false, vec![]);
    }

    // Check if explicitly marked as API (this takes precedence over detection setting)
    if is_explicitly_marked_api(func, config) {
        return (
            true,
            vec!["Explicitly marked as external API in .debtmap.toml".to_string()],
        );
    }

    // Check if automatic external API detection is disabled
    if !config.detect_external_api {
        // Automatic detection is disabled, but explicit markings still work
        return (
            false,
            vec!["Automatic external API detection disabled in .debtmap.toml".to_string()],
        );
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
        indicators.push("API pattern".to_string());
    }

    // Check for builder/factory patterns
    if is_builder_or_factory(name) {
        score += 2;
        indicators.push("Builder".to_string());
    }

    // Check for trait implementations
    if is_trait_method_pattern(name) {
        score += 2;
        indicators.push("Common trait method pattern".to_string());
    }

    // Check for constructor patterns (gets higher score)
    if is_constructor_pattern(name) {
        score += 3;
        indicators.push("Constructor".to_string());
    }

    // Check for public API prefixes
    if has_public_api_prefix(name) {
        score += 2;
        indicators.push("Public API prefix".to_string());
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

/// Classify function complexity and generate appropriate hints
fn classify_complexity_hints(func: &FunctionMetrics) -> Option<String> {
    match (func.cyclomatic, func.cognitive) {
        (c, g) if c <= 3 && g <= 5 => Some("Low complexity - low impact removal".to_string()),
        (c, g) if c > 10 || g > 15 => {
            Some("High complexity - removing may eliminate significant unused code".to_string())
        }
        _ => None,
    }
}

/// Detect if function is likely a test helper based on naming patterns
fn detect_test_helper_hints(func_name: &str) -> Option<String> {
    let is_test_helper = (func_name.contains("test") && func_name.contains("helper"))
        || func_name.contains("mock")
        || func_name.contains("fixture")
        || func_name.contains("helper")
        || func_name.contains("util");

    if is_test_helper {
        Some("Potential test helper - consider moving to test module".to_string())
    } else {
        None
    }
}

/// Generate enhanced dead code hints with API detection
pub fn generate_enhanced_dead_code_hints(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
) -> Vec<String> {
    let config = get_config();
    generate_enhanced_dead_code_hints_with_config(func, visibility, config)
}

/// Internal version that accepts a config parameter (useful for testing)
#[doc(hidden)]
pub fn generate_enhanced_dead_code_hints_with_config(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
    config: &ExternalApiConfig,
) -> Vec<String> {
    let mut hints = Vec::new();

    // Get external API detection results
    let (is_likely_api, api_indicators) =
        is_likely_external_api_with_config(func, visibility, config);

    if is_likely_api {
        hints.push("Likely external API - verify before removing".to_string());
        hints.extend(api_indicators);
    } else if matches!(visibility, FunctionVisibility::Public) {
        hints.push("Public but no external API indicators found".to_string());
    }

    // Add complexity-based hints
    if let Some(hint) = classify_complexity_hints(func) {
        hints.push(hint);
    }

    // Add test helper hints
    if let Some(hint) = detect_test_helper_hints(&func.name) {
        hints.push(hint);
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
            composition_metrics: None,
            language_specific: None,
        }
    }

    // Create a test config with detection enabled
    fn test_config() -> ExternalApiConfig {
        ExternalApiConfig {
            detect_external_api: true,
            api_functions: vec![],
            api_files: vec![],
        }
    }

    #[test]
    fn test_lib_rs_detection() {
        let func = create_test_function("process_data", "src/lib.rs");
        let visibility = FunctionVisibility::Public;
        let config = test_config();

        let (is_api, indicators) = is_likely_external_api_with_config(&func, &visibility, &config);

        assert!(is_api);
        assert!(indicators.iter().any(|i| i.contains("lib.rs")));
    }

    #[test]
    fn test_constructor_pattern() {
        let func = create_test_function("new", "src/data/processor.rs");
        let visibility = FunctionVisibility::Public;
        let config = test_config();

        let (is_api, indicators) = is_likely_external_api_with_config(&func, &visibility, &config);

        assert!(is_api);
        assert!(indicators.iter().any(|i| i.contains("Constructor")));
    }

    #[test]
    fn test_deep_internal_module() {
        let func = create_test_function("helper", "src/internal/impl/detail/utils/helpers.rs");
        let visibility = FunctionVisibility::Public;
        let config = test_config();

        let (is_api, _indicators) = is_likely_external_api_with_config(&func, &visibility, &config);

        assert!(!is_api);
    }

    #[test]
    fn test_api_pattern_names() {
        let func = create_test_function("get_configuration", "src/config.rs");
        let visibility = FunctionVisibility::Public;
        let config = test_config();

        let (_is_api, indicators) = is_likely_external_api_with_config(&func, &visibility, &config);

        assert!(indicators.iter().any(|i| i.contains("API pattern")));
    }

    #[test]
    fn test_private_function() {
        let func = create_test_function("internal_helper", "src/utils.rs");
        let visibility = FunctionVisibility::Private;
        let config = test_config();

        let (is_api, indicators) = is_likely_external_api_with_config(&func, &visibility, &config);

        assert!(!is_api);
        assert!(indicators.is_empty());
    }

    #[test]
    fn test_explicit_api_marking() {
        // Test that explicit marking detection works
        let config = ExternalApiConfig {
            detect_external_api: false,
            api_functions: vec!["parse".to_string(), "lib::connect".to_string()],
            api_files: vec!["src/api.rs".to_string(), "src/public/*.rs".to_string()],
        };

        // Test function name match
        let func1 = create_test_function("parse", "src/internal/parser.rs");
        assert!(is_explicitly_marked_api(&func1, &config));

        // Test module::function match
        let func2 = create_test_function("connect", "src/lib.rs");
        assert!(is_explicitly_marked_api(&func2, &config));

        // Test file match
        let func3 = create_test_function("anything", "src/api.rs");
        assert!(is_explicitly_marked_api(&func3, &config));

        // Test pattern match
        let func4 = create_test_function("something", "src/public/client.rs");
        assert!(is_explicitly_marked_api(&func4, &config));

        // Test non-match
        let func5 = create_test_function("helper", "src/internal/utils.rs");
        assert!(!is_explicitly_marked_api(&func5, &config));
    }

    #[test]
    fn test_classify_complexity_hints_low() {
        let mut func = create_test_function("simple_func", "src/lib.rs");
        func.cyclomatic = 2;
        func.cognitive = 3;

        let hint = classify_complexity_hints(&func);
        assert_eq!(
            hint,
            Some("Low complexity - low impact removal".to_string())
        );
    }

    #[test]
    fn test_classify_complexity_hints_high_cyclomatic() {
        let mut func = create_test_function("complex_func", "src/lib.rs");
        func.cyclomatic = 15;
        func.cognitive = 10;

        let hint = classify_complexity_hints(&func);
        assert_eq!(
            hint,
            Some("High complexity - removing may eliminate significant unused code".to_string())
        );
    }

    #[test]
    fn test_classify_complexity_hints_high_cognitive() {
        let mut func = create_test_function("cognitive_func", "src/lib.rs");
        func.cyclomatic = 8;
        func.cognitive = 20;

        let hint = classify_complexity_hints(&func);
        assert_eq!(
            hint,
            Some("High complexity - removing may eliminate significant unused code".to_string())
        );
    }

    #[test]
    fn test_classify_complexity_hints_medium() {
        let mut func = create_test_function("medium_func", "src/lib.rs");
        func.cyclomatic = 6;
        func.cognitive = 10;

        let hint = classify_complexity_hints(&func);
        assert_eq!(hint, None);
    }

    #[test]
    fn test_detect_test_helper_hints_test_helper() {
        let hint = detect_test_helper_hints("test_helper_function");
        assert_eq!(
            hint,
            Some("Potential test helper - consider moving to test module".to_string())
        );
    }

    #[test]
    fn test_detect_test_helper_hints_mock() {
        let hint = detect_test_helper_hints("create_mock_data");
        assert_eq!(
            hint,
            Some("Potential test helper - consider moving to test module".to_string())
        );
    }

    #[test]
    fn test_detect_test_helper_hints_fixture() {
        let hint = detect_test_helper_hints("setup_fixture");
        assert_eq!(
            hint,
            Some("Potential test helper - consider moving to test module".to_string())
        );
    }

    #[test]
    fn test_detect_test_helper_hints_general_helper() {
        let hint = detect_test_helper_hints("helper_function");
        assert_eq!(
            hint,
            Some("Potential test helper - consider moving to test module".to_string())
        );
    }

    #[test]
    fn test_detect_test_helper_hints_util() {
        let hint = detect_test_helper_hints("util_process");
        assert_eq!(
            hint,
            Some("Potential test helper - consider moving to test module".to_string())
        );
    }

    #[test]
    fn test_detect_test_helper_hints_regular_function() {
        let hint = detect_test_helper_hints("process_data");
        assert_eq!(hint, None);
    }

    #[test]
    fn test_detect_test_helper_hints_test_without_helper() {
        // "test" alone without "helper" shouldn't match
        let hint = detect_test_helper_hints("test_something");
        assert_eq!(hint, None);
    }
}
