//! Test detection for Rust functions
//!
//! Pure functions for detecting test functions and test files.

use quote::ToTokens;

/// Pure function to classify if a path represents a test file
pub fn classify_test_file(path_str: &str) -> bool {
    // Test directory patterns
    const TEST_DIR_PATTERNS: &[&str] = &[
        "/tests/",
        "/test/",
        "/testing/",
        "/mocks/",
        "/mock/",
        "/fixtures/",
        "/fixture/",
        "/test_helpers/",
        "/test_utils/",
        "/test_",
        "/mock",
        "/scenario",
        "\\tests\\",
        "\\test\\", // Windows paths
    ];

    // Test file suffixes
    const TEST_FILE_SUFFIXES: &[&str] = &["_test.rs", "_tests.rs", "/tests.rs", "/test.rs"];

    // Check directory patterns
    let has_test_dir = TEST_DIR_PATTERNS
        .iter()
        .any(|pattern| path_str.contains(pattern));

    // Check file suffixes
    let has_test_suffix = TEST_FILE_SUFFIXES
        .iter()
        .any(|suffix| path_str.ends_with(suffix));

    has_test_dir || has_test_suffix
}

/// Check if a function is a test function based on name and attributes
pub fn is_test_function(name: &str, item_fn: &syn::ItemFn) -> bool {
    has_test_attribute(&item_fn.attrs) || has_test_name_pattern(name)
}

/// Check if function has test-related attributes
pub fn has_test_attribute(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| match () {
        _ if attr.path().is_ident("test") => true,
        _ if attr
            .path()
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "test") =>
        {
            true
        }
        _ if attr.path().is_ident("cfg") => {
            attr.meta.to_token_stream().to_string().contains("test")
        }
        _ => false,
    })
}

/// Check if function name matches test naming patterns
pub fn has_test_name_pattern(name: &str) -> bool {
    const TEST_PREFIXES: &[&str] = &["test_", "it_", "should_"];
    const MOCK_PATTERNS: &[&str] = &["mock", "stub", "fake"];

    let name_lower = name.to_lowercase();

    match () {
        _ if TEST_PREFIXES.iter().any(|prefix| name.starts_with(prefix)) => true,
        _ if MOCK_PATTERNS
            .iter()
            .any(|pattern| name_lower.contains(pattern)) =>
        {
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_test_file_with_test_directories() {
        assert!(classify_test_file("src/tests/mod.rs"));
        assert!(classify_test_file("src/test/utils.rs"));
        assert!(classify_test_file("src/testing/helpers.rs"));
        assert!(classify_test_file("src/mocks/data.rs"));
        assert!(classify_test_file("src/mock/server.rs"));
        assert!(classify_test_file("src/fixtures/sample.rs"));
        assert!(classify_test_file("src/fixture/db.rs"));
        assert!(classify_test_file("src/test_helpers/common.rs"));
        assert!(classify_test_file("src/test_utils/setup.rs"));
        assert!(classify_test_file("src/test_integration.rs"));
        assert!(classify_test_file("src/mockito/client.rs"));
        assert!(classify_test_file("src/scenario/basic.rs"));
    }

    #[test]
    fn test_classify_test_file_with_test_suffixes() {
        assert!(classify_test_file("src/lib_test.rs"));
        assert!(classify_test_file("src/module_tests.rs"));
        assert!(classify_test_file("src/tests.rs"));
        assert!(classify_test_file("src/test.rs"));
        assert!(classify_test_file("integration_test.rs"));
        assert!(classify_test_file("unit_tests.rs"));
    }

    #[test]
    fn test_classify_test_file_with_windows_paths() {
        assert!(classify_test_file("src\\tests\\mod.rs"));
    }

    #[test]
    fn test_classify_test_file_non_test_files() {
        assert!(!classify_test_file("src/main.rs"));
        assert!(!classify_test_file("src/lib.rs"));
        assert!(!classify_test_file("src/core/module.rs"));
    }

    #[test]
    fn test_classify_test_file_negative_cases() {
        assert!(!classify_test_file("src/main.rs"));
        assert!(!classify_test_file("src/lib.rs"));
        assert!(!classify_test_file("src/analyzer.rs"));
        assert!(!classify_test_file("src/core/processor.rs"));
        assert!(!classify_test_file("src/utils/helper.rs"));
        assert!(!classify_test_file("src/latest.rs"));
        assert!(!classify_test_file("src/contest.rs"));
        assert!(!classify_test_file("src/protest.rs"));
    }

    #[test]
    fn test_classify_test_file_edge_cases() {
        assert!(classify_test_file("/tests/"));
        assert!(classify_test_file("/tests/file.rs"));
        assert!(classify_test_file("path/test.rs"));
        assert!(classify_test_file("path/tests.rs"));
        assert!(!classify_test_file(""));
        assert!(!classify_test_file("/"));
        assert!(classify_test_file("deeply/nested/tests/file.rs"));
        assert!(classify_test_file("very/deep/path/test_utils/util.rs"));
    }

    #[test]
    fn test_has_test_name_pattern_with_test_prefix() {
        assert!(has_test_name_pattern("test_something"));
        assert!(has_test_name_pattern("test_"));
    }

    #[test]
    fn test_has_test_name_pattern_with_it_prefix() {
        assert!(has_test_name_pattern("it_should_work"));
        assert!(has_test_name_pattern("it_"));
    }

    #[test]
    fn test_has_test_name_pattern_with_should_prefix() {
        assert!(has_test_name_pattern("should_do_something"));
        assert!(has_test_name_pattern("should_"));
    }

    #[test]
    fn test_has_test_name_pattern_with_mock() {
        assert!(has_test_name_pattern("mock_service"));
        assert!(has_test_name_pattern("get_mock"));
        assert!(has_test_name_pattern("MockBuilder"));
    }

    #[test]
    fn test_has_test_name_pattern_with_stub() {
        assert!(has_test_name_pattern("stub_response"));
        assert!(has_test_name_pattern("get_stub"));
        assert!(has_test_name_pattern("StubFactory"));
    }

    #[test]
    fn test_has_test_name_pattern_with_fake() {
        assert!(has_test_name_pattern("fake_data"));
        assert!(has_test_name_pattern("create_fake"));
        assert!(has_test_name_pattern("FakeImpl"));
    }

    #[test]
    fn test_has_test_name_pattern_regular_name() {
        assert!(!has_test_name_pattern("regular_function"));
        assert!(!has_test_name_pattern("process_data"));
        assert!(!has_test_name_pattern("handle_request"));
    }
}
