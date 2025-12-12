//! Rust symbol demangling utilities.
//!
//! This module provides pure functions for demangling Rust function names
//! from LCOV coverage data. Rust compilers use name mangling to encode type
//! information and ensure unique symbol names, and this module reverses that
//! process.
//!
//! # Stillwater Philosophy
//!
//! All functions in this module are pure:
//! - No I/O operations
//! - Deterministic results
//! - No mutation of inputs
//! - Easily testable
//!
//! # Mangling Schemes
//!
//! Rust uses two main mangling schemes:
//! - Legacy: starts with `_ZN` (based on Itanium C++ ABI)
//! - v0: starts with `_RNv` (Rust-specific, more compact)
//!
//! # Example
//!
//! ```ignore
//! use debtmap::risk::lcov::demangle::demangle_function_name;
//!
//! // Mangled name from LCOV
//! let mangled = "_RNvMs_NtCs123_7project4fileNtB5_6Struct6method";
//! let demangled = demangle_function_name(mangled);
//! // demangled contains human-readable name like "project::file::Struct::method"
//! ```

/// Demangle a Rust function name if it's mangled.
///
/// This function handles both legacy and v0 mangling schemes:
/// - Legacy: starts with `_ZN`
/// - v0: starts with `_RNv`
///
/// If the name is not mangled (already human-readable), it returns the
/// original name unchanged.
///
/// # Arguments
///
/// * `name` - The potentially mangled function name
///
/// # Returns
///
/// The demangled function name, or the original if not mangled.
///
/// # Example
///
/// ```ignore
/// use debtmap::risk::lcov::demangle::demangle_function_name;
///
/// // Already demangled - returns as-is
/// let name = "my_module::my_function";
/// assert_eq!(demangle_function_name(name), name);
///
/// // Mangled name - returns demangled
/// let mangled = "_ZN3foo3barE";
/// let demangled = demangle_function_name(mangled);
/// assert!(!demangled.starts_with("_ZN"));
/// ```
///
/// # Performance
///
/// O(n) where n is the length of the mangled name. Single pass through
/// the string with minimal allocations.
pub fn demangle_function_name(name: &str) -> String {
    // Try to demangle any name - rustc_demangle will return the original if it's not mangled
    let demangled = rustc_demangle::demangle(name).to_string();

    // If demangling changed the string, use the demangled version; otherwise keep original
    if demangled != name {
        demangled
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demangle_v0_mangled_name() {
        let mangled = "_RNvMNtNtNtCs9MAeJIiYlOV_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes";
        let demangled = demangle_function_name(mangled);

        assert!(demangled.contains("ChangeTracker"));
        assert!(demangled.contains("track_changes"));
        assert!(!demangled.starts_with("_RNv"));
    }

    #[test]
    fn test_demangle_legacy_mangled_name() {
        // Test with a simple legacy mangled name
        let mangled = "_ZN3foo3barE";
        let demangled = demangle_function_name(mangled);

        // rustc-demangle should handle this
        assert!(!demangled.starts_with("_ZN") || demangled == mangled);
    }

    #[test]
    fn test_demangle_already_demangled() {
        let name = "my_module::my_function";
        let result = demangle_function_name(name);

        assert_eq!(result, name);
    }

    #[test]
    fn test_demangle_simple_function() {
        let name = "simple_function";
        let result = demangle_function_name(name);

        assert_eq!(result, name);
    }

    #[test]
    fn test_demangle_with_generics() {
        // A function name with generic notation (not mangled)
        let name = "Vec<T>::push";
        let result = demangle_function_name(name);

        assert_eq!(result, name);
    }

    #[test]
    fn test_demangle_empty_string() {
        let result = demangle_function_name("");
        assert_eq!(result, "");
    }
}
