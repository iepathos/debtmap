//! Property-based testing generators for debtmap types.
//!
//! This module provides proptest strategies for generating random test data,
//! enabling property-based testing of pure functions.
//!
//! # Why Property-Based Testing?
//!
//! Traditional example-based tests check specific inputs. Property-based tests
//! verify that invariants hold across many randomly generated inputs:
//!
//! - Finds edge cases you wouldn't think to test
//! - Provides automatic shrinking to minimal failing cases
//! - Documents invariants as executable specifications
//!
//! # Available Strategies
//!
//! | Strategy | Generates | Use Case |
//! |----------|-----------|----------|
//! | [`any_config`] | `DebtmapConfig` | Testing config-dependent code |
//! | [`any_thresholds`] | `ThresholdsConfig` | Testing threshold logic |
//! | [`any_coverage`] | `FileCoverage` | Testing coverage calculations |
//! | [`any_complexity`] | `u32` | Testing complexity logic |
//! | [`any_ast`] | `syn::File` | Testing AST analysis |
//!
//! # Example: Testing Pure Functions
//!
//! ```rust,ignore
//! use proptest::prelude::*;
//! use debtmap::testkit::proptest_generators::{any_config, any_complexity};
//!
//! proptest! {
//!     #[test]
//!     fn complexity_is_positive(complexity in any_complexity()) {
//!         // Property: complexity is always positive
//!         prop_assert!(complexity > 0);
//!     }
//!
//!     #[test]
//!     fn score_is_bounded(
//!         complexity in any_complexity(),
//!         config in any_config(),
//!     ) {
//!         let score = calculate_score(complexity, &config);
//!
//!         // Property: score is always in valid range
//!         prop_assert!(score >= 0.0);
//!         prop_assert!(score <= 100.0);
//!     }
//! }
//! ```
//!
//! # Shrinking
//!
//! When a test fails, proptest automatically shrinks the failing input
//! to the minimal case that still fails, making debugging easier.
//!
//! # Deterministic Reproduction
//!
//! Failed tests print a seed that can be used to reproduce the failure:
//!
//! ```text
//! proptest: seed = 0x1234567890abcdef
//! ```
//!
//! Use `PROPTEST_SEED=0x1234567890abcdef cargo test` to reproduce.

use proptest::prelude::*;

use crate::config::{DebtmapConfig, ThresholdsConfig};
use crate::io::traits::FileCoverage;

/// Generate random complexity values (1-100).
///
/// Complexity of 0 is invalid (minimum is 1 for any function).
///
/// # Example
///
/// ```rust,ignore
/// proptest! {
///     #[test]
///     fn test_complexity(complexity in any_complexity()) {
///         assert!(complexity >= 1);
///     }
/// }
/// ```
pub fn any_complexity() -> impl Strategy<Value = u32> {
    1u32..=100
}

/// Generate random threshold configurations.
///
/// All thresholds are in reasonable ranges:
/// - Complexity: 1-100
/// - Max file length: 100-2000
/// - Max function length: 10-500
/// - Minimum debt score: 0.0-10.0
///
/// # Example
///
/// ```rust,ignore
/// proptest! {
///     #[test]
///     fn thresholds_are_valid(thresholds in any_thresholds()) {
///         // All thresholds should be in valid ranges
///     }
/// }
/// ```
pub fn any_thresholds() -> impl Strategy<Value = ThresholdsConfig> {
    (
        prop::option::of(1u32..=100),
        prop::option::of(100usize..=2000),
        prop::option::of(10usize..=500),
        prop::option::of(0.0f64..=10.0),
    )
        .prop_map(
            |(complexity, max_file_length, max_function_length, minimum_debt_score)| {
                ThresholdsConfig {
                    complexity,
                    max_file_length,
                    max_function_length,
                    minimum_debt_score,
                    ..Default::default()
                }
            },
        )
}

/// Generate random configurations.
///
/// Creates a `DebtmapConfig` with random thresholds.
/// Other fields are left as defaults.
///
/// # Example
///
/// ```rust,ignore
/// proptest! {
///     #[test]
///     fn config_is_usable(config in any_config()) {
///         // Config should be valid for analysis
///     }
/// }
/// ```
pub fn any_config() -> impl Strategy<Value = DebtmapConfig> {
    any_thresholds().prop_map(|thresholds| DebtmapConfig {
        thresholds: Some(thresholds),
        ..Default::default()
    })
}

/// Generate random file coverage data.
///
/// Generates coverage with:
/// - 1-100 total lines
/// - 0-total_lines hit lines
///
/// Always maintains the invariant that hit_lines <= total_lines.
///
/// # Example
///
/// ```rust,ignore
/// proptest! {
///     #[test]
///     fn coverage_is_valid(coverage in any_coverage()) {
///         // hit_lines should never exceed total_lines
///         assert!(coverage.hit_lines <= coverage.total_lines);
///     }
/// }
/// ```
pub fn any_coverage() -> impl Strategy<Value = FileCoverage> {
    (1usize..=100, 0usize..=100)
        .prop_filter_map("hits <= lines", |(lines, hits)| {
            if hits <= lines {
                Some((lines, hits))
            } else {
                Some((lines, lines)) // Clamp hits to lines
            }
        })
        .prop_map(|(lines, hits)| {
            let mut fc = FileCoverage::new();
            for i in 1..=lines {
                fc.add_line(i, if i <= hits { 1 } else { 0 });
            }
            fc
        })
}

