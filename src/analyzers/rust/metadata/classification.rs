//! Function role classification
//!
//! Pure functions for classifying function roles.

use crate::complexity::threshold_manager::FunctionRole;

/// Classify the role of a function based on name and test status
pub fn classify_function_role(name: &str, is_test: bool) -> FunctionRole {
    match () {
        _ if is_test => FunctionRole::Test,
        _ if name == "main" => FunctionRole::EntryPoint,
        _ => FunctionRole::CoreLogic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_test_function() {
        assert_eq!(classify_function_role("test_foo", true), FunctionRole::Test);
    }

    #[test]
    fn test_classify_main() {
        assert_eq!(
            classify_function_role("main", false),
            FunctionRole::EntryPoint
        );
    }

    #[test]
    fn test_classify_regular() {
        assert_eq!(
            classify_function_role("process_data", false),
            FunctionRole::CoreLogic
        );
    }
}
