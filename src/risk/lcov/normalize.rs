//! Function name normalization for LCOV matching.
//!
//! This module provides pure functions for normalizing Rust function names
//! to enable matching between AST-derived names and LCOV coverage data.
//!
//! # Stillwater Philosophy
//!
//! All functions in this module are pure:
//! - No I/O operations
//! - Deterministic results
//! - No mutation of inputs
//! - Easily testable
//!
//! # Normalization Process
//!
//! Function names from LCOV data often contain:
//! - Generic type parameters: `HashMap<K,V>::insert`
//! - Crate hashes: `<crate[71f4b4990cdcf1ab]::Type>::method`
//! - Impl block wrappers: `<Type>::method`
//! - Trailing generics: `method::<T>`
//!
//! This module strips these to produce matchable names.
//!
//! # Example
//!
//! ```ignore
//! use debtmap::risk::lcov::normalize::normalize_demangled_name;
//!
//! let result = normalize_demangled_name("HashMap<K,V>::insert");
//! assert_eq!(result.full_path, "HashMap::insert");
//! assert_eq!(result.method_name, "insert");
//! ```

use super::types::NormalizedFunctionName;
use std::borrow::Cow;

/// Strip trailing generic parameters from function names.
///
/// Handles nested generics like `method::<Vec<HashMap<K, V>>>`.
/// Returns `Cow` to avoid allocation when no stripping is needed.
///
/// This is a pure function with no side effects - it transforms strings
/// deterministically.
///
/// # Arguments
///
/// * `s` - The function name potentially with trailing generics
///
/// # Returns
///
/// A `Cow<str>` containing the name with trailing generics stripped.
/// Returns `Cow::Borrowed` if no stripping was needed (no allocation).
///
/// # Examples
///
/// ```ignore
/// use std::borrow::Cow;
/// use debtmap::risk::lcov::normalize::strip_trailing_generics;
///
/// assert_eq!(strip_trailing_generics("Type::method::<T>"), Cow::Borrowed("Type::method"));
/// assert_eq!(strip_trailing_generics("method::<Vec<T>>"), Cow::Borrowed("method"));
/// assert_eq!(strip_trailing_generics("method"), Cow::Borrowed("method"));
/// ```
///
/// # Performance
///
/// O(n) where n is the string length. Only allocates if stripping is needed.
pub fn strip_trailing_generics(s: &str) -> Cow<'_, str> {
    if let Some(pos) = s.rfind("::<") {
        // Count angle brackets to find matching close (handles nested generics)
        let mut depth = 0;
        let mut end_pos = None;

        for (i, ch) in s[pos + 3..].char_indices() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    if depth == 0 {
                        end_pos = Some(pos + 3 + i);
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }

        if let Some(end) = end_pos {
            let after = &s[end + 1..];
            // If nothing after the >, this is a trailing generic
            if after.is_empty() {
                return Cow::Owned(s[..pos].to_string());
            }
        }
    }
    Cow::Borrowed(s)
}

