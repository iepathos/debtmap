//! Testing Framework Pattern Detection

use super::detector::{FileContext, FunctionAst};

/// Detect pytest fixture (Python)
pub fn is_pytest_fixture(function: &FunctionAst) -> bool {
    function
        .decorators
        .iter()
        .any(|d| d.name.contains("pytest.fixture") || d.name == "fixture" || d.name == "@fixture")
}

/// Detect Rust test function
pub fn is_rust_test(function: &FunctionAst) -> bool {
    function.attributes.iter().any(|attr| {
        let attr_str = attr.to_string();
        attr_str.contains("#[test]")
            || attr_str.contains("#[tokio::test]")
            || attr_str.contains("#[cfg(test)]")
    }) || function.name.starts_with("test_")
}

/// Detect Jest test function (JavaScript/TypeScript)
pub fn is_jest_test(function: &FunctionAst, file_context: &FileContext) -> bool {
    // Check if file is a test file
    let path_str = file_context.path.to_string_lossy();
    let is_test_file = path_str.contains(".test.") || path_str.contains(".spec.");

    // Check if function calls test/it/describe
    let has_test_calls = function
        .calls
        .iter()
        .any(|call| call.name == "test" || call.name == "it" || call.name == "describe");

    is_test_file || has_test_calls
}

#[cfg(test)]
mod tests {
    use super::super::detector::{Attribute, Decorator, FunctionAst};
    use super::*;

    #[test]
    fn test_pytest_fixture_detection() {
        let mut function = FunctionAst::new("database".to_string());
        function.decorators.push(Decorator {
            name: "@pytest.fixture".to_string(),
        });

        assert!(is_pytest_fixture(&function));
    }

    #[test]
    fn test_rust_test_detection() {
        let mut function = FunctionAst::new("test_addition".to_string());
        function.attributes.push(Attribute {
            text: "#[test]".to_string(),
        });

        assert!(is_rust_test(&function));
    }
}
