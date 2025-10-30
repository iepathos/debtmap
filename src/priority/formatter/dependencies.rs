//! Dependency filtering and formatting utilities
//!
//! This module provides functions for filtering and formatting function dependencies
//! (callers and callees) based on configuration settings.

use crate::config::CallerCalleeConfig;

/// Pure function to determine if a function reference should be included in output
pub(crate) fn should_include_in_output(function_name: &str, config: &CallerCalleeConfig) -> bool {
    // Check for standard library patterns
    if !config.show_std_lib && is_standard_library_call(function_name) {
        return false;
    }

    // Check for external crate patterns (functions with :: that aren't std)
    if !config.show_external && is_external_crate_call(function_name) {
        return false;
    }

    true
}

/// Pure function to check if a call is to the standard library
fn is_standard_library_call(function_name: &str) -> bool {
    function_name.starts_with("std::")
        || function_name.starts_with("core::")
        || function_name.starts_with("alloc::")
        || function_name == "println"
        || function_name == "print"
        || function_name == "eprintln"
        || function_name == "eprint"
        || function_name == "write"
        || function_name == "writeln"
        || function_name == "format"
        || function_name == "panic"
        || function_name == "assert"
        || function_name == "debug_assert"
}

/// Pure function to check if a call is to an external crate
fn is_external_crate_call(function_name: &str) -> bool {
    // Has :: but not std/core/alloc
    function_name.contains("::")
        && !function_name.starts_with("std::")
        && !function_name.starts_with("core::")
        && !function_name.starts_with("alloc::")
        && !function_name.starts_with("crate::")
}

/// Pure function to filter a list of dependencies based on configuration
pub(crate) fn filter_dependencies(names: &[String], config: &CallerCalleeConfig) -> Vec<String> {
    names
        .iter()
        .filter(|name| should_include_in_output(name, config))
        .cloned()
        .collect()
}

/// Pure function to format a function reference for display
pub(crate) fn format_function_reference(function_name: &str) -> String {
    // Simplify long paths - show just the last component with file hint
    if function_name.contains("::") {
        let parts: Vec<&str> = function_name.split("::").collect();
        if parts.len() > 2 {
            format!("{}::{}", parts[parts.len() - 2], parts[parts.len() - 1])
        } else {
            function_name.to_string()
        }
    } else {
        function_name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_dependencies() {
        let config = CallerCalleeConfig {
            max_callers: 5,
            max_callees: 5,
            show_std_lib: false,
            show_external: false,
        };

        let names = vec!["my_func".to_string(), "std::vec::Vec".to_string()];
        let filtered = filter_dependencies(&names, &config);

        assert_eq!(filtered, vec!["my_func"]);
    }

    #[test]
    fn test_format_function_reference() {
        // Short names are unchanged
        assert_eq!(format_function_reference("my_function"), "my_function");

        // Two-segment paths are unchanged
        assert_eq!(format_function_reference("crate::helper"), "crate::helper");

        // Long paths are simplified to last two segments
        assert_eq!(
            format_function_reference("crate::utils::io::helper::read_file"),
            "helper::read_file"
        );

        assert_eq!(
            format_function_reference("std::collections::HashMap"),
            "collections::HashMap"
        );
    }
}
