use std::path::Path;

/// Detect if code is test-related based on path and function name
pub fn is_test_code(path: &Path, function_name: &str) -> bool {
    // Check if in test module
    if path.components().any(|c| c.as_os_str() == "tests") {
        return true;
    }

    // Check if test file
    if path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.ends_with("_test") || s.starts_with("test_"))
        .unwrap_or(false)
    {
        return true;
    }

    // Check if test function
    function_name.starts_with("test_") || function_name.contains("_test") || function_name == "test"
}

/// Check if a file is a test file based on its path
pub fn is_test_file(path: &Path) -> bool {
    // Check if in tests directory
    if path.components().any(|c| c.as_os_str() == "tests") {
        return true;
    }

    // Check file name patterns
    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
        return file_stem.ends_with("_test")
            || file_stem.starts_with("test_")
            || file_stem.ends_with("_tests")
            || file_stem == "test"
            || file_stem == "tests";
    }

    false
}

/// Check if a function name indicates a test function
pub fn is_test_function(function_name: &str) -> bool {
    function_name.starts_with("test_")
        || function_name.starts_with("tests_")
        || function_name.ends_with("_test")
        || function_name.ends_with("_tests")
        || function_name.contains("_test_")
        || function_name == "test"
        || function_name == "tests"
        // Common test framework patterns
        || function_name.starts_with("it_")
        || function_name.starts_with("should_")
        || function_name.starts_with("assert_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_test_code_in_tests_directory() {
        let path = PathBuf::from("src/tests/my_module.rs");
        assert!(is_test_code(&path, "some_function"));

        let path = PathBuf::from("tests/integration.rs");
        assert!(is_test_code(&path, "helper"));
    }

    #[test]
    fn test_is_test_code_test_file() {
        let path = PathBuf::from("src/my_module_test.rs");
        assert!(is_test_code(&path, "helper"));

        let path = PathBuf::from("src/test_utils.rs");
        assert!(is_test_code(&path, "setup"));
    }

    #[test]
    fn test_is_test_code_test_function() {
        let path = PathBuf::from("src/lib.rs");
        assert!(is_test_code(&path, "test_something"));
        assert!(is_test_code(&path, "something_test"));
        assert!(is_test_code(&path, "test"));
        assert!(!is_test_code(&path, "regular_function"));
    }

    #[test]
    fn test_is_test_file() {
        assert!(is_test_file(&PathBuf::from("tests/foo.rs")));
        assert!(is_test_file(&PathBuf::from("src/tests/bar.rs")));
        assert!(is_test_file(&PathBuf::from("foo_test.rs")));
        assert!(is_test_file(&PathBuf::from("test_foo.rs")));
        assert!(is_test_file(&PathBuf::from("foo_tests.rs")));
        assert!(!is_test_file(&PathBuf::from("src/main.rs")));
        assert!(!is_test_file(&PathBuf::from("lib.rs")));
    }

    #[test]
    fn test_is_test_function() {
        // Standard test patterns
        assert!(is_test_function("test_foo"));
        assert!(is_test_function("tests_bar"));
        assert!(is_test_function("foo_test"));
        assert!(is_test_function("bar_tests"));
        assert!(is_test_function("foo_test_bar"));
        assert!(is_test_function("test"));
        assert!(is_test_function("tests"));

        // Test framework patterns
        assert!(is_test_function("it_should_work"));
        assert!(is_test_function("should_handle_error"));
        assert!(is_test_function("assert_equals"));

        // Non-test functions
        assert!(!is_test_function("regular_function"));
        assert!(!is_test_function("testing_helper"));
        assert!(!is_test_function("attestation"));
    }
}