/// Generate random coverage percentages (0.0-100.0).
///
/// # Example
///
/// ```rust,ignore
/// proptest! {
///     #[test]
///     fn percentage_is_bounded(pct in any_coverage_percentage()) {
///         assert!(pct >= 0.0 && pct <= 100.0);
///     }
/// }
/// ```
pub fn any_coverage_percentage() -> impl Strategy<Value = f64> {
    0.0f64..=100.0
}

/// Generate random ASTs with varying complexity.
///
/// Creates ASTs with 0-20 if-statements, resulting in
/// cyclomatic complexity of 1-21.
///
/// Note: This generates simple synthetic ASTs. For more realistic
/// ASTs, consider using corpus-based fuzzing.
///
/// # Example
///
/// ```rust,ignore
/// proptest! {
///     #[test]
///     fn ast_is_parseable(ast in any_ast()) {
///         // AST should always be valid
///         assert!(!ast.items.is_empty());
///     }
/// }
/// ```
pub fn any_ast() -> impl Strategy<Value = syn::File> {
    (0u32..=20).prop_map(crate::testkit::helpers::create_test_ast)
}

/// Generate random nesting depths (1-10).
///
/// # Example
///
/// ```rust,ignore
/// proptest! {
///     #[test]
///     fn nesting_is_reasonable(depth in any_nesting_depth()) {
///         assert!(depth >= 1 && depth <= 10);
///     }
/// }
/// ```
pub fn any_nesting_depth() -> impl Strategy<Value = u32> {
    1u32..=10
}

/// Generate random line counts (1-1000).
///
/// # Example
///
/// ```rust,ignore
/// proptest! {
///     #[test]
///     fn loc_is_positive(lines in any_lines_of_code()) {
///         assert!(lines >= 1);
///     }
/// }
/// ```
pub fn any_lines_of_code() -> impl Strategy<Value = usize> {
    1usize..=1000
}

/// Generate random file paths.
///
/// Creates paths like "src/module_123.rs" for testing file-based operations.
///
/// # Example
///
/// ```rust,ignore
/// proptest! {
///     #[test]
///     fn path_is_valid(path in any_file_path()) {
///         assert!(path.extension().map(|e| e == "rs").unwrap_or(false));
///     }
/// }
/// ```
pub fn any_file_path() -> impl Strategy<Value = std::path::PathBuf> {
    (0u32..=100).prop_map(|n| std::path::PathBuf::from(format!("src/module_{}.rs", n)))
}

/// Generate random function names.
///
/// Creates valid Rust function names like "func_42".
///
/// # Example
///
/// ```rust,ignore
/// proptest! {
///     #[test]
///     fn name_is_valid(name in any_function_name()) {
///         assert!(!name.is_empty());
///     }
/// }
/// ```
pub fn any_function_name() -> impl Strategy<Value = String> {
    (0u32..=100).prop_map(|n| format!("func_{}", n))
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn complexity_in_range(complexity in any_complexity()) {
            prop_assert!(complexity >= 1);
            prop_assert!(complexity <= 100);
        }

        #[test]
        fn thresholds_are_valid(thresholds in any_thresholds()) {
            if let Some(c) = thresholds.complexity {
                prop_assert!(c >= 1);
                prop_assert!(c <= 100);
            }
            if let Some(mfl) = thresholds.max_file_length {
                prop_assert!(mfl >= 100);
                prop_assert!(mfl <= 2000);
            }
            if let Some(mfnl) = thresholds.max_function_length {
                prop_assert!(mfnl >= 10);
                prop_assert!(mfnl <= 500);
            }
            if let Some(mds) = thresholds.minimum_debt_score {
                prop_assert!(mds >= 0.0);
                prop_assert!(mds <= 10.0);
            }
        }

        #[test]
        fn coverage_invariant_holds(coverage in any_coverage()) {
            // hit_lines should never exceed total_lines
            prop_assert!(coverage.hit_lines <= coverage.total_lines);
        }

        #[test]
        fn coverage_percentage_bounded(pct in any_coverage_percentage()) {
            prop_assert!(pct >= 0.0);
            prop_assert!(pct <= 100.0);
        }

        #[test]
        fn ast_has_items(ast in any_ast()) {
            // All generated ASTs should have at least one item (the function)
            prop_assert!(!ast.items.is_empty());
        }

        #[test]
        fn nesting_depth_in_range(depth in any_nesting_depth()) {
            prop_assert!(depth >= 1);
            prop_assert!(depth <= 10);
        }

        #[test]
        fn lines_of_code_positive(lines in any_lines_of_code()) {
            prop_assert!(lines >= 1);
            prop_assert!(lines <= 1000);
        }

        #[test]
        fn file_path_is_rust(path in any_file_path()) {
            prop_assert!(path.extension().map(|e| e == "rs").unwrap_or(false));
        }

        #[test]
        fn function_name_not_empty(name in any_function_name()) {
            prop_assert!(!name.is_empty());
        }
    }

    // Example property tests that verify invariants
    proptest! {
        #[test]
        fn config_has_thresholds(config in any_config()) {
            // Generated configs should always have thresholds
            prop_assert!(config.thresholds.is_some());
        }
    }
}
