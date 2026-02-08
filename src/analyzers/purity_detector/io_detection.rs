//! I/O and mutation detection
//!
//! Pure functions for detecting I/O operations and mutation methods.

use super::constants::{IO_PATTERNS, MUTATION_METHODS};

/// Check if a path represents an I/O operation
pub fn is_io_call(path_str: &str) -> bool {
    IO_PATTERNS.iter().any(|pattern| path_str.contains(pattern))
}

/// Check if a method name is a mutation method
pub fn is_mutation_method(method_name: &str) -> bool {
    MUTATION_METHODS.contains(&method_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_io_call() {
        assert!(is_io_call("println"));
        assert!(is_io_call("std::fs::read"));
        assert!(is_io_call("tokio::spawn"));
        assert!(!is_io_call("add"));
        assert!(!is_io_call("calculate"));
    }

    #[test]
    fn test_is_mutation_method() {
        assert!(is_mutation_method("push"));
        assert!(is_mutation_method("pop"));
        assert!(is_mutation_method("insert"));
        assert!(!is_mutation_method("len"));
        assert!(!is_mutation_method("iter"));
    }
}