/// Normalize a demangled function name for consolidation.
///
/// Removes generic type parameters and crate hash IDs to
/// group multiple monomorphizations of the same function.
/// Also extracts the method name for flexible matching.
///
/// # Transformations Applied
///
/// 1. Remove impl block angle brackets: `<Type>::method` -> `Type::method`
/// 2. Remove crate hash IDs: `<crate[hash]::Type>::method` -> `crate::Type::method`
/// 3. Strip trailing generics: `method::<T>` -> `method`
/// 4. Remove type parameters: `HashMap<K,V>::insert` -> `HashMap::insert`
///
/// # Arguments
///
/// * `demangled` - The demangled function name to normalize
///
/// # Returns
///
/// A `NormalizedFunctionName` with:
/// - `full_path`: The fully normalized path
/// - `method_name`: Just the final method name segment
/// - `original`: The original input for debugging
///
/// # Examples
///
/// ```ignore
/// use debtmap::risk::lcov::normalize::normalize_demangled_name;
///
/// let result = normalize_demangled_name("<debtmap[71f4b4990cdcf1ab]::Foo>::bar");
/// assert_eq!(result.full_path, "debtmap::Foo::bar");
/// assert_eq!(result.method_name, "bar");
///
/// let result = normalize_demangled_name("std::collections::HashMap<K,V>::insert");
/// assert_eq!(result.full_path, "std::collections::HashMap::insert");
/// assert_eq!(result.method_name, "insert");
///
/// let result = normalize_demangled_name("<Struct as Trait>::method");
/// assert_eq!(result.method_name, "method");
///
/// let result = normalize_demangled_name("Type::method::<T>");
/// assert_eq!(result.full_path, "Type::method");
/// assert_eq!(result.method_name, "method");
/// ```
///
/// # Performance
///
/// O(n) where n is the string length. Multiple passes but linear overall.
pub fn normalize_demangled_name(demangled: &str) -> NormalizedFunctionName {
    // Handle impl method patterns: <module::path::Type>::method -> module::path::Type::method
    // Remove angle brackets and crate hash: <crate[hash]::rest>::method -> crate::rest::method
    let without_impl_brackets = if demangled.starts_with('<') {
        if let Some(angle_end) = demangled.find('>') {
            let content = &demangled[1..angle_end];
            let after = &demangled[(angle_end + 1)..];

            // Remove hash from path if present: crate[hash]::rest -> crate::rest
            let content_without_hash = if let Some(bracket_start) = content.find('[') {
                if let Some(bracket_end) = content.find(']') {
                    // Reconstruct: before[hash]after -> beforeafter
                    format!(
                        "{}{}",
                        &content[..bracket_start],
                        &content[(bracket_end + 1)..]
                    )
                } else {
                    content.to_string()
                }
            } else {
                content.to_string()
            };

            format!("{}{}", content_without_hash, after)
        } else {
            demangled.to_string()
        }
    } else {
        demangled.to_string()
    };

    // Strip trailing generic parameters from functions (e.g., "method::<T>" -> "method")
    let without_function_generics = strip_trailing_generics(&without_impl_brackets);

    // Now remove generic type parameters: "HashMap<K,V>::insert" -> "HashMap::insert"
    // Use fold to track depth and only keep characters outside angle brackets
    let result = without_function_generics
        .chars()
        .fold((String::new(), 0usize), |(mut acc, depth), ch| match ch {
            '<' => (acc, depth + 1),
            '>' if depth > 0 => (acc, depth - 1),
            _ if depth == 0 => {
                acc.push(ch);
                (acc, depth)
            }
            _ => (acc, depth),
        })
        .0;

    // Extract method name (final segment after last ::)
    let method_name = result.rsplit("::").next().unwrap_or(&result).to_string();

    NormalizedFunctionName {
        full_path: result,
        method_name,
        original: demangled.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_trailing_generics_simple() {
        assert_eq!(
            strip_trailing_generics("Type::method::<WorkflowExecutor>"),
            Cow::Borrowed("Type::method")
        );
        assert_eq!(
            strip_trailing_generics("crate::Type::method::<T>"),
            Cow::Borrowed("crate::Type::method")
        );
        assert_eq!(
            strip_trailing_generics("Type::method"), // No generics
            Cow::Borrowed("Type::method")
        );
    }

    #[test]
    fn test_strip_trailing_generics_nested() {
        // Nested generics
        assert_eq!(
            strip_trailing_generics("method::<Vec<HashMap<K, V>>>"),
            Cow::Borrowed("method")
        );
        // Multiple type parameters
        assert_eq!(
            strip_trailing_generics("method::<T, U, V>"),
            Cow::Borrowed("method")
        );
        // Complex nested case
        assert_eq!(
            strip_trailing_generics("Type::method::<Result<Vec<T>, Error>>"),
            Cow::Borrowed("Type::method")
        );
    }

    #[test]
    fn test_normalize_removes_generics() {
        let result = normalize_demangled_name("HashMap<String, i32>::insert");
        assert_eq!(result.full_path, "HashMap::insert");
        assert_eq!(result.method_name, "insert");

        let result = normalize_demangled_name("Vec<T>::push");
        assert_eq!(result.full_path, "Vec::push");
        assert_eq!(result.method_name, "push");

        let result = normalize_demangled_name("simple_function");
        assert_eq!(result.full_path, "simple_function");
        assert_eq!(result.method_name, "simple_function");
    }

    #[test]
    fn test_normalize_preserves_module_path() {
        let result = normalize_demangled_name("std::collections::HashMap<K,V>::insert");
        assert_eq!(result.full_path, "std::collections::HashMap::insert");
        assert_eq!(result.method_name, "insert");
    }

    #[test]
    fn test_normalize_removes_crate_hash() {
        let result = normalize_demangled_name("<debtmap[71f4b4990cdcf1ab]::Foo>::bar");
        assert_eq!(result.full_path, "debtmap::Foo::bar");
        assert_eq!(result.method_name, "bar");
    }

    #[test]
    fn test_normalize_extracts_method_name() {
        let result = normalize_demangled_name("prodigy::cook::CommitTracker::create_auto_commit");
        assert_eq!(
            result.full_path,
            "prodigy::cook::CommitTracker::create_auto_commit"
        );
        assert_eq!(result.method_name, "create_auto_commit");

        let result = normalize_demangled_name("<Foo as Bar>::method");
        assert_eq!(result.method_name, "method");
    }

    #[test]
    fn test_normalize_strips_trailing_generics() {
        // Test that normalize_demangled_name now strips trailing generics
        let result = normalize_demangled_name("Type::method::<WorkflowExecutor>");
        assert_eq!(result.full_path, "Type::method");
        assert_eq!(result.method_name, "method");

        let result = normalize_demangled_name("SetupPhaseExecutor::execute::<T>");
        assert_eq!(result.full_path, "SetupPhaseExecutor::execute");
        assert_eq!(result.method_name, "execute");
    }

    #[test]
    fn test_normalize_impl_method_with_angle_brackets() {
        // This is the actual pattern from LCOV that fails to match
        let demangled =
            "<prodigy::cook::workflow::resume::ResumeExecutor>::execute_remaining_steps";
        let result = normalize_demangled_name(demangled);

        // After normalization, should be usable for matching
        assert_eq!(result.method_name, "execute_remaining_steps");

        // The full_path should preserve the structure but be matchable
        assert!(
            result.full_path.contains("ResumeExecutor"),
            "full_path should contain ResumeExecutor, got: {}",
            result.full_path
        );
        assert!(
            result.full_path.contains("execute_remaining_steps"),
            "full_path should contain method name, got: {}",
            result.full_path
        );
    }

    #[test]
    fn test_multiple_impl_methods() {
        // Test multiple common patterns that appear in real codebases
        let test_cases = vec![
            ("<Type>::method", "method", "Simple impl method"),
            (
                "<crate::module::Type>::method",
                "method",
                "Fully qualified impl method",
            ),
            (
                "<impl Trait for Type>::method",
                "method",
                "Trait impl method",
            ),
            (
                "<Type<T>>::generic_method",
                "generic_method",
                "Generic impl method",
            ),
        ];

        for (input, expected_method, description) in test_cases {
            let result = normalize_demangled_name(input);
            assert_eq!(
                result.method_name, expected_method,
                "{}: method_name mismatch for '{}'",
                description, input
            );
        }
    }

    #[test]
    fn test_normalize_preserves_impl_for_matching() {
        // The normalization should make impl methods matchable
        let demangled = "<foo::bar::Baz>::do_something";
        let normalized = normalize_demangled_name(demangled);

        // Should be able to match against any of these forms:
        // 1. Just the method name: "do_something"
        // 2. Type::method: "Baz::do_something"
        // 3. Full path: "foo::bar::Baz::do_something"

        assert_eq!(normalized.method_name, "do_something");

        // The full_path should be matchable (without angle brackets causing issues)
        assert!(
            !normalized.full_path.starts_with('<'),
            "full_path should not start with angle bracket, got: {}",
            normalized.full_path
        );
    }
}
